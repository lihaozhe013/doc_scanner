use std::{
    collections::{HashMap, VecDeque},
    fs,
    path::{Path, PathBuf},
};

use eframe::{App, CreationContext, egui};
use scanner_core::{ImageId, ProcessingResult, QueueStatus};
use tracing::{info, warn};

use crate::{
    canvas::CanvasView,
    persistence,
    state::{PreviewInfo, QueueItem},
    tasks::{CancellationToken, TaskId, TaskRunner, WorkerEvent},
    ui::{inspector, queue},
};

pub struct ScannerApp {
    worker: TaskRunner,
    items: Vec<QueueItem>,
    selected: Option<usize>,
    canvas: CanvasView,
    messages: VecDeque<String>,
    export_tasks: HashMap<TaskId, (ImageId, CancellationToken)>,
    export_total: usize,
    export_completed: usize,
    export_failed: usize,
    export_progress: HashMap<TaskId, f32>,
    session_path: Option<PathBuf>,
}

impl ScannerApp {
    pub fn new(
        _creation_context: &CreationContext<'_>,
    ) -> std::io::Result<Self> {
        Ok(Self {
            worker: TaskRunner::new()?,
            items: Vec::new(),
            selected: None,
            canvas: CanvasView::new(),
            messages: VecDeque::new(),
            export_tasks: HashMap::new(),
            export_total: 0,
            export_completed: 0,
            export_failed: 0,
            export_progress: HashMap::new(),
            session_path: None,
        })
    }

    fn poll_worker(&mut self, ctx: &egui::Context) {
        for event in self.worker.drain_events() {
            match event {
                WorkerEvent::Loaded { item_id, result } => match result {
                    Ok(mut loaded) => {
                        if let Some(index) = self.index_for(item_id) {
                            loaded.source.id = item_id;
                            let source_name =
                                loaded.source.display_name.clone();
                            let color_image =
                                color_image_from_dynamic(&loaded.image);
                            let texture = ctx.load_texture(
                                format!("source-{item_id:?}"),
                                color_image,
                                egui::TextureOptions::LINEAR,
                            );
                            let item = &mut self.items[index];
                            item.set_loaded(loaded.source);
                            item.source_texture = Some(texture);
                            item.error = None;
                            info!("[image_import] loaded {source_name}");
                            self.request_preview(item_id);
                        }
                    }
                    Err(error) => {
                        if let Some(index) = self.index_for(item_id) {
                            let item_name =
                                self.items[index].display_name.clone();
                            self.items[index].status = QueueStatus::Failed;
                            self.items[index].error = Some(error.clone());
                            warn!(
                                "[image_import] failed to load {item_name}: {error}"
                            );
                            self.push_message(format!("{item_name}: {error}"));
                        }
                    }
                },
                WorkerEvent::PreviewReady {
                    item_id,
                    revision,
                    result,
                } => {
                    self.handle_preview(ctx, item_id, revision, result);
                }
                WorkerEvent::PreviewCancelled { item_id, revision } => {
                    if let Some(index) = self.index_for(item_id) {
                        let item = &mut self.items[index];
                        if item.edit.revision == revision {
                            item.status = QueueStatus::Ready;
                        }
                    }
                }
                WorkerEvent::ExportProgress {
                    task_id,
                    item_id,
                    progress,
                } => {
                    self.export_progress.insert(task_id, progress);
                    if let Some(index) = self.index_for(item_id) {
                        self.items[index].status = QueueStatus::Processing;
                    }
                    if !self.export_tasks.contains_key(&task_id) {
                        warn!("[export] received progress for an unknown task");
                    }
                }
                WorkerEvent::ExportFinished {
                    task_id,
                    item_id,
                    result,
                } => {
                    self.export_tasks.remove(&task_id);
                    self.export_progress.remove(&task_id);
                    match result {
                        Ok(export) => {
                            if let Some(index) = self.index_for(item_id) {
                                self.items[index].status =
                                    QueueStatus::Completed;
                                self.items[index].error = None;
                            }
                            info!(
                                "[export] completed {}",
                                file_name_for_message(&export.path)
                            );
                            self.push_message(format!(
                                "Exported {}",
                                file_name_for_message(&export.path)
                            ));
                            self.export_completed =
                                self.export_completed.saturating_add(1);
                        }
                        Err(error) => {
                            if let Some(index) = self.index_for(item_id) {
                                self.items[index].status = QueueStatus::Failed;
                                self.items[index].error = Some(error.clone());
                            }
                            warn!(
                                "[export] failed for item {item_id:?}: {error}"
                            );
                            self.push_message(format!(
                                "Export failed: {error}"
                            ));
                            self.export_failed =
                                self.export_failed.saturating_add(1);
                        }
                    }
                }
                WorkerEvent::ExportCancelled { task_id, item_id } => {
                    self.export_tasks.remove(&task_id);
                    self.export_progress.remove(&task_id);
                    if let Some(index) = self.index_for(item_id) {
                        self.items[index].status = QueueStatus::Cancelled;
                    }
                    self.push_message("Export cancelled".to_owned());
                }
            }
        }
        if !self.export_tasks.is_empty() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn handle_preview(
        &mut self,
        ctx: &egui::Context,
        item_id: ImageId,
        revision: u64,
        result: std::result::Result<ProcessingResult, String>,
    ) {
        let Some(index) = self.index_for(item_id) else {
            return;
        };
        if self.items[index].edit.revision != revision {
            info!("[processing] discarded stale preview for {item_id:?}");
            return;
        }
        match result {
            Ok(processed) => {
                let dimensions = processed.output_dimensions;
                let texture = ctx.load_texture(
                    format!("preview-{item_id:?}-{revision}"),
                    color_image_from_dynamic(&processed.image),
                    egui::TextureOptions::LINEAR,
                );
                let item = &mut self.items[index];
                item.preview_texture = Some(texture);
                item.preview = Some(PreviewInfo {
                    revision,
                    width: dimensions.width,
                    height: dimensions.height,
                });
                item.status = QueueStatus::Ready;
                item.error = None;
                info!(
                    "[processing] preview ready for {item_id:?}, revision {revision}"
                );
            }
            Err(error) => {
                let item = &mut self.items[index];
                item.status = QueueStatus::Failed;
                item.error = Some(error.clone());
                warn!(
                    "[processing] preview failed for {}: {error}",
                    item.display_name
                );
                self.push_message(format!("Preview failed: {error}"));
            }
        }
    }

    fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        let (
            open,
            next,
            previous,
            reset_view,
            undo,
            redo,
            save,
            export,
            cancel,
        ) = ctx.input(|input| {
            let command = input.modifiers.command;
            (
                input.key_pressed(egui::Key::O),
                input.key_pressed(egui::Key::N),
                input.key_pressed(egui::Key::P),
                input.key_pressed(egui::Key::R),
                command
                    && input.key_pressed(egui::Key::Z)
                    && !input.modifiers.shift,
                command
                    && input.key_pressed(egui::Key::Z)
                    && input.modifiers.shift,
                command && input.key_pressed(egui::Key::S),
                command && input.key_pressed(egui::Key::E),
                input.key_pressed(egui::Key::Escape),
            )
        });
        if open {
            self.open_files();
        }
        if next || ctx.input(|input| input.key_pressed(egui::Key::Enter)) {
            self.select_relative(1);
        }
        if previous {
            self.select_relative(-1);
        }
        if reset_view {
            self.canvas.reset_view();
        }
        if undo {
            self.edit_selected(EditAction::Undo);
        }
        if redo {
            self.edit_selected(EditAction::Redo);
        }
        if save {
            self.save_session();
        }
        if export {
            self.export_selected();
        }
        if cancel {
            self.cancel_exports();
        }
    }

    fn show_toolbar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Document Scanner");
                ui.separator();
                if ui
                    .button("Open files")
                    .on_hover_text("Open files (O)")
                    .clicked()
                {
                    self.open_files();
                }
                if ui.button("Open folder").clicked() {
                    self.open_folder();
                }
                if ui
                    .add_enabled(
                        !self.items.is_empty(),
                        egui::Button::new("Previous"),
                    )
                    .on_hover_text("Previous page (P)")
                    .clicked()
                {
                    self.select_relative(-1);
                }
                if ui
                    .add_enabled(
                        !self.items.is_empty(),
                        egui::Button::new("Next"),
                    )
                    .on_hover_text("Next page (N)")
                    .clicked()
                {
                    self.select_relative(1);
                }
                if ui
                    .add_enabled(
                        !self.items.is_empty(),
                        egui::Button::new("Accept & next"),
                    )
                    .on_hover_text(
                        "Accept current page and move to next (Enter)",
                    )
                    .clicked()
                {
                    self.select_relative(1);
                }
                if ui
                    .button("Save session")
                    .on_hover_text("Save session (Ctrl/Cmd+S)")
                    .clicked()
                {
                    self.save_session();
                }
                if ui.button("Load session").clicked() {
                    self.load_session();
                }
                ui.separator();
                if ui
                    .add_enabled(
                        self.selected.is_some(),
                        egui::Button::new("Export selected"),
                    )
                    .on_hover_text("Export selected page (Ctrl/Cmd+E)")
                    .clicked()
                {
                    self.export_selected();
                }
                if ui
                    .add_enabled(
                        !self.items.is_empty(),
                        egui::Button::new("Export all"),
                    )
                    .clicked()
                {
                    self.export_all();
                }
                if ui
                    .add_enabled(
                        !self.export_tasks.is_empty(),
                        egui::Button::new("Cancel"),
                    )
                    .clicked()
                {
                    self.cancel_exports();
                }
            });
        });
    }

    fn show_canvas(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Canvas");
                if ui.button("Fit").on_hover_text("Reset view (R)").clicked() {
                    self.canvas.reset_view();
                }
                let mut zoom = self.canvas.zoom();
                if ui
                    .add(egui::Slider::new(&mut zoom, 0.25..=4.0).text("Zoom"))
                    .changed()
                {
                    self.canvas.set_zoom(zoom);
                }
                ui.label(format!("{}%", (self.canvas.zoom() * 100.0).round()));
                if let Some(index) = self.selected
                    && let Some(preview) = &self.items[index].preview
                {
                    ui.separator();
                    ui.small(format!(
                        "Output preview: {} × {} · revision {}",
                        preview.width, preview.height, preview.revision
                    ));
                }
            });
            ui.add_space(8.0);

            if let Some(index) = self.selected {
                let mut preview_request = None;
                ui.columns(2, |columns| {
                    columns[0].heading("Source");
                    let (changed, item_id) = {
                        let before = self.items[index].edit.clone();
                        let item = &mut self.items[index];
                        let source_texture = item.source_texture.as_ref();
                        let image_size = item
                            .metadata
                            .as_ref()
                            .map(|metadata| [metadata.width, metadata.height]);
                        let response = self.canvas.show(
                            &mut columns[0],
                            source_texture,
                            image_size,
                            &mut item.edit.quadrilateral,
                        );
                        let item_id = item.id;
                        let changed =
                            response.quad_changed && item.record_edit(before);
                        (changed, item_id)
                    };
                    if changed {
                        preview_request = Some(item_id);
                    }

                    columns[1].heading("Processed preview");
                    if let Some(texture) =
                        self.items[index].preview_texture.as_ref()
                        && let Some(preview) =
                            self.items[index].preview.as_ref()
                    {
                        let available_width =
                            columns[1].available_width().max(160.0);
                        let aspect =
                            preview.width as f32 / preview.height.max(1) as f32;
                        let size = egui::vec2(
                            available_width,
                            (available_width / aspect).min(280.0),
                        );
                        columns[1].add(
                            egui::Image::new((texture.id(), size))
                                .maintain_aspect_ratio(true),
                        );
                        columns[1].small(format!(
                            "{} × {} · revision {}",
                            preview.width, preview.height, preview.revision
                        ));
                    } else {
                        columns[1]
                            .small("The processed preview will appear here.");
                    }
                });
                if let Some(item_id) = preview_request {
                    self.request_preview(item_id);
                }
            } else {
                ui.centered_and_justified(|ui| {
                    ui.heading("Import pages to begin");
                });
            }
        });
    }

    fn show_inspector(&mut self, ui: &mut egui::Ui) {
        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(280.0)
            .show(ui, |ui| {
                if let Some(index) = self.selected {
                    let (action, edit_changed, item_id) = {
                        let item = &mut self.items[index];
                        let previous_revision = item.edit.revision;
                        let action = inspector::show(ui, item);
                        (
                            action,
                            item.edit.revision != previous_revision,
                            item.id,
                        )
                    };
                    if edit_changed {
                        self.request_preview(item_id);
                    }
                    if action == inspector::InspectorAction::ApplyToAll {
                        self.apply_to_all(index);
                    }
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(self.items[index].can_undo(), egui::Button::new("Undo"))
                            .clicked()
                        {
                            self.edit_selected(EditAction::Undo);
                        }
                        if ui
                            .add_enabled(self.items[index].can_redo(), egui::Button::new("Redo"))
                            .clicked()
                        {
                            self.edit_selected(EditAction::Redo);
                        }
                    });
                    if ui.button("Reset quad").clicked()
                        && self.items[index].reset_quadrilateral()
                    {
                        self.request_preview(self.items[index].id);
                    }
                } else {
                    ui.heading("Inspector");
                    ui.add_space(8.0);
                    ui.label("Select a page to edit its perspective and enhancement settings.");
                }
            });
    }

    fn show_status(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("status").show(ui, |ui| {
            ui.horizontal(|ui| {
                if self.export_total > 0 {
                    let active_progress =
                        self.export_progress.values().sum::<f32>();
                    let fraction = (self.export_completed as f32
                        + active_progress)
                        / self.export_total as f32;
                    ui.add(egui::ProgressBar::new(fraction.min(1.0)).text(
                        format!(
                            "{}/{}",
                            self.export_completed, self.export_total
                        ),
                    ));
                    if self.export_failed > 0 {
                        ui.colored_label(
                            egui::Color32::from_rgb(240, 120, 100),
                            format!("{} failed", self.export_failed),
                        );
                    }
                }
                if let Some(message) = self.messages.back() {
                    ui.label(message);
                } else {
                    ui.small("Ready");
                }
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        if let Some(path) = &self.session_path {
                            ui.small(format!(
                                "Session: {}",
                                file_name_for_message(path)
                            ));
                        }
                    },
                );
            });
        });
    }

    fn open_files(&mut self) {
        let Some(paths) = rfd::FileDialog::new()
            .add_filter(
                "Raster images",
                &["jpg", "jpeg", "png", "bmp", "tif", "tiff"],
            )
            .pick_files()
        else {
            return;
        };
        self.add_paths(paths);
    }

    fn open_folder(&mut self) {
        let Some(folder) = rfd::FileDialog::new().pick_folder() else {
            return;
        };
        let mut paths = Vec::new();
        match fs::read_dir(&folder) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        paths.push(path);
                    }
                }
            }
            Err(error) => {
                self.push_message(format!("Could not read folder: {error}"));
                return;
            }
        }
        paths.sort_by(|left, right| {
            file_name_for_sort(left)
                .cmp(&file_name_for_sort(right))
                .then_with(|| {
                    scanner_core::display_name_for_path(left)
                        .cmp(&scanner_core::display_name_for_path(right))
                })
        });
        self.add_paths(paths);
    }

    fn add_paths(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            if self.items.iter().any(|item| item.path == path) {
                continue;
            }
            let item = QueueItem::new(path);
            let item_id = item.id;
            let item_path = item.path.clone();
            let item_name = item.display_name.clone();
            self.items.push(item);
            if self.selected.is_none() {
                self.selected = Some(self.items.len() - 1);
            }
            self.worker.load(item_id, item_path);
            info!("[image_import] queued {item_name}");
        }
        if !self.items.is_empty() {
            self.push_message(format!("{} page(s) in queue", self.items.len()));
        }
    }

    fn request_preview(&mut self, item_id: ImageId) {
        let Some(index) = self.index_for(item_id) else {
            return;
        };
        let (path, edit, revision) = {
            let item = &self.items[index];
            if item.metadata.is_none() {
                return;
            }
            (item.path.clone(), item.edit.clone(), item.edit.revision)
        };
        let cancellation = self.worker.preview(item_id, revision, path, edit);
        let item = &mut self.items[index];
        if let Some(token) = item.preview_cancellation.take() {
            token.cancel();
        }
        item.preview_cancellation = Some(cancellation);
        item.status = QueueStatus::Previewing;
    }

    fn export_selected(&mut self) {
        let Some(folder) = self.choose_export_folder() else {
            return;
        };
        let Some(index) = self.selected else {
            return;
        };
        self.begin_export_batch();
        self.queue_export(index, &folder);
    }

    fn export_all(&mut self) {
        let Some(folder) = self.choose_export_folder() else {
            return;
        };
        let indices = self
            .items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| item.metadata.as_ref().map(|_| index))
            .collect::<Vec<_>>();
        if indices.is_empty() {
            self.push_message("No loaded pages are ready to export".to_owned());
            return;
        }
        self.begin_export_batch();
        for index in indices {
            self.queue_export(index, &folder);
        }
    }

    fn choose_export_folder(&mut self) -> Option<PathBuf> {
        let folder = rfd::FileDialog::new().pick_folder();
        if folder.is_none() {
            self.push_message("Export cancelled".to_owned());
        }
        folder
    }

    fn queue_export(&mut self, index: usize, destination: &Path) {
        let Some(item) = self.items.get(index) else {
            return;
        };
        if item.metadata.is_none() {
            self.push_message(format!(
                "{} is not loaded yet",
                item.display_name
            ));
            return;
        }
        let (item_id, path, edit) =
            (item.id, item.path.clone(), item.edit.clone());
        let (task_id, cancellation) =
            self.worker
                .export(item_id, path, destination.to_owned(), edit);
        self.export_tasks.insert(task_id, (item_id, cancellation));
        self.export_total = self.export_total.saturating_add(1);
        if let Some(item) = self.items.get_mut(index) {
            item.status = QueueStatus::Queued;
            item.error = None;
        }
        info!("[export] queued item {item_id:?}");
    }

    fn begin_export_batch(&mut self) {
        if self.export_tasks.is_empty() {
            self.export_total = 0;
            self.export_completed = 0;
            self.export_failed = 0;
            self.export_progress.clear();
        }
    }

    fn cancel_exports(&mut self) {
        for (_, token) in self.export_tasks.values() {
            token.cancel();
        }
        self.push_message("Cancellation requested".to_owned());
    }

    fn save_session(&mut self) {
        let default_name = self
            .session_path
            .as_deref()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("document-scanner.scanner-session.json");
        let path = rfd::FileDialog::new()
            .set_file_name(default_name)
            .add_filter("Scanner session", &["json"])
            .save_file();
        let Some(path) = path else {
            return;
        };
        match persistence::save(&path, &self.items) {
            Ok(()) => {
                self.session_path = Some(path.clone());
                self.push_message(format!(
                    "Saved session {}",
                    file_name_for_message(&path)
                ));
            }
            Err(error) => {
                self.push_message(format!("Could not save session: {error}"))
            }
        }
    }

    fn load_session(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Scanner session", &["json"])
            .pick_file()
        else {
            return;
        };
        match persistence::load(&path) {
            Ok(session_items) => {
                self.items = session_items
                    .into_iter()
                    .map(QueueItem::from_session)
                    .collect();
                self.selected = (!self.items.is_empty()).then_some(0);
                self.session_path = Some(path.clone());
                for item in &self.items {
                    self.worker.load(item.id, item.path.clone());
                }
                self.push_message(format!(
                    "Loaded session {}",
                    file_name_for_message(&path)
                ));
            }
            Err(error) => {
                self.push_message(format!("Could not load session: {error}"))
            }
        }
    }

    fn apply_to_all(&mut self, source_index: usize) {
        let Some(enhancement) = self
            .items
            .get(source_index)
            .map(|item| item.edit.enhancement.clone())
        else {
            return;
        };
        let mut changed_ids = Vec::new();
        for (index, item) in self.items.iter_mut().enumerate() {
            if index == source_index || item.metadata.is_none() {
                continue;
            }
            let before = item.edit.clone();
            item.edit.enhancement = enhancement.clone();
            if item.record_edit(before) {
                changed_ids.push(item.id);
            }
        }
        for item_id in changed_ids {
            self.request_preview(item_id);
        }
        self.push_message(
            "Applied enhancement settings to loaded pages".to_owned(),
        );
    }

    fn edit_selected(&mut self, action: EditAction) {
        let Some(index) = self.selected else {
            return;
        };
        let changed = match action {
            EditAction::Undo => self.items[index].undo(),
            EditAction::Redo => self.items[index].redo(),
        };
        if changed {
            self.request_preview(self.items[index].id);
        }
    }

    fn select_relative(&mut self, offset: isize) {
        if self.items.is_empty() {
            return;
        }
        let current = self.selected.unwrap_or(0) as isize;
        let next =
            (current + offset).rem_euclid(self.items.len() as isize) as usize;
        self.selected = Some(next);
    }

    fn index_for(&self, item_id: ImageId) -> Option<usize> {
        self.items.iter().position(|item| item.id == item_id)
    }

    fn push_message(&mut self, message: String) {
        while self.messages.len() >= 4 {
            self.messages.pop_front();
        }
        self.messages.push_back(message);
    }
}

#[derive(Debug, Clone, Copy)]
enum EditAction {
    Undo,
    Redo,
}

impl App for ScannerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_worker(&ctx);
        self.handle_shortcuts(&ctx);
        self.show_toolbar(ui);
        egui::Panel::left("queue")
            .resizable(true)
            .default_size(240.0)
            .show(ui, |ui| queue::show(ui, &self.items, &mut self.selected));
        self.show_inspector(ui);
        self.show_status(ui);
        self.show_canvas(ui);
    }
}

fn color_image_from_dynamic(image: &image::DynamicImage) -> egui::ColorImage {
    let rgba = image.to_rgba8();
    egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    )
}

fn file_name_for_message(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("output")
        .to_owned()
}

fn file_name_for_sort(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_else(|| path.to_string_lossy().to_ascii_lowercase())
}

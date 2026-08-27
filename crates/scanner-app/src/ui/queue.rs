use eframe::egui;
use scanner_core::QueueStatus;

use crate::state::QueueItem;

pub fn show(
    ui: &mut egui::Ui,
    items: &[QueueItem],
    selected: &mut Option<usize>,
) {
    ui.heading("Pages");
    ui.add_space(8.0);
    if items.is_empty() {
        ui.label("No pages imported yet.");
        ui.add_space(4.0);
        ui.small("Use Open files or Open folder to begin.");
        return;
    }

    egui::ScrollArea::vertical().show(ui, |ui| {
        for (index, item) in items.iter().enumerate() {
            let is_selected = *selected == Some(index);
            ui.horizontal(|ui| {
                if let Some(texture) = &item.source_texture {
                    ui.add(
                        egui::Image::new((
                            texture.id(),
                            egui::vec2(48.0, 48.0),
                        ))
                        .fit_to_exact_size(egui::vec2(48.0, 48.0)),
                    );
                } else {
                    ui.allocate_space(egui::vec2(48.0, 48.0));
                }
                ui.vertical(|ui| {
                    let response =
                        ui.selectable_label(is_selected, &item.display_name);
                    if response.clicked() {
                        *selected = Some(index);
                    }
                    ui.small(status_label(item.status));
                    if let Some(metadata) = &item.metadata {
                        ui.small(format!(
                            "{} × {}",
                            metadata.width, metadata.height
                        ));
                    }
                });
            });
            if is_selected {
                ui.separator();
            }
            if let Some(error) = &item.error {
                ui.add_space(2.0);
                ui.colored_label(egui::Color32::from_rgb(240, 120, 100), error);
            }
            ui.add_space(6.0);
        }
    });
}

fn status_label(status: QueueStatus) -> &'static str {
    match status {
        QueueStatus::NotStarted => "Not started",
        QueueStatus::Loading => "Loading",
        QueueStatus::Ready => "Ready",
        QueueStatus::Previewing => "Previewing",
        QueueStatus::Queued => "Queued",
        QueueStatus::Processing => "Processing",
        QueueStatus::Completed => "Completed",
        QueueStatus::Skipped => "Skipped",
        QueueStatus::Failed => "Failed",
        QueueStatus::Cancelled => "Cancelled",
    }
}

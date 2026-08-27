use eframe::egui;
use scanner_core::QueueStatus;

use crate::{i18n::I18n, state::QueueItem};

pub fn show(
    ui: &mut egui::Ui,
    items: &[QueueItem],
    selected: &mut Option<usize>,
    i18n: &I18n,
) {
    ui.heading(i18n.tr("queue.title"));
    ui.add_space(8.0);
    if items.is_empty() {
        ui.label(i18n.tr("queue.empty"));
        ui.add_space(4.0);
        ui.small(i18n.tr("queue.empty_hint"));
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
                    ui.small(status_label(i18n, item.status));
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

fn status_label(i18n: &I18n, status: QueueStatus) -> String {
    i18n.tr(match status {
        QueueStatus::NotStarted => "status.not_started",
        QueueStatus::Loading => "status.loading",
        QueueStatus::Ready => "status.ready",
        QueueStatus::Previewing => "status.previewing",
        QueueStatus::Queued => "status.queued",
        QueueStatus::Processing => "status.processing",
        QueueStatus::Completed => "status.completed",
        QueueStatus::Skipped => "status.skipped",
        QueueStatus::Failed => "status.failed",
        QueueStatus::Cancelled => "status.cancelled",
    })
}

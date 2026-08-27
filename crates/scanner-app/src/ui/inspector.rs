use eframe::egui;
use scanner_core::{EnhancementPreset, OutputFormat};

use crate::state::QueueItem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorAction {
    None,
    ApplyToAll,
}

pub fn show(ui: &mut egui::Ui, item: &mut QueueItem) -> InspectorAction {
    ui.heading("Inspector");
    ui.add_space(8.0);
    ui.label(&item.display_name);
    if let Some(metadata) = &item.metadata {
        ui.small(format!(
            "{} × {} · {} channels",
            metadata.width, metadata.height, metadata.channels
        ));
    }
    ui.separator();

    let before = item.edit.clone();
    ui.label("Enhancement");
    egui::ComboBox::from_id_salt("preset")
        .selected_text(item.edit.enhancement.preset.label())
        .show_ui(ui, |ui| {
            for preset in EnhancementPreset::ALL {
                ui.selectable_value(
                    &mut item.edit.enhancement.preset,
                    preset,
                    preset.label(),
                );
            }
        });
    show_preset_controls(ui, item);

    ui.separator();
    ui.label("Output");
    egui::ComboBox::from_id_salt("output-format")
        .selected_text(item.edit.output.format.label())
        .show_ui(ui, |ui| {
            for format in OutputFormat::ALL {
                ui.selectable_value(
                    &mut item.edit.output.format,
                    format,
                    format.label(),
                );
            }
        });
    ui.add_enabled_ui(item.edit.output.format == OutputFormat::Jpeg, |ui| {
        ui.add(
            egui::Slider::new(&mut item.edit.output.jpeg_quality, 1..=100)
                .text("JPEG quality"),
        );
    });

    let changed = item.record_edit(before);
    if changed {
        ui.ctx().request_repaint();
    }

    ui.separator();
    if ui.button("Apply current preset to all").clicked() {
        return InspectorAction::ApplyToAll;
    }
    InspectorAction::None
}

fn show_preset_controls(ui: &mut egui::Ui, item: &mut QueueItem) {
    match item.edit.enhancement.preset {
        EnhancementPreset::Original => {
            ui.small("No enhancement");
        }
        EnhancementPreset::AdaptiveBlackAndWhite => {
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.adaptive_block_size,
                    3..=101,
                )
                .text("Block size"),
            );
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.adaptive_threshold_offset,
                    -32.0..=32.0,
                )
                .text("Offset"),
            );
        }
        EnhancementPreset::EnhancedColor => {
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.color_brightness,
                    -100.0..=100.0,
                )
                .text("Brightness"),
            );
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.color_contrast,
                    0.0..=2.5,
                )
                .text("Contrast"),
            );
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.denoise_strength,
                    0.0..=1.0,
                )
                .text("Denoise"),
            );
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.sharpening_strength,
                    0.0..=2.0,
                )
                .text("Sharpen"),
            );
        }
        EnhancementPreset::MagicColor => {
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.magic_local_contrast,
                    0.0..=2.5,
                )
                .text("Local contrast"),
            );
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.magic_saturation,
                    0.0..=2.5,
                )
                .text("Saturation"),
            );
        }
    }
}

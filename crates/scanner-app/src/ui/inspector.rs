use eframe::egui;
use scanner_core::{EnhancementPreset, OutputFormat};

use crate::{i18n::I18n, state::QueueItem};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorAction {
    None,
    ApplyToAll,
}

pub fn show(
    ui: &mut egui::Ui,
    item: &mut QueueItem,
    i18n: &I18n,
) -> InspectorAction {
    ui.heading(i18n.tr("inspector.title"));
    ui.add_space(8.0);
    ui.label(&item.display_name);
    if let Some(metadata) = &item.metadata {
        let text = i18n.text(
            "inspector.channels",
            &[
                ("width", metadata.width.to_string()),
                ("height", metadata.height.to_string()),
                ("channels", metadata.channels.to_string()),
            ],
        );
        ui.small(text);
    }
    ui.separator();

    let before = item.edit.clone();
    ui.label(i18n.tr("inspector.enhancement"));
    egui::ComboBox::from_id_salt("preset")
        .selected_text(i18n.preset_label(item.edit.enhancement.preset))
        .show_ui(ui, |ui| {
            for preset in EnhancementPreset::ALL {
                ui.selectable_value(
                    &mut item.edit.enhancement.preset,
                    preset,
                    i18n.preset_label(preset),
                );
            }
        });
    show_preset_controls(ui, item, i18n);

    ui.separator();
    ui.label(i18n.tr("inspector.output"));
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
                .text(i18n.tr("inspector.jpeg_quality")),
        );
    });

    let changed = item.record_edit(before);
    if changed {
        ui.ctx().request_repaint();
    }

    ui.separator();
    if ui.button(i18n.tr("inspector.apply_all")).clicked() {
        return InspectorAction::ApplyToAll;
    }
    InspectorAction::None
}

fn show_preset_controls(ui: &mut egui::Ui, item: &mut QueueItem, i18n: &I18n) {
    match item.edit.enhancement.preset {
        EnhancementPreset::Original => {
            ui.small(i18n.tr("preset.original_hint"));
        }
        EnhancementPreset::AdaptiveBlackAndWhite => {
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.adaptive_block_size,
                    3..=101,
                )
                .text(i18n.tr("preset.block_size")),
            );
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.adaptive_threshold_offset,
                    -32.0..=32.0,
                )
                .text(i18n.tr("preset.offset")),
            );
        }
        EnhancementPreset::EnhancedColor => {
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.color_brightness,
                    -100.0..=100.0,
                )
                .text(i18n.tr("preset.brightness")),
            );
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.color_contrast,
                    0.0..=2.5,
                )
                .text(i18n.tr("preset.contrast")),
            );
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.denoise_strength,
                    0.0..=1.0,
                )
                .text(i18n.tr("preset.denoise")),
            );
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.sharpening_strength,
                    0.0..=2.0,
                )
                .text(i18n.tr("preset.sharpen")),
            );
        }
        EnhancementPreset::MagicColor => {
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.magic_local_contrast,
                    0.0..=2.5,
                )
                .text(i18n.tr("preset.local_contrast")),
            );
            ui.add(
                egui::Slider::new(
                    &mut item.edit.enhancement.magic_saturation,
                    0.0..=2.5,
                )
                .text(i18n.tr("preset.saturation")),
            );
        }
    }
}

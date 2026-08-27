use eframe::egui;
use tracing::{info, warn};

use crate::{app::ScannerApp, i18n::LanguagePreference, preferences};

/// Renders the top action toolbar and the language picker.
pub fn show(ui: &mut egui::Ui, app: &mut ScannerApp) {
    egui::Panel::top("toolbar").show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.heading(app.i18n.tr("app.title"));
            ui.separator();
            if ui
                .button(app.i18n.tr("toolbar.open_files"))
                .on_hover_text(app.i18n.tr("toolbar.open_files_hint"))
                .clicked()
            {
                app.open_files();
            }
            if ui.button(app.i18n.tr("toolbar.open_folder")).clicked() {
                app.open_folder();
            }
            if ui
                .add_enabled(
                    !app.items.is_empty(),
                    egui::Button::new(app.i18n.tr("toolbar.previous")),
                )
                .on_hover_text(app.i18n.tr("toolbar.previous_hint"))
                .clicked()
            {
                app.select_relative(-1);
            }
            if ui
                .add_enabled(
                    !app.items.is_empty(),
                    egui::Button::new(app.i18n.tr("toolbar.next")),
                )
                .on_hover_text(app.i18n.tr("toolbar.next_hint"))
                .clicked()
            {
                app.select_relative(1);
            }
            if ui
                .add_enabled(
                    !app.items.is_empty(),
                    egui::Button::new(app.i18n.tr("toolbar.accept_next")),
                )
                .on_hover_text(app.i18n.tr("toolbar.accept_next_hint"))
                .clicked()
            {
                app.select_relative(1);
            }
            if ui
                .button(app.i18n.tr("toolbar.save_session"))
                .on_hover_text(app.i18n.tr("toolbar.save_session_hint"))
                .clicked()
            {
                app.save_session();
            }
            if ui.button(app.i18n.tr("toolbar.load_session")).clicked() {
                app.load_session();
            }
            ui.separator();
            if ui
                .add_enabled(
                    app.selected.is_some(),
                    egui::Button::new(app.i18n.tr("toolbar.export_selected")),
                )
                .on_hover_text(app.i18n.tr("toolbar.export_selected_hint"))
                .clicked()
            {
                app.export_selected();
            }
            if ui
                .add_enabled(
                    !app.items.is_empty(),
                    egui::Button::new(app.i18n.tr("toolbar.export_all")),
                )
                .clicked()
            {
                app.export_all();
            }
            if ui
                .add_enabled(
                    !app.export_tasks.is_empty(),
                    egui::Button::new(app.i18n.tr("toolbar.cancel")),
                )
                .clicked()
            {
                app.cancel_exports();
            }
            ui.separator();
            show_language_picker(ui, app);
        });
    });
}

fn show_language_picker(ui: &mut egui::Ui, app: &mut ScannerApp) {
    let current = app.i18n.preference();
    let mut requested = None;
    ui.label(app.i18n.tr("toolbar.language"));
    egui::ComboBox::from_id_salt("language")
        .selected_text(current.label(&app.i18n))
        .show_ui(ui, |ui| {
            for preference in LanguagePreference::ALL {
                if ui
                    .selectable_label(
                        current == preference,
                        preference.label(&app.i18n),
                    )
                    .clicked()
                {
                    requested = Some(preference);
                }
            }
        });
    let ctx = ui.ctx().clone();
    if let Some(preference) = requested.filter(|p| *p != current) {
        apply_language(app, preference, &ctx);
    }
}

fn apply_language(
    app: &mut ScannerApp,
    preference: LanguagePreference,
    ctx: &egui::Context,
) {
    app.i18n.set_preference(preference);
    info!(
        "[i18n] language set to {preference:?}, resolved to {:?}",
        app.i18n.language()
    );
    let preferences = preferences::Preferences {
        language: preference,
    };
    if let Err(error) = preferences::save(&preferences) {
        warn!("[preferences] failed to persist preferences: {error}");
        let message = app
            .i18n
            .text("messages.preferences_save_failed", &[("error", error)]);
        app.push_message(message);
    }
    ctx.send_viewport_cmd(egui::ViewportCommand::Title(
        app.i18n.tr("app.title"),
    ));
    ctx.request_repaint();
}

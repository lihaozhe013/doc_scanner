mod app;
mod canvas;
mod persistence;
mod state;
mod tasks;
mod ui;

use eframe::egui;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;

const SOURCE_HAN_SANS_SC: &str = "SourceHanSansSC-Regular";

fn main() -> eframe::Result {
    let _logging_guard = init_logging();
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1_440.0, 900.0])
            .with_min_inner_size([960.0, 640.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Document Scanner",
        native_options,
        Box::new(|creation_context| {
            configure_fonts(&creation_context.egui_ctx);
            app::ScannerApp::new(creation_context)
                .map(|app| Box::new(app) as Box<dyn eframe::App>)
                .map_err(|error| {
                    Box::new(error) as Box<dyn std::error::Error + Send + Sync>
                })
        }),
    )
}

fn configure_fonts(ctx: &egui::Context) {
    ctx.set_fonts(font_definitions());
}

fn font_definitions() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        SOURCE_HAN_SANS_SC.to_owned(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/SourceHanSansSC-Regular.otf"
        ))),
    );

    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace]
    {
        if let Some(font_names) = fonts.families.get_mut(&family) {
            font_names.insert(0, SOURCE_HAN_SANS_SC.to_owned());
        }
    }

    fonts
}

fn init_logging() -> Option<WorkerGuard> {
    let appender = tracing_appender::rolling::never(".", "debug.log");
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let subscriber = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(Level::INFO)
        .with_writer(writer)
        .finish();
    if tracing::subscriber::set_global_default(subscriber).is_ok() {
        Some(guard)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_font_is_primary_for_both_font_families() {
        let fonts = font_definitions();

        assert!(fonts.font_data.contains_key(SOURCE_HAN_SANS_SC));
        assert_eq!(
            fonts.families[&egui::FontFamily::Proportional][0],
            SOURCE_HAN_SANS_SC
        );
        assert_eq!(
            fonts.families[&egui::FontFamily::Monospace][0],
            SOURCE_HAN_SANS_SC
        );
    }

    #[test]
    fn bundled_font_can_layout_english_and_chinese() {
        let ctx = egui::Context::default();
        configure_fonts(&ctx);

        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.label("Document Scanner 文档扫描");
            ui.monospace("Document Scanner 文档扫描");
        });
        output.drop_without_applying_deltas();
    }
}

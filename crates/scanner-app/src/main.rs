mod app;
mod canvas;
mod persistence;
mod state;
mod tasks;
mod ui;

use eframe::egui;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;

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
            app::ScannerApp::new(creation_context)
                .map(|app| Box::new(app) as Box<dyn eframe::App>)
                .map_err(|error| {
                    Box::new(error) as Box<dyn std::error::Error + Send + Sync>
                })
        }),
    )
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

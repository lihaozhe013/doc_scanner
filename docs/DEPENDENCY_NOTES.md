# Dependency Notes

The direct dependency versions below were queried from the crates.io registry
and Docs.rs on 2026-08-27, then resolved into `Cargo.lock` with Cargo 1.96.
The manifest uses compatible version requirements; the lockfile records the
exact graph used by this repository.

| Crate | Selected current release | Role |
| --- | ---: | --- |
| `eframe` | 0.36.1 | Native application shell |
| `egui` | 0.36.1 | UI and canvas widgets |
| `opencv` | 0.100.1 | Optional adapter boundary |
| `image` | 0.25.10 | Portable raster decode, encode, and processing backend |
| `rfd` | 0.17.2 | Cross-platform file dialogs |
| `serde` | 1.0.229 | Serializable session and edit state |
| `serde_json` | 1.0.151 | Session file format |
| `thiserror` | 2.0.20 | Domain errors |
| `anyhow` | 1.0.104 | Application and CLI error context |
| `tracing` | 0.1.44 | Structured diagnostics |
| `tracing-appender` | 0.2.5 | File log writer |
| `tracing-subscriber` | 0.3.23 | Log subscriber configuration |
| `clap` | 4.6.6 | CLI argument parsing |
| `uuid` | 1.26.0 | Stable in-session identifiers |

`scanner-app` uses standard worker threads for background work, so Rayon is
not required by the initial implementation. The OpenCV dependency is
feature-gated because its binding generator requires native OpenCV headers,
libraries, and libclang on the build machine.

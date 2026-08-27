# Document Scanner

Document Scanner is a native, offline-first desktop application for turning
photographed or scanned raster pages into clean, standardized image files.
The first release targets JPEG/JPG, PNG, BMP, and TIFF input and keeps all
edits non-destructive until export.

## Current status

Implemented in this repository:

- A Cargo workspace with `scanner-core`, `scanner-app`, and `scanner-cli`.
- UI-independent image loading, format validation, quadrilateral validation,
  perspective correction, deterministic enhancement presets, and safe export.
- A native `eframe`/`egui` desktop shell with file/folder import, a page queue,
  draggable document corners, asynchronous previews, and selected/batch export.
- A bundled Source Han Sans SC font is the primary font for both proportional
  and monospace UI text on every supported platform.
- English and Simplified Chinese UI localization with system-locale auto
  detection, a runtime language switcher, and a persisted preference
  (see [docs/I18N.md](docs/I18N.md)).
- Versioned session metadata serialization and focused unit/integration tests.
- An optional OpenCV adapter feature boundary for environments that provide a
  compatible native OpenCV installation.

The OpenCV adapter is deliberately optional for the default build. The native
image backend keeps clean-machine builds and the core test suite independent of
system libraries while preserving the planned adapter boundary for behavior
parity work.

Not included in the first release: OCR, searchable PDF, RAW, video, camera
capture, cloud storage, accounts, or a runtime dependency on ImageMagick,
Blender, a shell command, or a local web server.

## Development

The repository pins the Rust toolchain in `rust-toolchain.toml`. Run the
following commands from the repository root:

```bash
cargo run -p scanner-app
cargo run -p scanner-cli -- --help
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

The application writes diagnostics to `debug.log` by default. The file is
ignored by Git and does not contain complete source paths.

## CLI example

```bash
cargo run -p scanner-cli -- \
  --input ./input-pages \
  --output ./exports \
  --preset adaptive-black-and-white \
  --format png
```

The CLI uses the same `scanner-core` pipeline as the GUI. Existing output files
are never overwritten by default; deterministic numbered names are generated
when a collision occurs.

## Project documents

- [Product and engineering plan](PROJECT_PLAN.md)
- [Native core architecture decision](docs/ADR-0001-native-core.md)
- [Dependency version notes](docs/DEPENDENCY_NOTES.md)
- [Manual GUI verification](docs/MANUAL_TESTING.md)
- [Internationalization design](docs/I18N.md)

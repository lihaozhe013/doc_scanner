# ADR-0001: Native Rust Core and Desktop Shell

- Status: Accepted for the initial implementation
- Date: 2026-08-27

## Context

The product is a local, canvas-centric document workflow. Image processing
must be testable without a window, preserve source files, and behave
consistently in the GUI and the automation surface. A browser frontend or a
runtime conversion process would add an unnecessary IPC or deployment
boundary.

## Decision

Use a Cargo workspace with three crates:

- `scanner-core` owns image metadata, edit state, geometry, processing, and
  export. It has no GUI dependency.
- `scanner-app` owns the `eframe`/`egui` application, queue, canvas, worker
  lifecycle, and user-facing state.
- `scanner-cli` is a thin headless caller of `scanner-core` for regression and
  deterministic batch work.

The initial default processing backend uses the Rust `image` crate. The core
keeps the backend boundary in `scanner-core::backend` and exposes an optional
`opencv-backend` feature for environments with a native OpenCV installation.
This makes the default workspace build reproducible on clean macOS, Windows,
and Linux machines while leaving OpenCV-specific types out of the application
and persisted state.

All coordinates are normalized canonical image coordinates in the inclusive
unit square. The processing boundary converts them to pixel coordinates. The
export path chooses a collision-safe destination, writes a temporary file in
the destination directory, and reopens the generated image before reporting
success.

## Consequences

The default backend is portable and easy to test, but its enhancement output
is not yet a byte-for-byte port of an older OpenCV prototype. Any intentional
algorithm parity change must be covered by fixture expectations and recorded
in a follow-up decision. Enabling the OpenCV feature requires the platform's
OpenCV headers and libraries and is therefore not part of the default CI job.

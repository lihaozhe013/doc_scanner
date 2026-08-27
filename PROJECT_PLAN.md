# Document Scanner: Product and Engineering Plan

Status: In progress — Milestones 0–1 and the initial native GUI workflow are implemented.

This document remains the implementation blueprint for the repository. The
implemented behavior is summarized in `README.md`; milestones below continue
to describe planned work that has not yet been completed.

## 1. Executive Decision

Build a native cross-platform desktop application in Rust with `eframe` and
`egui`.

- Keep image processing in a UI-independent Rust core.
- Use OpenCV through a narrow adapter for the first implementation so that the
  existing perspective and enhancement behavior can be reproduced reliably.
- Keep a command-line entry point as an optional automation and regression
  surface, but make the GUI the primary user experience.
- Do not use OpenCV HighGUI, terminal input, or a background local web server
  as the application UI.
- Never modify source images in place. Treat every edit as a non-destructive
  session state until the user explicitly exports it.

The first release should optimize for a dependable local workflow rather than
for OCR, cloud processing, or an exhaustive image-editing feature set.

## 2. Product Definition

### 2.1 Target users

The application is for people who need to turn photographed or scanned pages
into clean, standardized image files without operating a full image editor.
The initial user should be able to process a folder of pages locally and
visually verify every result.

### 2.2 Core problem

The current prototype proves the image operations but makes the user operate a
window and a terminal at the same time. The product must turn that proof of
concept into a coherent workflow:

1. Import a group of images.
2. Review the pages in a visible queue.
3. Correct the document quadrilateral in an image canvas.
4. Preview a scan effect.
5. Apply settings to one page or many pages.
6. Export results with visible progress and useful error recovery.

### 2.3 Initial supported inputs

Support these raster formats in the first release:

- JPEG and JPG
- PNG
- BMP
- TIFF

Unsupported formats must produce a clear, recoverable error. Do not add PDF,
RAW, video, camera capture, or cloud storage in the first release unless the
product scope is explicitly revised.

### 2.4 Product goals

- Provide a usable local desktop workflow with no terminal interaction.
- Make manual perspective correction accurate and easy to review.
- Keep preview interaction responsive while processing full-resolution output
  in the background.
- Make batch processing observable, cancellable, and recoverable.
- Preserve the original image and make export behavior explicit.
- Make the processing core testable without a GUI or GPU.
- Support macOS, Windows, and Linux through portable Rust code.

### 2.5 Non-goals for the first release

- OCR, searchable PDF, document text extraction, or language recognition.
- Automatic cloud backup, account management, or collaborative editing.
- Mobile applications.
- A general-purpose photo editor.
- Video or live camera scanning.
- Dependence on Blender, ImageMagick, a system command, or another external
  conversion process at runtime.

## 3. Proposed User Experience

### 3.1 Main application layout

Use a three-region desktop layout with a persistent bottom action/status bar.

```text
+----------------+--------------------------------------+-------------------+
| Page queue     | Image canvas                         | Inspector         |
|                |                                      |                   |
| thumbnails     | original or processed preview        | perspective       |
| status         | draggable quadrilateral               | enhancement       |
| selection      | zoom / pan / fit                      | output settings   |
|                | before-and-after mode                | apply-to-all      |
+----------------+--------------------------------------+-------------------+
| Previous | Next | Undo | Redo | Reset | Export | Progress | Messages      |
+------------------------------------------------------------------------+
```

The exact visual design can evolve, but the responsibilities should remain
stable: queue navigation on the left, visual editing in the center, and
parameters on the right.

### 3.2 Import flow

- Allow the user to select individual files or a directory.
- Add files in deterministic filename order unless the user explicitly sorts
  the queue.
- Show a thumbnail, filename, dimensions, and processing status for every
  item.
- Skip unsupported or unreadable files with an item-level error instead of
  aborting the entire import.
- Do not duplicate large source images unnecessarily in memory.

### 3.3 Canvas flow

The canvas must support:

- Fit image to available space.
- Continuous zoom with a visible zoom indicator.
- Pan with a secondary mouse gesture or a dedicated interaction mode.
- Four draggable corner handles.
- A polygon overlay connecting the four handles.
- Point labels and a clear selected-handle state.
- Reset view and reset quadrilateral independently.
- Undo and redo of meaningful editing actions.
- Optional side-by-side or split before/after preview.

The initial interaction should use four handles rather than requiring four
one-time clicks. A new page may start with a default full-image quadrilateral;
the user can refine it by dragging the corners.

### 3.4 Quadrilateral validation

Before a preview or export is accepted, validate that the quadrilateral:

- Has exactly four finite points.
- Is inside the canonical image bounds, or is clipped by an explicit and
  documented policy.
- Is convex and non-self-intersecting.
- Has non-zero area.
- Has no edge below a minimum practical length.
- Produces positive, finite output dimensions.

Display a local validation message next to the affected controls. Do not write
an output file for an invalid quadrilateral.

### 3.5 Enhancement controls

The first release should expose these named presets:

- Original
- Adaptive black and white
- Enhanced color
- Magic color

The presets should be backed by serializable parameters, even if only a subset
of the parameters is exposed initially. This allows future sliders without
changing the session model.

Recommended initial controls:

- Black-and-white block size and threshold offset.
- Color brightness, contrast, denoise strength, and sharpening strength.
- Magic-color local contrast and saturation.
- Output format and JPEG quality where applicable.

Changing a control should update a preview asynchronously. Provide an
explicit way to apply the current preset to all compatible queue items.

### 3.6 Navigation and keyboard behavior

Every keyboard shortcut must also have a visible menu item, tooltip, or button
equivalent. Suggested defaults:

- `O`: open files or folder.
- `Enter`: accept the current page and move to the next page.
- `N`: next page.
- `P`: previous page.
- `R`: reset view.
- `Z` / `Shift+Z`: undo / redo, subject to platform conventions.
- `Escape`: cancel an active operation or clear a transient interaction.
- `Ctrl/Cmd+S`: save session metadata if session persistence is enabled.
- `Ctrl/Cmd+E`: export.

Do not require the terminal for normal use.

## 4. Technical Stack

### 4.1 Required stack

- Rust stable, pinned through a repository toolchain file.
- `eframe` and `egui` for the native application shell and UI.
- OpenCV Rust bindings behind a processing adapter for the first release.
- A Rust image representation suitable for preview conversion and texture
  upload.
- `rfd` or an equivalent cross-platform file dialog.
- `rayon` or standard worker threads for independent image jobs.
- `serde` and `serde_json` for session/configuration data.
- `thiserror` for domain errors and `anyhow` only at suitable application
  boundaries.
- `tracing` with a file appender for diagnostics.

Do not lock the architecture to exact dependency versions in this document.
Choose compatible current versions, commit the lockfile, and update them as
deliberate dependency changes.

### 4.2 Why native egui is the initial choice

The application is local, canvas-centric, and single-user. Native egui avoids
introducing a second frontend language, an IPC boundary, and a second state
management system. It also matches the implementer's existing Rust and egui
experience.

Tauri, a Web frontend, or a different desktop framework can be reconsidered
if the product later requires a shared web UI, mobile targets, rich HTML
layout, or a larger frontend team. Such a decision should be recorded in an
ADR rather than introduced incrementally as an unplanned second UI stack.

## 5. Repository and Module Architecture

Prefer a Cargo workspace when the implementation begins. If a single crate is
initially more productive, preserve the same dependency boundaries as modules
and split crates when the boundaries become stable.

```text
Cargo.toml
rust-toolchain.toml
crates/
  scanner-core/
    src/
      lib.rs
      model.rs
      geometry.rs
      pipeline.rs
      effects.rs
      export.rs
      error.rs
      backend/
  scanner-app/
    src/
      main.rs
      app.rs
      state.rs
      ui/
      canvas/
      render/
      tasks/
      persistence/
  scanner-cli/
    src/main.rs
tests/
  fixtures/
docs/
```

The intended dependency direction is:

```text
scanner-app ───────┐
scanner-cli ───────┼──> scanner-core ───> image/OpenCV adapters
                   │
                   └──> platform/UI dependencies stay outside core
```

### 5.1 `scanner-core`

Own the product's source of truth for image operations:

- Source image metadata and canonical orientation.
- Stable identifiers and serializable edit state.
- Point, quadrilateral, bounds, and geometry validation.
- Perspective transform calculations.
- Enhancement presets and parameters.
- Preview and final processing orchestration.
- Output encoding and post-export validation.
- Domain-specific errors.

`scanner-core` must not depend on egui, eframe, wgpu, window handles, file
dialogs, or UI state. Keep OpenCV-specific values behind a small adapter so
the application is not coupled to `Mat` or generated binding details.

### 5.2 `scanner-app`

Own the desktop experience:

- Process startup and the eframe application object.
- Top-level state and page navigation.
- Queue presentation and selection.
- Canvas presentation and pointer interaction.
- Texture cache and preview lifecycle.
- Task submission, polling, cancellation, and stale-result handling.
- User-facing errors, dialogs, status messages, and keyboard commands.

UI modules express user intent. They must not parse image file formats, call
OpenCV directly, write output files, or know worker implementation details.

### 5.3 `scanner-cli`

Provide a small optional headless surface for deterministic batch processing,
regression testing, and automation. It should call `scanner-core`, not copy
processing logic. It must not become a second product workflow with behavior
that diverges silently from the GUI.

## 6. Domain Model

The names below are conceptual contracts. The implementation may choose
different Rust names, but the concepts and ownership should remain explicit.

### 6.1 Source asset

`SourceImage` should contain:

- A stable in-session identifier.
- A source path held as a `PathBuf`.
- A display name derived for the UI.
- Original dimensions and color/decode metadata that is relevant to output.
- Canonical orientation information.
- A queue status and the most recent error, if any.

The original source path may be stored in a session file, but it must not be
written verbatim to normal logs when it could expose private information.

### 6.2 Edit state

`EditState` should contain only user intent and serializable parameters:

- A quadrilateral in canonical image coordinates.
- Rotation or orientation adjustments, if supported.
- Enhancement preset and its parameters.
- Crop or margin settings, if supported.
- Export format and quality settings.
- A dirty flag or equivalent revision number.

Do not put `TextureHandle`, GPU resources, decoded cache buffers, channels, or
worker handles into the serializable edit state.

### 6.3 Coordinate contract

Use one canonical coordinate system:

1. Decode and normalize the image orientation.
2. Store points as normalized coordinates or precise coordinates in that
   canonical image space.
3. Convert to pixel coordinates only at the processing boundary.
4. Map canonical image coordinates to screen coordinates for rendering.
5. Use the inverse mapping for pointer input.

The texture rectangle is a rendering detail. It must never be the source of
truth for a document corner.

### 6.4 Processing state

Each queue item should expose a state similar to:

- `NotStarted`
- `Loading`
- `Ready`
- `Previewing`
- `Queued`
- `Processing`
- `Completed`
- `Skipped`
- `Failed`
- `Cancelled`

The UI must distinguish a stale preview from a current preview. A result from
an older edit revision must be discarded rather than replacing a newer result.

## 7. Image Processing Pipeline

The default pipeline is:

```text
Read source
  -> decode and validate dimensions
  -> normalize orientation
  -> validate edit state
  -> perspective warp
  -> apply enhancement preset
  -> encode selected output format
  -> atomically write output
  -> reopen and validate generated image
```

Preview processing may use a downscaled working image, but it must use the
same operation order and parameter semantics as final processing.

### 7.1 Perspective transform

- Order points deterministically before computing the transform.
- Reject duplicate, collinear, self-intersecting, or near-zero-area input.
- Use finite numeric calculations and checked output dimensions.
- Keep the transform implementation independent of the UI selection order.
- Add fixtures for landscape, portrait, skewed, and near-boundary documents.

### 7.2 Enhancement presets

Presets must be deterministic functions of an input image and a parameter
object. They must not read UI state, global configuration, or environment
variables.

The first implementation may reproduce the prototype's OpenCV operations:

- Adaptive thresholding for black and white.
- Denoising, sharpening, brightness, and contrast for enhanced color.
- Lab/CLAHE-based local contrast and saturation adjustment for magic color.

Do not change algorithm defaults casually during the port. If output behavior
changes, document it and update fixture expectations deliberately.

### 7.3 Preview and final output

- Preview jobs operate on a bounded working resolution.
- Export jobs always reload or access the original-resolution source safely.
- Preview output must never be mistaken for the export source.
- Use explicit output format and quality settings.
- Avoid overwriting an existing output by default.
- Handle duplicate names deterministically.
- Write through a temporary file in the destination directory, flush and
  close it, then atomically replace the final path where the platform permits.

## 8. Background Tasks and Message Boundaries

The eframe/egui update loop must remain short and non-blocking.

Workers receive immutable job inputs containing:

- A task identifier.
- An image/session identifier.
- An edit revision or generation number.
- A source reference or safely prepared working data.
- Processing parameters.
- A cancellation handle.

Workers return typed events such as:

- `PreviewReady`.
- `ExportProgress`.
- `ExportCompleted`.
- `TaskFailed`.
- `TaskCancelled`.

Only the UI thread may mutate egui state or upload textures. After receiving a
worker event, request a repaint. Do not poll with arbitrary sleeps in the UI
loop.

When a user changes a slider or moves a point repeatedly, debounce or cancel
obsolete preview work. Use the revision number as the final guard against
stale results.

## 9. Persistence and Export

### 9.1 Session persistence

Session persistence is recommended after the basic editing workflow is stable.
A session file should contain:

- Source references and lightweight metadata.
- Queue order and per-item edit state.
- Output settings.
- Application schema version.

It should not embed original images by default. If a source cannot be found
when reopening a session, show a relink action instead of silently dropping
the item.

Version the session schema from its first release and provide a useful error
for newer or incompatible schemas.

### 9.2 Export semantics

- Export one selected item or the entire queue.
- Show destination and collision policy before starting a batch.
- Keep partial successes visible.
- Allow failed items to be retried after the cause is corrected.
- Never report success until the output can be reopened and basic dimensions
  and format checks pass.

## 10. Error Handling and Safety

Treat every image file, path, point, and persisted session as untrusted input.

- Reject malformed or unreadable image data with a structured error.
- Protect against unreasonable dimensions and allocation sizes.
- Reject non-finite coordinates and invalid output dimensions.
- Never panic on malformed user files or normal filesystem failures.
- Include item and operation context in errors without exposing secrets.
- Preserve the source when export fails.
- Make cancellation and partial failure explicit in the UI.

Avoid silently changing orientation, color interpretation, output format, or
quality. If a decoder cannot preserve a property, document the limitation or
surface it as a warning.

## 11. Testing Strategy

### 11.1 Unit tests

Cover at least:

- Point ordering.
- Convexity and self-intersection checks.
- Degenerate quadrilateral rejection.
- Bounds and finite-number validation.
- Screen-to-image and image-to-screen coordinate round trips.
- Output dimension calculation.
- Enhancement parameter validation.
- Collision-name generation.
- Session serialization and schema rejection.

### 11.2 Image fixtures

Keep small, legally distributable fixtures under `tests/fixtures`:

- A flat page.
- A perspective-skewed page.
- Portrait and landscape images.
- A high-contrast page.
- A color page.
- Invalid or truncated input samples where practical.

Use tolerant image comparisons where the backend or encoder makes exact bytes
unstable. Test exact dimensions, channel expectations, representative pixel
regions, and deterministic parameters.

### 11.3 Integration tests

Verify that:

- An imported source is never modified.
- A valid session can preview and export an image.
- An exported image can be reopened by the project reader.
- Invalid input fails without producing a misleading output.
- A batch continues after an item-level failure.
- Cancellation leaves a clear state and no falsely successful result.
- A stale preview cannot replace a newer preview.

### 11.4 GUI verification

GUI behavior should have concise, platform-neutral manual verification steps
in `docs/MANUAL_TESTING.md`. At minimum, verify:

1. Open files and a directory.
2. Select a page and drag every corner.
3. Zoom, pan, reset, undo, and redo.
4. Switch presets and observe the preview.
5. Export one page and a batch.
6. Cancel a long-running operation.
7. Recover from a missing source and a write failure.
8. Quit and reopen without corrupting session or source files.

## 12. Delivery Roadmap

### Milestone 0: Repository foundation

Deliver:

- Cargo workspace or clearly separated modules.
- Stable toolchain configuration.
- Basic error, logging, and configuration conventions.
- CI checks for formatting, compilation, tests, and diff hygiene.
- Initial architecture decision record.

Exit criteria: the project builds on the maintained platforms or has a
documented platform-specific setup gap, and the core crate can be tested
without starting a window.

### Milestone 1: Core behavior parity

Deliver:

- Image loading and validation.
- Point ordering and quadrilateral validation.
- Perspective transform.
- The three enhancement modes plus original output.
- Deterministic export and round-trip tests.

Exit criteria: fixture tests demonstrate behavior close to the prototype and
all generated images can be reopened successfully.

### Milestone 2: Native GUI shell

Deliver:

- eframe/egui application window.
- Queue and thumbnail model.
- Canvas with fit, zoom, pan, and coordinate conversion.
- Draggable quadrilateral handles.
- Basic menus, shortcuts, status messages, and file dialogs.

Exit criteria: a user can manually correct and export one image without using
the terminal.

### Milestone 3: Preview and editing workflow

Deliver:

- Asynchronous preview tasks.
- Preset selector and parameter panel.
- Before/after preview.
- Undo/redo and per-item dirty state.
- Stale-result protection.

Exit criteria: repeated point dragging and parameter changes keep the UI
responsive and always display the newest valid result.

### Milestone 4: Batch processing

Deliver:

- Apply-to-all behavior.
- Batch queue, progress, cancellation, retry, and partial-failure reporting.
- Collision policy and atomic output.
- Optional session save/load.

Exit criteria: a mixed batch completes with accurate per-item status and no
source mutation.

### Milestone 5: Product polish and release

Deliver:

- Native application icons and packaging workflow.
- First-run and empty-state guidance.
- Consistent error messages and diagnostics.
- Manual verification on macOS, Windows, and Linux.
- Release notes and supported-platform documentation.

Exit criteria: a clean machine can install, launch, process, and export a
representative batch without a development environment or terminal.

### Later candidates

Consider these only after the core workflow is reliable:

- Automatic document edge detection with manual correction.
- Rotation and deskew tools.
- Multi-page PDF export.
- OCR and searchable output.
- Camera capture.
- Additional color profiles and metadata preservation.
- A separate WebAssembly or web client sharing the Rust core where practical.

## 13. Risks and Mitigations

### OpenCV build and packaging complexity

Mitigation: isolate OpenCV in `scanner-core` adapters, document supported
toolchains, keep fixture tests independent from the GUI, and evaluate native
Rust image operations only after behavior parity is established.

### Large-image memory pressure

Mitigation: use bounded previews, avoid duplicate buffers, release inactive
previews, limit concurrent full-resolution jobs, and report allocation or
decode failures clearly.

### Stale asynchronous results

Mitigation: attach an edit revision to every preview request and reject results
from older revisions.

### Coordinate drift between preview and export

Mitigation: store canonical image coordinates, centralize mapping code, test
round trips, and never use the displayed texture dimensions as authoritative
data.

### UI quality plateau

Mitigation: define the layout and interaction model before adding advanced
processing features; keep styling, keyboard commands, and empty/error states
as explicit UI responsibilities.

### Scope expansion

Mitigation: keep OCR, PDF, camera, and cloud features out of the first
milestones unless a written product decision changes the scope.

## 14. Definition of Done

A feature is complete only when:

- Its behavior is represented in the appropriate core or UI boundary.
- User input and filesystem failures are handled without panic.
- Relevant unit or integration tests exist.
- Image-producing changes include fixture or round-trip coverage.
- GUI-only behavior has platform-neutral manual verification steps.
- Logs use the relevant stable feature prefix.
- `cargo fmt --all`, relevant tests/checks, `git diff --check`, and final diff
  inspection have been completed.
- No generated logs, local databases, build artifacts, credentials, or user
  image outputs are committed.
- Documentation clearly labels implemented, planned, unsupported, and
  experimental behavior.

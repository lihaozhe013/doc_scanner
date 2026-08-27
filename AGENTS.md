# Repository Engineering Policy

These rules are mandatory for every change in this repository. Keep this file
focused on durable engineering policy. Feature plans, release notes, design
decisions, investigations, and manual test procedures belong in dedicated
documents under `docs/`.

## 1. Language and Communication

- All source-code comments, Rust doc comments, commit messages, pull request
  descriptions, and engineering documentation MUST be written in English.
- Chinese text MUST NOT be added to comments or engineering documentation.
  Localized user-facing strings are exempt and MUST remain separate from
  engineering documentation whenever practical.
- Names and prose MUST be clear enough to explain intent. Do not add comments
  that merely restate the code.
- Do not add emojis or decorative comments to source, documentation, or commit
  messages.

## 2. Product Scope and Boundaries

- This repository is a cross-platform desktop application for importing,
  previewing, perspective-correcting, enhancing, and exporting local raster
  document images.
- The initial supported formats are JPEG/JPG, PNG, BMP, and TIFF. Do not add
  PDF, RAW, video, camera capture, OCR, cloud storage, or account workflows
  unless the product scope is explicitly changed.
- The application MUST be offline-first. Runtime behavior MUST NOT depend on
  Blender, ImageMagick, a shell command, a system GUI utility, a local web
  server, or another external conversion process.
- Original source images MUST NOT be modified in place. All edits MUST remain
  non-destructive until the user explicitly exports them.
- The GUI is the primary product workflow. A CLI may exist for automation and
  regression testing, but it MUST call the same processing core and MUST NOT
  silently diverge from GUI behavior.
- Planned, experimental, unsupported, and implemented behavior MUST be clearly
  labeled in documentation.

## 3. Architecture and Dependency Direction

The application is intentionally split into layers. Dependencies flow from
the UI and application coordination toward domain services, never in the
opposite direction.

- `src/main.rs` or the application binary entry point owns only process
  startup, native window creation, and the eframe/egui render loop setup.
- `app` modules own top-level application state, queue navigation, task
  polling, persistence coordination, and user-facing status.
- UI modules own egui presentation and user intent. UI code MUST NOT parse
  image formats, call OpenCV directly, write output files, or invoke worker
  implementation details.
- Canvas modules own image viewport layout, zoom, pan, pointer hit testing,
  coordinate conversion, handles, and overlays. Canvas rendering MUST NOT
  become the source of truth for edit data.
- Core/domain modules own image metadata, edit state, geometry validation,
  perspective transformation, enhancement, export, and domain errors.
- The processing core MUST NOT depend on egui, eframe, wgpu, window handles,
  file dialogs, or UI state.
- OpenCV-specific types and generated binding details MUST remain behind a
  narrow core adapter. Do not expose `Mat` or FFI implementation details to UI
  modules or persisted state.
- Background workers receive immutable job inputs and communicate with the UI
  through typed channels or an equivalent explicit message boundary.
- Only the UI thread may mutate egui state or upload GPU textures.
- Keep public APIs small and predictable. Avoid global mutable state, circular
  module dependencies, and convenience modules that become dumping grounds.

## 4. Image and Coordinate Safety

- Treat image files, persisted sessions, paths, and all user-provided numeric
  values as untrusted input.
- Validate image dimensions, allocation sizes, decode results, channel formats,
  finite numeric values, and output dimensions before processing.
- Store quadrilateral points in one canonical image coordinate system. The
  displayed texture rectangle is a rendering detail and MUST NOT be the source
  of truth.
- Reject duplicate, non-finite, collinear, self-intersecting, or near-zero-area
  quadrilaterals with structured errors.
- Do not silently infer destructive orientation, unit, color, quality, or format
  conversions from a filename or arbitrary image dimensions.
- Preview and final export MUST use the same operation order and parameter
  semantics, even when preview uses a smaller working image.
- Export MUST default to a collision-safe behavior. Do not overwrite an
  existing user file without an explicit policy.
- Write generated images through a temporary file and atomic replacement where
  the platform permits.
- Reopen every generated image through the project reader before reporting
  successful export.
- Never use `panic!`, `unwrap`, or `expect` for malformed user input, image
  content, persisted data, normal filesystem failures, or worker results.
- Any use of `unsafe` outside a well-established dependency MUST be isolated,
  justified, and reviewed in the change description.

## 5. UI, Rendering, and Task Execution

- The eframe/egui update loop MUST remain short and non-blocking.
- Full-resolution decode, perspective transforms, enhancement, encoding, and
  batch processing MUST run outside the UI update loop.
- Attach an image/edit revision to every preview request and discard results
  from older revisions.
- Use bounded preview resolutions and avoid unnecessary copies of large image
  buffers.
- Do not store egui texture handles, GPU resources, worker handles, channels,
  or transient caches inside serializable edit state.
- Keep user intent separate from derived preview data.
- Every keyboard shortcut MUST have a visible UI equivalent or discoverable
  menu/tooltip path.
- Errors, cancellation, partial batch failure, and stale work MUST have visible
  user-facing states; logging alone is insufficient.

## 6. Cross-Platform Requirements

- New functionality MUST support every maintained platform unless the task
  explicitly narrows its scope.
- The maintained desktop targets are macOS, Windows, and Linux unless a
  documented product decision changes them.
- A Windows-only toolchain, PowerShell script, batch file, registry operation,
  or Win32 command MUST NOT be the sole implementation of build, test,
  development, or maintenance workflows.
- Prefer portable Rust code and established cross-platform crates. Isolate
  unavoidable platform-specific behavior behind explicit `cfg` boundaries and
  provide equivalent behavior for other maintained platforms.
- Use `std::path::Path` and `PathBuf` for filesystem paths. Do not hardcode
  path separators, drive letters, home directories, or executable suffixes in
  shared code.
- Use cross-platform file dialogs, window APIs, image loading, and atomic file
  replacement. Do not make a GUI acceptance path depend on one operating
  system.
- Platform-specific packaging scripts are allowed only inside the relevant
  packaging workflow. They MUST NOT become prerequisites for normal
  development on other platforms.
- Do not introduce environment variables for routine configuration when a
  command-line option, configuration file, or stable application default is
  sufficient. Any required environment variable MUST be documented and kept
  to the narrowest possible scope.
- If repository automation cannot reasonably be implemented in Rust, prefer a
  portable Python standard-library script and invoke it with `uv run`. Do not
  create parallel shell, PowerShell, and batch implementations for the same
  workflow.

## 7. Logging and Debugging

- Application logs MUST be written to `debug.log` by default. Running the GUI
  MUST NOT require stdout or stderr redirection to capture logs.
- Normal application logging MUST NOT write to the terminal. Startup MUST
  remain resilient if the log file cannot be created.
- `RUST_LOG` may be used as an optional log-level override, but the application
  MUST provide a useful default without it.
- Never log passwords, tokens, private keys, credentials, local secrets, or
  complete user-provided paths when they may contain sensitive information.
  Prefer an item identifier or display filename.
- Logs added for a feature or investigation MUST use a stable feature prefix,
  such as `[image_import]`, `[canvas]`, `[processing]`, `[batch]`, or `[export]`.
- When handing off a debugging workflow, provide a ready-to-run command that
  exercises the relevant flow and filters `debug.log` into a focused log file.
  For example, after exercising the flow and closing the application:

  ```bash
  cargo run
  rg "\[(canvas|processing|export)\]" debug.log > focused-debug.log
  ```

- Generated `*.log` files MUST remain untracked and MUST NOT be included in
  commits or release archives.

## 8. Code Organization and File Size

- Preserve existing structure and formatting unless a refactor is part of the
  requested change.
- Every source file over 1,000 lines MUST trigger an explicit design review
  before more responsibilities are added. Evaluate cohesion, dependency
  direction, state ownership, and whether behavior can move to focused
  modules.
- Do not allow a file to cross the 1,000-line threshold without recording the
  assessment in the change summary or commit body.
- When modifying an existing file that already exceeds 1,000 lines, avoid
  increasing its scope. If the affected behavior has a clear boundary, split
  it during the change. If an immediate split would make the change riskier,
  state the reason and identify the intended module boundary.
- New modules MUST have one clear responsibility. Keep entry points and
  application coordinators thin.
- Prefer `cargo fmt`-standard Rust, explicit imports, and `Result`-based error
  propagation with `thiserror` or `anyhow` at appropriate boundaries.
- Do not add comments that merely narrate obvious control flow.

## 9. Required Validation

- Before every commit, run `cargo fmt --all`.
- After formatting, run the most relevant automated checks. `cargo test` is the
  minimum default for Rust behavior changes; use `cargo check --all-targets`
  when a full test run is not applicable.
- Run focused geometry, image fixture, serialization, and export round-trip
  tests for changes affecting those areas.
- Run `cargo clippy --all-targets --all-features -- -D warnings` when the
  dependency set and platform permit it, or clearly document why it could not
  be run.
- GUI behavior that cannot be validated reliably in the agent environment
  MUST be handed off with concise, platform-neutral manual verification steps
  under `docs/`.
- Do not report a check as successful unless it was actually run. Clearly state
  any check that could not be completed and why.
- Before declaring work complete, run `git diff --check`, inspect the final
  diff, and confirm generated artifacts are not included.
- Image-producing changes MUST verify that the source is unchanged and that
  generated files can be reopened successfully.

## 10. Documentation and Task Tracking

- `README.md` contains the product overview, supported scope, user entry
  points, and developer setup.
- Detailed architecture, feature plans, migration notes, design decisions,
  release notes, investigations, and manual test procedures belong in focused
  English documents under `docs/`.
- Documentation MUST describe actual behavior. Clearly label planned,
  unsupported, experimental, and platform-specific behavior.
- Do not maintain stale checklists of completed work. When a task is complete,
  remove its pending entry from the relevant planning document or record the
  completed decision in the appropriate document.
- Keep architectural decisions in ADRs when a choice affects dependencies,
  data formats, persistence, platform support, or public behavior.

## 11. Commits and Git Operations

- Inspect `git status` before editing. Preserve unrelated user changes.
- Every commit MUST use a complete Conventional Commits message:
  `<type>(optional-scope): imperative summary`.
- Use the narrowest accurate type, such as `feat`, `fix`, `refactor`, `docs`,
  `test`, `build`, `ci`, or `chore`.
- Non-trivial commits MUST include a body explaining motivation, behavior
  changes, compatibility considerations, and validation performed.
- Breaking changes MUST use `!` in the header or a `BREAKING CHANGE:` footer.
- When creating a commit, append a `Co-Authored-By` trailer based only on the
  model identity available in the current execution context. Do not invent a
  more specific identity than the context supports.
- Do not commit, amend, push, create a branch, or create a pull request unless
  the user explicitly requests that operation.
- Do not use destructive commands such as `git reset --hard`, force-push, or
  broad recursive deletion without explicit user approval.

## 12. Repository Hygiene and Sensitive Data

- Never commit generated logs, credentials, private keys, build artifacts,
  local profile databases, temporary files, or user image outputs.
- Keep test fixtures small, legally distributable, and free of personal data.
- Use `.gitignore` for logs, build directories, local sessions, caches, and
  generated exports.
- Preserve unrelated changes and do not rewrite user-owned files outside the
  requested scope.
- Use `rg` for text search, `fd` for file discovery, and `apply_patch` for
  source and documentation edits where practical.
- Prefer recoverable operations when cleanup is explicitly authorized, and
  confirm exact targets before deleting or overwriting material data.


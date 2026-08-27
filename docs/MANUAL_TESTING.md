# Manual GUI Verification

These checks cover the GUI behavior that cannot be fully exercised by the
headless test suite. Use small JPEG, PNG, BMP, and TIFF files and keep the
originals in a separate directory.

1. Start the app with `cargo run -p scanner-app`.
2. Use **Open files** and **Open folder**. Confirm supported files appear in
   deterministic filename order and unsupported or unreadable files produce an
   item-level error.
3. Select a page. Confirm the queue shows its dimensions and the canvas shows
   four corner handles. Drag each handle, then use **Fit**, zoom, and canvas
   panning. Confirm the quadrilateral remains the source of truth rather than
   the displayed texture rectangle.
4. Switch between **Original**, **Adaptive black and white**, **Enhanced
   color**, and **Magic color**. Change an exposed parameter and confirm the
   preview updates without blocking the window.
5. Use **Undo**, **Redo**, and **Reset quad**. Confirm a stale preview cannot
   replace the newest edit after repeated point dragging.
6. Export the selected page to an empty directory. Reopen the exported image
   in another viewer and confirm the source file timestamp and contents are
   unchanged.
7. Export the queue to the same directory twice. Confirm the second run uses
   collision-safe numbered names and reports per-item results.
8. During a larger export, use **Cancel**. Confirm queued items cancel
   immediately and an active item is cancelled after its current core
   operation finishes; no cancelled item is reported as successfully exported.
9. Save and reopen a session. Move one source file temporarily and confirm the
   missing item is reported instead of silently discarded.
10. Language: switch the **Language** picker between English and 简体中文.
    Confirm the toolbar, queue, inspector, canvas, status messages, and file
    dialog filters all change language, the window title updates, Chinese
    text renders with the bundled font, and the choice survives an app
    restart. With **System default** selected, confirm the language matches
    the OS locale on restart. See [docs/I18N.md](I18N.md).
11. Repeat the smoke test on macOS, Windows, and Linux. Record any native file
    dialog or renderer setup gap in the issue tracker before release.

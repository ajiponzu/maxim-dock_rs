---
name: maximdock-development
description: Implement or review MaXImDock Rust Dock changes, including hidden-window recovery, physical-pixel geometry, Windows adapters and phase acceptance records. Use only in the MaXImDock repository.
---

# MaXImDock development

Read `docs/codex-implementation-brief.md` completely before phase implementation. Read `docs/notes.md` and `docs/acceptance.md` for existing evidence and remaining work. Scope implementation to the requested phase.

Preserve these project-specific invariants:

- `core` geometry and visibility transitions remain testable without egui or Win32. Negative coordinates and all four edges are supported even while monitor selection is primary-only.
- Hidden windows receive no regular egui drawing. Continue recovery through eframe `App::logic` and an independent repaint wake, not hidden UI pointer events. Read the selected eframe/winit source when upgrading this behavior.
- Cursor/monitor/anchor coordinates are physical pixels. Convert egui points to pixels only in the Windows window adapter. Keep HWND lifetime bound to eframe's App and perform native window operations on the UI thread.
- Keep show/hide ownership consistent. The current adapter uses native ShowWindow for both; mixing native show with winit cached visibility mutation can change activation behavior. Preserve SW_SHOWNOACTIVATE and clickability.
- Inspect Win32 error conventions; ShowWindow returns previous visibility, whereas ShellExecuteW values <=32 are failures. Do not treat those results identically.

For visibility changes, run fmt, clippy with `-D warnings`, unit tests and `cargo run --locked -- --smoke-test`. The smoke deliberately uses synthetic hot-zone entry; record actual mouse-edge, Shell launch and DPI checks separately. Add tests for changed invariants rather than documentation wording.

Before adding dependencies, record purpose and alternatives in notes. Maintain Phase 1/2/3 acceptance status, reasons for deferred features and unverified manual conditions. Do not mark manual acceptance passed from synthetic input alone.

For Phase 2 configuration changes, preserve version/UUID/order and Top as the default edge. Missing saved paths can represent unplugged drives; validate syntax on load, existence at addition/launch. Do not overwrite unreadable/invalid configurations until explicit byte-preserving backup. Apply only after validation and successful same-directory temporary-file replacement. Check the loaded snapshot before saving to catch external edits.

Drawing produces commands; Shell launch, persistence and dialogs execute outside draw functions. Keep native dialogs off the UI thread so hidden polling continues. Settings use a child viewport and closing that viewport must not close the root app. The native smoke uses a temporary config and verifies Apply/save/reload, immediate edge repositioning and settings with a hidden root.

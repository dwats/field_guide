# Field Guide

Local-only wiki for building up knowledge of a subject as you learn it: entries, nested tags, typed relations between entries, and an optional board view of those relations. Desktop app: Tauri 2 shell, Rust core, Svelte UI. No cloud. MIT licensed.

Read `docs/design.md` before touching the data model, relations, storage, or export code. `docs/plan.md` has the build order and the tests that gate each phase. Work phases in order unless told otherwise.

## Names

- Product and window title: "Field Guide" (two words, always).
- Top-level unit the user creates: a guide. "Minecraft guide", "export this guide". Never "bucket" or "vault" in UI copy; the vault is the on-disk container and only appears in Settings.
- Unit inside a guide: an entry. "New entry", "this entry has no body yet". Never "note", "article", or "page" in UI copy or code; a future scratchpad feature gets its own name.
- Crate, binary, and config: `fieldguide`. If the crate name is taken on crates.io, use `fieldguide-app` for the published name and keep the binary `fieldguide`.
- Bundle identifier: set in `tauri.conf.json`; use the owner's reverse-domain handle plus `.fieldguide`.

## Hard constraints

- No Electron. Tauri 2 with the OS webview.
- No network access except the user-triggered "Check for updates" menu item, scoped to one release URL in the Tauri capability file. Nothing else may make a request, and nothing runs on startup.
- No telemetry, no accounts, no background sync, no auto-update.
- Data lives in the platform data directory by default. A file named `portable` next to the executable (next to the `.app` bundle on macOS, next to the AppImage on Linux) switches to portable mode, where everything including the webview profile lives beside the executable. Nothing is written outside the data directory in either mode. Never write inside a macOS `.app` bundle. See `docs/design.md` section 2.
- Destructive actions are reversible or explicit: deleted entries go to Trash; media cleanup is a user action that shows what it will remove; renames rewrite links and keep the old title as an alias.
- Everything the user creates must round-trip through export and import without loss.

## Layout

```
core/     Rust crate. Data model, SQLite, relations, search, markdown, export/import, backups. No Tauri dependency.
app/      Tauri 2 crate. Window, menus, single-instance guard, commands that forward to core. Thin.
ui/       Svelte 5 + TypeScript + Vite. Presentation only.
docs/     Design and plan. Update them when behavior changes.
tools/    Walkthrough scripts that drive a release build. Not part of the build or the gates.
```

Business rules live in `core`. If a rule can be tested without a window, it belongs there.

## Commands

- `cargo test --workspace`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all`
- `cd ui && npm test` (Vitest)
- `cargo tauri dev`
- `cargo tauri build`

## Testing

Tests assert behavior, not structure. Prefer round-trip tests (create, export, import, compare) and property-style tests for the relation model. Do not write tests that only check a function was called or that a struct has a field. Each phase in `docs/plan.md` lists the tests that must pass before moving on.

Core tests run against in-memory SQLite. Export, import, backup, and data-directory tests use a temp directory.

## Documentation style

- Lead with the essential fact. Details follow.
- No emojis in docs, comments, or UI copy. Entry content must render them.
- No filler words, no marketing voice, no restating what the code already says.
- Record decisions and their reasons in `docs/design.md`.

## Code conventions

- Rust edition 2024. `thiserror` for error types in `core`; `anyhow` only in `app`.
- Public `core` API takes and returns plain structs. `rusqlite` types never cross the crate boundary.
- IDs are UUID v7 strings. Titles, tag paths, and verb labels are for display and for the export format, never foreign keys.
- Paths are built with `std::path`, never string concatenation. Platform directories come from the `dirs` crate. Paths stored in `config.toml` are relative to the data directory whenever they are under it.
- Files are written to a temporary name in the same directory and renamed into place. This applies to media, exports, backups, and `config.toml`.
- Migrations are numbered SQL files in `core/migrations/`, forward only.
- Line endings are LF everywhere; `.gitattributes` enforces it.
- UI tooling is ESM (Vite requires it). Keep the UI dumb: call `invoke`, render the result, send actions back.

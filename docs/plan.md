# Plan

Build `core` through export/import before writing any UI. The export format and the relation rules are the hard parts; getting them tested first means the UI is a thin layer over something already proven. Phases 0 to 3 are sequential. Phases 4 onward can be reordered.

Each phase lists what "done" means. Do not start the next phase with failing gates.

## Phase 0: Bootstrap

- Cargo workspace with `core/` (lib) and `app/` (Tauri 2). `ui/` scaffolded with Svelte 5 + TypeScript + Vite.
- `rust-toolchain.toml` pinning stable. `.gitattributes` with `* text=auto eol=lf`. `.gitignore` for Rust and Node. `LICENSE` (MIT). A one-paragraph `README.md` in the style from `CLAUDE.md`.
- `rusqlite` with the `bundled` and `backup` features so the binary has no system SQLite dependency. The bundled SQLite compiles FTS5 in; there is no separate feature for it.
- Migration runner in `core`: numbered SQL files, applied forward, version recorded in the DB. Opening a DB with a newer version than the build knows returns a typed error.
- Data directory resolution in `core` (`docs/design.md` section 2): platform default via `dirs`, portable mode when a `portable` file sits next to the executable (or the `.app` / AppImage), relative vault path in `config.toml`.

Tests:
- Data directory: no marker resolves to the platform directory; marker present resolves to the executable's directory; marker present and directory unwritable returns an error, not a fallback.
- Config: a vault path under the data directory is written relative and reads back correctly after the data directory moves.
- Migration version newer than the build is refused.

Done when: `cargo test --workspace`, `cargo clippy -D warnings`, and `cargo fmt --check` pass; `cargo tauri dev` opens an empty window; `cargo tauri build` produces a runnable binary. Bundling is off in `tauri.conf.json` (installers are not part of the distribution plan in `docs/design.md` section 10), so the binary is `target/release/fieldguide`. Icons come from `app/icon.svg` via `cargo tauri icon app/icon.svg -o app/icons`; the generated `app/icons/` is committed.

## Phase 1: Core data model

Guides, tags, entries, aliases, trash, rename, media rows, FTS5 index, backups.

Tests:
- Tag tree: create nested path, move a subtree, rename cascades to display paths, delete reparents children and untags entries, merge retags.
- Ancestor inclusion: an entry tagged `A/B/C` is returned for `A`, `A/B`, and `A/B/C`.
- Title uniqueness is case-insensitive per guide; aliases cannot collide with titles or other aliases in the guide; the same title is allowed in two guides.
- Wikilink resolution hits titles and aliases, is case-insensitive, and returns nothing across guides unless asked.
- Rename: `[[Old]]` and `[[Old|text]]` in referencing entries are rewritten to the new title with `|text` preserved; non-referencing entries are untouched; the old title becomes an alias; undo restores bodies and removes the alias.
- Trash: a trashed entry disappears from browse, search, and wikilink resolution; restore brings it back with tags and gallery intact; purge removes the row and its mentions; a trashed entry's title can be reused by a new entry only after purge.
- FTS: prefix queries return matches in title, alias, and body; results update after edit; deleted entries drop out; a query with and without diacritics matches the same entry.
- Guide browse filter by name substring and by global tags.
- Backup: `backup_vault` produces a file that opens and contains the same entries; a sixth backup removes the oldest; backup runs while the vault is open for writing.

## Phase 2: Core relations

Verbs, qualifier labels, relations, qualifiers, read-side rendering, mentions.

Tests:
- `create_relation` from the subject side stores the canonical edge; from the object side using the inverse label stores the same canonical edge.
- Reading an edge from subject, object, and a qualifier-value entry yields the display forms in `docs/design.md` section 4, for all four combinations of verb inverse present/absent and qualifier inverse present/absent.
- Symmetric verbs render identically from both sides regardless of stored direction.
- Creating an existing `(subject, verb, object)` merges qualifiers and does not create a second edge.
- Same qualifier label twice on one edge is kept with order preserved.
- Stub creation: target that does not exist is created with `is_stub = 1`, suggested tag from the verb's side default, caller may override or omit the tag; a body edit clears the flag.
- New verb label colliding with an existing label or inverse label in the guide is rejected.
- Deleting a verb or qualifier label in use is rejected; merge moves edges and qualifiers, then deletes.
- Trashing an entry hides its edges from both endpoints and hides qualifiers that point at it; restore shows them again; purge deletes them.
- Cross-guide edge: both entries list it; same-guide query excludes it; external toggle includes it; verb belongs to the originating entry's guide.
- Mentions: saving a body with `[[Oak Log]]` creates a mention row, removing the link removes it, mentions never appear in the relation list.
- Copy vocabulary from another guide duplicates verbs and qualifier labels with new ids.

## Phase 3: Core export and import

Archive writer, archive reader, mapping suggestions, import report, transactional apply.

Tests:
- Round trip at every scope (vault, guide, tag subtree, entry): export, import into an empty vault, compare entity by entity including qualifier order, board layouts, aliases, and media hashes.
- Golden test: the archive for a fixture vault matches the layout in `docs/design.md` section 7, every `.md` and `.json` file parses with standard tools, and exporting twice yields identical entries apart from `exported_at` in the manifest.
- Slugs: titles differing only by case or accents get distinct slugs; a 200-character title produces a slug within the cap; slugs contain only `[a-z0-9-]`.
- Trashed entries are absent from the archive, and relations that touch them are absent too.
- Media deduplication: two entries using the same image produce one file in the archive and one `media` row after import.
- Mapping: exact normalized matches auto-map; near matches rank the expected candidate first; unmatched items create new entries; "create under" places a tag at the chosen parent.
- Entry identity: same id with newer `updated` overwrites; older is skipped and reported; title collision with different id renames the incoming entry and reports it.
- Wikilinks in imported bodies resolve against the archive's own entries before the target guide.
- External endpoints resolve by id, then by guide name and title; unresolved ones are reported and not created unless the caller asks for stubs.
- An archive with a newer `format_version` is refused before anything is written.
- A failing import (a damaged media file, or an entry that fails validation after earlier entries were written) leaves the vault's content and media directory unchanged, compared through a second export, since WAL files make a byte comparison of `vault.db` meaningless.
- Importing into a tag target adds that tag to every imported entry.

## Phase 4: Shell and UI skeleton

Tauri commands for everything in phases 1 to 3. Single-instance plugin and vault lock with the "open elsewhere" message. Daily backup on first open. Synced-folder warning. Guide browse (card and table views with name and tag filter). Entry page with CodeMirror editor, preview from `core`, tag picker. Tag editor with drag-to-nest. Settings page showing mode (portable or default), data path, backups with restore, and the update-check button placeholder.

Tests: Vitest for any pure UI logic (filter string parsing, staged-input state machine). A short manual checklist in `docs/checklist.md` for the views. Markdown rendering, backup restore, and sync-folder detection are core behavior and get core tests.

Done when: create a guide, write an entry with a wikilink, nest a tag, and see the preview resolve the link; a second launch focuses the first window.

## Phase 5: Relation builder

The staged input line from `docs/design.md` section 4, the stub prompt, and the relation list on the entry page grouped by verb with both-direction rendering. Edit and delete an edge and its qualifiers.

Done when: the `Stone Block DROPS Cobblestone Block +mined with Pickaxe` example can be entered from Stone Block, from Cobblestone Block, and reads correctly on all three pages.

## Phase 6: Category browse and trash

Tag tree sidebar, live-filtered entry list, stubs-only toggle. Trash view with restore, purge, and empty.

Done when: typing in the filter updates the list on each keystroke with no visible lag at a few thousand entries; a deleted entry can be restored from Trash with its relations intact.

## Phase 7: Board

SVG pan and zoom, force layout for unplaced nodes, persisted positions, named boards with saved filters, external and mentions toggles, tag, text, and topic filters.

Done when: a guide with a few hundred entries stays responsive while dragging, and reopening a board restores positions.

## Phase 8: Media

Drag-and-drop and clipboard paste of images into the editor and gallery. Dropped files arrive through the window's native drag-drop events as paths and core reads them from disk, so nothing large passes through the webview. The tag editor's drag moves to pointer events first, since Tauri's native drop and HTML drag and drop cannot coexist on Windows. Content-addressed storage with atomic writes; images converted to lossless WebP. Guide cover art. "Clean up unused media" in Settings with the file list and total; the unreferenced-size and low-disk notices.

Done when: a pasted screenshot appears inline, in the gallery, and in the export archive exactly once; cleanup lists and removes an orphaned file and nothing else.

Images are converted to lossless WebP on ingest (`docs/design.md` section 3); a lossy option is a later setting (section 11). Video is not part of this phase (see Later).

## Phase 9: Import and export UI

Scope picker for export. Import flow: file picker, target picker, mapping wizard with accept-all, report view with "create stubs" for unresolved relations.

Done when: a friend's guide export imports into your vault with their tags mapped onto yours and nothing lost.

Candidates raised during phase 3, to decide here: a hop-depth export scope with a preview; per-field overwrite choices (cover, description, guide tags) when importing into an existing guide.

## Phase 10: Packaging and update check

- Windows: plain exe, plus a zip with a fixed WebView2 runtime for machines without it.
- macOS: zipped `.app`, unsigned, with the right-click Open entry in the README.
- Linux: AppImage.
- GitHub Actions workflow using `tauri-action` for the macOS and Linux builds.
- In portable mode the webview profile is placed under `webview/` in the data directory.
- "Check for updates" menu item, capability scoped to the release URL, `latest.json` layout documented in `docs/release.md`.

Done when:
- The Windows exe runs from a thumb drive on a clean machine with WebView2, and the app makes no network request until the menu item is clicked (verified with a packet capture or proxy log).
- In portable mode, a Process Monitor capture of a full session (every view, a paste, an export) shows no writes to AppData, the registry, or the user profile.
- The macOS build from Actions launches after right-click Open and finds its data directory next to the `.app` when `portable` is present.

## Later

Not scheduled. Candidates that need their own decision before they get a phase.

- Video: drag-and-drop of `.webm` into entries, playback in the preview. The renderer already turns `media/*.webm` into a video element, so an archive that carries one plays. Without transcoding, which the binary cannot carry, every clip is stored as recorded, so this may not be worth its weight.
- Lossy WebP as a setting beside the lossless default (`docs/design.md` section 11).

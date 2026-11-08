# Design

Field Guide is a local-only wiki you build up while learning a subject. A vault holds guides (one per game, course, or subject). A guide holds notes, a tag tree, and a vocabulary of verbs. Notes are markdown with inline media. Notes relate to each other through typed edges: `Oak Tree DROPS Oak Log`. Browsing is by tag tree with live filtering. The board view is an alternative way to look at the same notes and edges, on a pannable surface, for people who think in relationship maps.

The working store is one SQLite file per vault. The export format is a zip of markdown and JSON that a person can read without the app.

## 1. Architecture

- **Shell:** Tauri 2. One process; the Rust binary owns the window and opens the OS webview (WebView2, WKWebView, WebKitGTK). Frontend assets are compiled into the binary and served over `tauri://`. No local server.
- **Core:** Rust crate with no Tauri dependency. Owns the schema, relation semantics, markdown parsing, search, export/import, and backups. Exposes plain structs. All business rules and their tests live here.
- **UI:** Svelte 5 + TypeScript. Editor is CodeMirror 6. Markdown preview HTML comes from `core` so rendering is identical in the app and in exported HTML, and so wikilink and media resolution is tested once.
- **IPC:** `#[tauri::command]` functions in `app/` forwarding to `core`. Serde JSON both ways.
- **Capabilities:** the frontend gets the commands it needs plus the asset protocol scoped to the vault directory. The only HTTP permission is the update-check URL.

### Why SQLite as the working store, not loose markdown files

Relations, qualifiers, tag trees, aliases, and board layouts are structured data. Keeping them in files next to markdown means either sidecar files per note or front matter that grows past what a person wants to hand-edit. SQLite gives transactions, FTS5, and a single file to copy. Human readability is delivered by export, which is a first-class feature, not an afterthought.

## 2. On disk

Default data directory, per platform, from the `dirs` crate:

```
Windows   %LOCALAPPDATA%\FieldGuide\
macOS     ~/Library/Application Support/FieldGuide/
Linux     $XDG_DATA_HOME/fieldguide/   (usually ~/.local/share/fieldguide/)
```

`%LOCALAPPDATA%` rather than `%APPDATA%` because a vault of screenshots and video must not ride along with a roaming profile.

### Portable mode

A file named `portable` (contents ignored) in the directory containing the executable switches the app to portable mode: the data directory is that same directory. On macOS "the directory containing the executable" means the directory containing the `.app` bundle, never a path inside it. On Linux it is the directory containing the AppImage (`$APPIMAGE`), since the AppImage itself mounts read-only.

If the directory is not writable in portable mode, the app refuses to start and says why. It does not fall back to the platform directory; a thumb-drive user whose data silently lands in AppData has lost the thing they opted into.

In portable mode nothing is written outside the data directory. That includes the webview's own profile (cache, local storage), which is pointed at `webview/` under the data directory. Settings shows which mode is active and the resolved path.

### Layout, in either mode

```
config.toml              vault path, window state, UI prefs
vault/
  vault.db               SQLite, WAL mode
  media/<sha256>.<ext>   raw media files, content-addressed
backups/
  vault-<date>.db        rolling copies of vault.db, media excluded
webview/                 portable mode only
```

The vault path in `config.toml` may point elsewhere (another drive, for example). It is stored relative to the data directory whenever it is under it, so a portable folder survives a drive-letter change or a move to another machine.

### Safety

- **Single writer.** The shell uses the single-instance plugin so a second launch focuses the running window. Core takes an exclusive lock on `vault.db` at open; a vault that is already open elsewhere produces a clear "vault is open in another Field Guide" error rather than a second writer.
- **Synced folders.** If the data directory path contains a known sync root (Dropbox, OneDrive, Google Drive, iCloud Drive), show a one-time warning that SQLite under a sync client can corrupt. Do not block. `config.toml` records the path the warning was shown for, so it returns only when the data directory moves.
- **Rolling backups.** On the first open of each day (UTC), copy `vault.db` into `backups/` with SQLite's online backup API. Files are named `vault-<date>_<time>-<ms>.db` in UTC so name order is time order. Keep the five most recent. Settings lists them with a "restore" action that swaps the current file after taking one more backup of it. Restore first checks that the file is a SQLite database at this build's schema or older and passes `quick_check`, then backs up the current database, closes the vault, copies the backup beside `vault.db`, removes stale `-wal` and `-shm` files, renames the copy into place, and reopens. A refused file changes nothing.
- **Atomic writes.** Media, exports, backups, and `config.toml` are written to a temporary name in the same directory and renamed into place. A yanked drive leaves either the old file or the new one, never a partial.
- **Version gates.** The DB records its migration version and every archive records `format_version`. An older build opening a newer vault, or importing a newer archive, stops with "update Field Guide" instead of guessing.

## 3. Data model

Schema sketch; the migrations are authoritative.

```
guide             id, name, description, cover_media_id, created_at, updated_at
tag               id, scope_guide_id (NULL = global, used for guide tags), parent_id, name, name_norm, color
                  unique (scope_guide_id, parent_id, name_norm)
guide_tag         guide_id, tag_id            (global-scope tags only)
note              id, guide_id, title, title_norm, body_md, is_stub, deleted_at, created_at, updated_at
                  unique (guide_id, title_norm)
note_alias        note_id, guide_id, alias, alias_norm   unique (guide_id, alias_norm); triggers keep aliases and titles in one namespace
note_tag          note_id, tag_id
media             id, sha256, ext, mime, bytes, width, height, duration_ms, created_at
note_media        note_id, media_id, sort      (gallery membership; inline use is derived from body)
verb              id, guide_id, label, label_norm, inverse_label (NULL ok), inverse_norm, symmetric, subject_default_tag_id, object_default_tag_id
                  unique (guide_id, label_norm); labels and inverses share one pool per guide
qualifier_label   id, guide_id, label, label_norm, inverse_label (NULL ok), inverse_norm   same pool rule, in a pool of its own
relation          id, subject_note_id, verb_id, object_note_id, created_at
                  unique (subject_note_id, verb_id, object_note_id)
relation_qualifier id, relation_id, qualifier_label_id, value_note_id | value_text | value_num, sort
mention           note_id, target_norm, target_note_id (NULL while unresolved)   cache of [[wikilinks]], one row per distinct link text
board             id, guide_id, name, name_norm, filters_json, created_at, updated_at   unique (guide_id, name_norm)
board_node        board_id, note_id, x, y          (notes from other guides allowed, for the external toggle)
notes_fts         FTS5 over title, aliases, body; tokenizer unicode61 with remove_diacritics
```

Names are compared through the `*_norm` columns: trimmed, Unicode-lowercased copies that core maintains. SQLite's `lower()` folds only ASCII, so it is not used for uniqueness. Guide names are unique across the vault the same way.

### Tags

- Per-guide tree. A note tagged `Consumables/Potions/Healing` is included when browsing `Potions` or `Consumables` (ancestor inclusion). Display separator is `/`.
- Global-scope tags (guide_id NULL) are the ones applied to guides for the top-level browse view.
- Rename cascades by id. Move is a parent change. Merge retags, moves the source's children under the target, and deletes the source. Deleting a tag with children reparents the children to the deleted tag's parent and untags its notes. In both cases a child whose name is already taken at its new level is merged into the tag that holds it, so nothing is lost and sibling names stay unique.

### Notes

- Title unique per guide, case-insensitive. Aliases share that namespace. `[[Oak Log]]` resolves against titles and aliases. Titles and aliases are one line, trimmed, and may not contain `[[`, `]]`, or `|`, which would break wikilink syntax.
- A stub is a note with `is_stub = 1` and an empty body. Created by the relation builder when the target does not exist. Any body edit clears the flag.
- Body is CommonMark + GFM tables, strikethrough, task lists, plus:
  - `[[Title]]` and `[[Title|shown text]]` wikilinks
  - `![alt](media/<sha256>.<ext>)` for images and video; `.webm` renders as `<video controls>`
- The same body text is stored in the DB and written to the export, unchanged. `core` renders the body to HTML (`markdown::render`); the UI rewrites `media/...` sources to the asset-protocol URL in the rendered DOM, at render time only.
- Raw HTML in a body is shown as text, never passed through. Bodies arrive from other people's archives, and the webview has no need for inline markup.
- Rendered wikilinks carry the resolved note id (`data-note-id`) or the unresolved target text (`data-target`); the UI decides what a click does. The renderer and the mention scanner share one link parser, so a link inside a code span renders as code but still gets a mention row; that divergence is accepted.

### Links

Wikilinks are human-readable in the markdown and id-based underneath. On save, each distinct `[[...]]` target in a note gets a `mention` row keyed by its normalized text, carrying the id it resolves to against titles and aliases in the guide, or NULL while nothing has that name yet. Links are often written before their targets exist, so when a note gains a name (created, renamed, aliased) the rows waiting for that name in its guide resolve, and when a name goes away (alias removed, rename undone, note purged) the rows that reached the note through it go back to unresolved. Trashed targets stay resolved so restore and rename keep working. From then on the id is the link; the text is its serialization. Relations (section 4) use ids directly and never go through titles.

Renaming a note does two things in one transaction:

1. Rewrites `[[Old Title]]` and `[[Old Title|text]]` to the new title in every note that the `mention` table says references it. Only the link target changes; `|text` is kept.
2. Adds the old title as an alias.

The rewrite keeps text and links consistent inside the vault. The alias catches links that arrive later from someone else's archive and still use the old name. Rename is undoable from the note page for the rest of the session. Undo rewrites the current bodies back and removes the alias rather than restoring snapshots, so edits made after the rename survive; only a note's most recent rename can be undone. A rename that changes only the casing rewrites links but adds no alias, since the old title would collide with the new one.

### Trash

Deleting a note sets `deleted_at`. A trashed note is hidden from browse, search, wikilink resolution, the board, and the relation lists of other notes, but its edges, qualifiers, tags, and gallery rows are kept. Restore clears `deleted_at` and everything reappears. Purge (from the Trash view, or "empty trash") deletes the row and cascades: edges where it is an endpoint, qualifiers where it is the value, the mentions it made, and gallery rows. Links in other notes that pointed at it keep their text and become unresolved. A title stays reserved while its note is in the trash; it frees up on purge. Trashed notes are excluded from export.

### Media

- Dropped or pasted files are hashed, copied to `vault/media/<sha256>.<ext>`, deduplicated by hash, and a `media` row is created. Clipboard paste of an image is a first-class path, since Print Screen then Ctrl+V is the common capture flow for game notes.
- Images are converted to WebP on ingest (decided 2026-11-08) so a vault of screenshots stays small; the hash is taken over the stored WebP bytes, so the same source image still deduplicates. Video is stored as dropped; `.webm` is the only format the preview plays.
- Inline placement inserts the markdown reference at the cursor. Gallery placement adds a `note_media` row. A file can be in both.
- Media is vault-scoped, not guide-scoped, so the same screenshot can serve two guides. Export only includes media referenced by the exported notes.
- Media is never deleted automatically. "Clean up unused media" in Settings lists files that no live or trashed note references, with sizes and a total, and deletes them on confirmation. A non-modal notice appears when unreferenced media exceeds 250 MB (adjustable) or free space on the vault's drive drops below 1 GB.

## 4. Relations

This is the part that has to feel right, so the rules are spelled out.

### Definitions

- **Verb:** per-guide, user-defined. `label` (`DROPS`), optional `inverse_label` (`DROPPED BY`), `symmetric` flag (`ADJACENT TO`, inverse = self). Optional default tags for the subject and object side, used only as a suggestion when creating stubs.
- **Relation:** one stored edge `(subject, verb, object)` plus an ordered list of qualifiers. Direction is canonical: subject is the verb's grammatical subject.
- **Qualifier:** `label + value` attached to an edge. Labels are a per-guide pool with their own optional inverse (`mined with` / `mines`); labels and inverses must not collide within that pool, which is separate from the verb pool. Values are a note reference, text, or number. The same label may appear more than once on one edge.

### Reading an edge from each side

Stored: `Stone Block DROPS Cobblestone Block`, qualifiers `[mined with: Pickaxe]`.

| Page | Shows |
|---|---|
| Stone Block | `DROPS Cobblestone Block · mined with Pickaxe` |
| Cobblestone Block | `DROPPED BY Stone Block · mined with Pickaxe` |
| Pickaxe | `MINES Stone Block → Cobblestone Block` |

Without an inverse on the verb, the object side shows `← DROPS Stone Block`. Without an inverse on the qualifier label, Pickaxe shows `mined with · Stone Block DROPS Cobblestone Block` under a "Referenced by" section. Inverses are display rules applied at read time; nothing is stored twice.

Whatever stands in the verb slot renders uppercase, as in these examples, so a verb typed as `drops` and a qualifier inverse typed as `mines` both read as verbs; qualifier labels in qualifier position render as typed. Further qualifiers on the edge follow the line, each separated by `·`. `core` builds these lines (`relations_for_note`) along with the structured parts, so the UI and the exported HTML render them the same way.

### Entry flow

One input line on any note page, staged with autocomplete:

1. **Verb.** Suggestions include labels and inverse labels from this guide. Picking an inverse label flips the direction before storing. Typing a new label prompts for an inverse (optional) and symmetric (default off).
2. **Target.** Suggestions from titles and aliases in this guide, then other guides marked as external. Enter on no match opens the stub prompt: title prefilled, suggested tag prefilled from the verb's default for that side, both editable, tag not required.
3. **Qualifiers.** `+` starts one. Label autocompletes from the pool; a new label prompts for an optional inverse. Value autocompletes against notes; anything that does not resolve is stored as text, or as a number if it parses as one.

`core` exposes `lookup_note(guide, text)` and `create_relation(CreateRelation)` where the request carries the resolved verb id, direction, target id or stub spec, and qualifiers. The UI does staging and prompts; `core` does resolution, validation, and writes. Resolution rules are tested in `core`.

### Rules

- Creating `(subject, verb, object)` that already exists merges qualifiers into the existing edge. No duplicates: a qualifier identical in label and value to one already on the edge is not added again, and new ones keep their order after the existing ones.
- A relation whose verb is symmetric stores whichever direction it was entered from; both pages render the same label, and the reverse edge counts as the same edge when merging.
- Cross-guide edges are allowed. The verb belongs to the guide of the note the edge was created from. Both notes list the edge; views default to same-guide and have a "show external" toggle.
- Trashing a note hides its edges and the qualifiers that point at it; restoring brings them back. Purging deletes them. A trashed note cannot be the origin, target, or qualifier value of a new edge.
- Renaming a verb or qualifier label is a label change on the definition. Deleting one that is in use is blocked; the UI offers merge-into-another. Merging verbs moves the edges, and an edge that would duplicate one the target verb already has merges its qualifiers into that edge instead. Merging qualifier labels relabels the qualifiers and drops exact duplicates that result.
- Mentions (`[[wikilinks]]` in body text) are not relations. They are extracted on save into the `mention` table and shown as a thin, hideable "mentions" edge. They never appear in the relation list on a note page.

### Starter vocabulary

None. A new guide has zero verbs and zero qualifier labels. Vocabulary arrives three ways: typing a new one at the input line, "copy verbs and qualifiers from guide X" when creating a guide, and import (verbs and qualifier labels travel with every export and go through the same mapping step as tags). Copying gives the words new ids, skips any whose label or inverse already exists in the target, and leaves default tags behind, since tags belong to a guide.

## 5. Browse views

- **Guides:** card view (cover, name, description, tags) and table view. Filter by name substring and by global tags.
- **Categories:** the tag tree of the current guide. Selecting a tag lists notes under it with ancestor inclusion. Filters apply live as the user types: free text runs FTS5 prefix queries against title, aliases, and body; tag chips narrow further; a "stubs only" toggle surfaces undescribed things.
- **Note page:** rendered body, gallery, tags, and the relation list grouped by verb with both directions shown as in section 4.
- **Trash:** per guide. Restore or purge individual notes; empty trash.

## 6. Board

- Scope: current guide. Toggles for external edges and mentions.
- Filters, all live: tag chips (ancestor-inclusive), free text (FTS5), and topics: a set of selected notes plus a hop depth. The board shows notes that pass the filter and every edge whose endpoints are both visible.
- Nodes are cards: title, cover thumbnail if the note has gallery media, tag chips. Edges are labeled with the verb; qualifiers render as a second line on the label. Mentions are dashed.
- Layout: SVG with pan and zoom. Unplaced nodes get force-directed positions on open. Dragging persists `(x, y)` to `board_node` on drag end. A guide can have several named boards, each with its own layout and saved filters. Board names are unique per guide, case-insensitively. Filters are stored as JSON and read back into a typed struct by `core`; a position may be saved for a note from another guide, since the external toggle can show one.
- Canvas rendering is a later optimization if SVG falls over past a few hundred nodes.

## 7. Export

Export produces a `.zip`. Scopes: vault, guide, tag subtree, single note. Every archive is self-contained: it carries the verbs, qualifier labels, and tags that its relations and notes reference, and the media its notes use. Trashed notes are not exported.

```
manifest.json                     format_version, app_version, exported_at, scope, root ids
guides/<slug>/
  guide.json                      id, name, description, cover (media path), tags (global tag paths)
  tags.json                       tree: [{id, name, color, children: [...]}]
  verbs.json                      [{id, label, inverse_label, symmetric, subject_default_tag, object_default_tag}]
  qualifiers.json                 [{id, label, inverse_label}]
  notes/<slug>.md                 front matter + body
  relations.jsonl                 one edge per line
  boards/<slug>.json              layout and filters
media/<sha256>.<ext>              raw files, no base64
```

Slugs are lowercase ASCII, `[a-z0-9-]`, capped at 64 characters, with a numeric suffix on collision. Other scripts are transliterated (`Café` to `cafe`, Cyrillic and CJK to Latin approximations) so file names stay readable; a title with nothing printable falls back to `note-<id prefix>`. Two notes whose titles differ only by case or accents get distinct slugs. This keeps archives usable on case-insensitive and case-sensitive filesystems alike and inside Windows path limits. The archive writer uses ZIP64 so exports over 4 GB work on FAT32 thumb drives. Entries are written in a stable order (sorted by id) with a fixed timestamp, so two exports of the same vault differ only in the manifest's `exported_at`. Text entries are deflated; media entries are stored uncompressed, since those formats are already compressed.

A relation is written in the guide that owns its verb; an edge whose verb belongs to a guide outside the scope is not in the archive. Full guide and vault exports carry the whole tag tree, vocabulary, and boards. A tag or note export carries only the tags its notes use (with their ancestors) and the vocabulary its relations use, and no boards, since a board's notes may lie outside the scope. Trashed notes, and edges and qualifiers that touch them, are left out.

Note front matter:

```
---
id: 018f...
title: Stone Block
aliases: [Stone]
tags: [Blocks/Natural]
stub: false
gallery: [media/9a3f....png]
created: 2026-11-07T19:02:11Z
updated: 2026-11-07T19:40:03Z
---
```

Relation line, ids for machines and titles for people:

```
{"id":"018f...","subject":{"id":"...","title":"Stone Block"},"verb":{"id":"...","label":"DROPS"},"object":{"id":"...","title":"Cobblestone Block"},"qualifiers":[{"label":"mined with","note":{"id":"...","title":"Pickaxe"}}]}
```

Endpoints outside the exported scope are written as `{"external": true, "guide": "<name>", "title": "..."}` with the id when known; qualifier note values use the same shape. Qualifier lines carry `label_id` beside the label so import can match by id before falling back to the label.

Media is stored raw inside the zip. Base64 would be a third larger and harder to open; a zip already handles binary.

## 8. Import

- Source: any archive at any scope. Target: a new guide, an existing guide, or a tag within a guide (imported notes get that tag added).
- Guide names are unique per vault, case-insensitively, so an archive's guide matches at most one local guide. When it matches, the target picker defaults to "Import into existing <name>" with "Import as new" as the alternative; when nothing matches it defaults to "Import as new". "Import as new" into a vault that already holds that name creates the guide as `<name> (imported)`, mirroring the note-title rule.
- An archive whose `format_version` is newer than the app understands is refused with a message to update.
- **Mapping step, before anything is written.** For each incoming tag path, verb, and qualifier label:
  - exact match after normalization (case, whitespace, trailing `s`) maps automatically
  - otherwise the top three local candidates by string similarity are offered, with "create new" and, for tags, "create under ..." as alternatives. The metric is the `similarity::Similarity` trait; the default is a normalized edit distance with no dependency, chosen until real archives show whether a prefix-weighted metric ranks better
  - the user can accept all suggestions at once or go through them
- Note identity: same id updates in place if the incoming `updated` is newer. Different id with a title collision keeps both and renames the incoming note `Title (imported)`. Both cases appear in the import report. Imported notes keep their ids and timestamps; tags, vocabulary, and boards are matched by name and get new ids. A board is identified by its name within the target guide, so importing again replaces its saved filters and positions instead of adding a copy. Board filters travel as tag paths.
- Wikilinks in imported bodies resolve first against the archive's own notes (by the title at export time), then against the target guide's titles and aliases.
- Relations whose external endpoint resolves (by id, or by guide name and title) are created. Unresolved ones are listed in the report with an option to create stubs for them. With stubs requested, qualifier values that point at missing notes become stubs too; without, they are kept as the title text and counted in the report. Every guide's notes are imported before any relation, so edges between two archive guides resolve.
- A word that collides on import reuses the local word when the label matches; when only its inverse collides, it is created without the inverse. Both are rare, since the mapping step has already matched labels.
- The whole import is one transaction. Media files are extracted to a staging folder beside `media/`, checked against their hashes, and moved into place only as the transaction commits; a failure removes what was moved. Any failure leaves the vault and its media untouched.

## 9. Update check

A menu item, "Check for updates", fetches `latest.json` from the release URL, compares semver with the running version, and shows a dialog with the version and a link. It never downloads, never runs on a schedule, and there is no setting to make it automatic. The Tauri HTTP capability is scoped to that one URL.

## 10. Distribution

- Windows: a bare `.exe` for machines with the WebView2 runtime (all Windows 11, nearly all Windows 10), and a separate zip bundling a fixed WebView2 runtime for machines without it.
- macOS: a zipped `.app`. Unsigned; the README documents right-click, Open for the first launch. Signing and notarization need an Apple Developer account and can be added later without changing anything else.
- Linux: AppImage.
- macOS and Linux builds come from a GitHub Actions workflow with `tauri-action`, since neither can be built from Windows.

## 11. Open questions

- Whether qualifier values should support dates as a distinct type.
- Whether image ingest uses lossless WebP (pure-Rust encoder from the `image` project, pixels unchanged, roughly a quarter smaller than PNG) or lossy WebP (`libwebp` through a C binding, several times smaller, pixels changed). Decide when phase 8 starts.
- Whether export should offer a hop-depth scope (a note plus everything within N edges) with a preview of what it will include (phase 9). The archive format needs no change for it.
- Whether import into an existing guide should ask per field (cover, description, guide tags) before overwriting (phase 9). Today the cover is set only when the local guide has none and the description is left alone.

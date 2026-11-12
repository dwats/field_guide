# Design

Field Guide is a local-only wiki you build up while learning a subject. A vault holds guides (one per game, course, or subject). A guide holds entries, a tag tree, and a vocabulary of verbs. Entries are markdown with inline media. Entries relate to each other through typed edges: `Oak Tree DROPS Oak Log`. Browsing is by tag tree with live filtering. The board view is an alternative way to look at the same entries and edges, on a pannable surface, for people who think in relationship maps.

The working store is one SQLite file per vault. The export format is a zip of markdown and JSON that a person can read without the app.

## 1. Architecture

- **Shell:** Tauri 2. One process; the Rust binary owns the window and opens the OS webview (WebView2, WKWebView, WebKitGTK). Frontend assets are compiled into the binary and served over `tauri://`. No local server.
- **Core:** Rust crate with no Tauri dependency. Owns the schema, relation semantics, markdown parsing, search, export/import, and backups. Exposes plain structs. All business rules and their tests live here.
- **UI:** Svelte 5 + TypeScript. Editor is CodeMirror 6. Markdown preview HTML comes from `core` so rendering is identical in the app and in exported HTML, and so wikilink and media resolution is tested once.
- **IPC:** `#[tauri::command]` functions in `app/` forwarding to `core`. Serde JSON both ways.
- **Capabilities:** the frontend gets the commands it needs plus the asset protocol scoped to the vault directory. The only HTTP permission is the update-check URL.

### Why SQLite as the working store, not loose markdown files

Relations, qualifiers, tag trees, aliases, and board layouts are structured data. Keeping them in files next to markdown means either sidecar files per entry or front matter that grows past what a person wants to hand-edit. SQLite gives transactions, FTS5, and a single file to copy. Human readability is delivered by export, which is a first-class feature, not an afterthought.

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
- **Rolling backups.** On the first open of each day (UTC), copy `vault.db` into `backups/` with SQLite's online backup API. Files are named `vault-<date>_<time>-<ms>.db` in UTC so name order is time order. Keep the five most recent. Settings lists them with a "restore" action that swaps the current file after taking one more backup of it. Restore first checks that the file is a SQLite database holding Field Guide's tables at this build's schema or older and passes `quick_check`, then backs up the current database, closes the vault, copies the backup beside `vault.db`, removes stale `-wal` and `-shm` files, renames the copy into place, and reopens. A refused file changes nothing.
- **Atomic writes.** Media, exports, backups, and `config.toml` are written to a temporary name in the same directory and renamed into place. A yanked drive leaves either the old file or the new one, never a partial.
- **Version gates.** The DB records its migration version and every archive records `format_version`. An older build opening a newer vault, or importing a newer archive, stops with "update Field Guide" instead of guessing.

## 3. Data model

Schema sketch; the migrations are authoritative.

```
guide             id, name, description, cover_media_id, created_at, updated_at
tag               id, scope_guide_id (NULL = global, used for guide tags), parent_id, name, name_norm, color
                  unique (scope_guide_id, parent_id, name_norm)
guide_tag         guide_id, tag_id            (global-scope tags only)
entry              id, guide_id, title, title_norm, body_md, is_stub, deleted_at, created_at, updated_at
                  unique (guide_id, title_norm)
entry_alias        entry_id, guide_id, alias, alias_norm   unique (guide_id, alias_norm); triggers keep aliases and titles in one namespace
entry_tag          entry_id, tag_id
media             id, sha256, ext, mime, bytes, width, height, duration_ms, created_at
entry_media        entry_id, media_id, sort      (gallery membership; inline use is derived from body)
verb              id, guide_id, label, label_norm, inverse_label (NULL ok), inverse_norm, symmetric, subject_default_tag_id, object_default_tag_id
                  unique (guide_id, label_norm); labels and inverses share one pool per guide
qualifier_label   id, guide_id, label, label_norm, inverse_label (NULL ok), inverse_norm   same pool rule, in a pool of its own
relation          id, subject_entry_id, verb_id, object_entry_id, created_at
                  unique (subject_entry_id, verb_id, object_entry_id)
relation_qualifier id, relation_id, qualifier_label_id, value_entry_id | value_text | value_num, sort
mention           entry_id, target_norm, target_entry_id (NULL while unresolved)   cache of [[wikilinks]], one row per distinct link text
board             id, guide_id, name, name_norm, filters_json, created_at, updated_at   unique (guide_id, name_norm)
board_node        board_id, entry_id, x, y          (entries from other guides allowed, for the external toggle)
entries_fts         FTS5 over title, aliases, body; tokenizer unicode61 with remove_diacritics
```

Names are compared through the `*_norm` columns: trimmed, Unicode-lowercased copies that core maintains. SQLite's `lower()` folds only ASCII, so it is not used for uniqueness. Guide names are unique across the vault the same way.

### Tags

- Per-guide tree. An entry tagged `Consumables/Potions/Healing` is included when browsing `Potions` or `Consumables` (ancestor inclusion). Display separator is `/`.
- Global-scope tags (guide_id NULL) are the ones applied to guides for the top-level browse view.
- Rename cascades by id. Move is a parent change. Merge retags, moves the source's children under the target, and deletes the source. Deleting a tag with children reparents the children to the deleted tag's parent and untags its entries. In both cases a child whose name is already taken at its new level is merged into the tag that holds it, so nothing is lost and sibling names stay unique.

### Entries

- Title unique per guide, case-insensitive. Aliases share that namespace. `[[Oak Log]]` resolves against titles and aliases. Titles and aliases are one line, trimmed, and may not contain `[[`, `]]`, or `|`, which would break wikilink syntax.
- A stub is an entry with `is_stub = 1` and an empty body. Every entry created with an empty body starts as one, whatever created it: the relation builder, New entry, the palette, or a click on an unresolved link (decided 2026-11-11, so "Stubs only" lists everything nobody has described yet). A body of only white space counts as empty. Any body edit clears the flag; emptying the body later does not set it again. Import keeps the flag the archive carries.
- Body is CommonMark + GFM tables, strikethrough, task lists, plus:
  - `[[Title]]` and `[[Title|shown text]]` wikilinks
  - `![alt](media/<sha256>.<ext>)` for images and video; `.webm` renders as `<video controls>`
- The same body text is stored in the DB and written to the export, unchanged. `core` renders the body to HTML (`markdown::render`); the UI rewrites `media/...` sources to the asset-protocol URL in the rendered DOM, at render time only.
- Raw HTML in a body is shown as text, never passed through. Bodies arrive from other people's archives, and the webview has no need for inline markup.
- Rendered wikilinks carry the resolved entry id (`data-entry-id`) or the unresolved target text (`data-target`); the UI decides what a click does. The renderer and the mention scanner share one link parser, so a link inside a code span renders as code but still gets a mention row; that divergence is accepted.

### Links

Wikilinks are human-readable in the markdown and id-based underneath. On save, each distinct `[[...]]` target in an entry gets a `mention` row keyed by its normalized text, carrying the id it resolves to against titles and aliases in the guide, or NULL while nothing has that name yet. Links are often written before their targets exist, so when an entry gains a name (created, renamed, aliased) the rows waiting for that name in its guide resolve, and when a name goes away (alias removed, rename undone, entry purged) the rows that reached the entry through it go back to unresolved. Trashed targets stay resolved so restore and rename keep working. From then on the id is the link; the text is its serialization. Relations (section 4) use ids directly and never go through titles.

Renaming an entry does two things in one transaction:

1. Rewrites `[[Old Title]]` and `[[Old Title|text]]` to the new title in every entry that the `mention` table says references it. Only the link target changes; `|text` is kept.
2. Adds the old title as an alias.

The rewrite keeps text and links consistent inside the vault. The alias catches links that arrive later from someone else's archive and still use the old name. Rename is undoable from the entry page for the rest of the session. Undo rewrites the current bodies back and removes the alias rather than restoring snapshots, so edits made after the rename survive; only an entry's most recent rename can be undone. A rename that changes only the casing rewrites links but adds no alias, since the old title would collide with the new one.

### Trash

Deleting an entry sets `deleted_at`. A trashed entry is hidden from browse, search, wikilink resolution, the board, and the relation lists of other entries, but its edges, qualifiers, tags, and gallery rows are kept. Restore clears `deleted_at` and everything reappears. Purge (from the Trash view, or "empty trash") deletes the row and cascades: edges where it is an endpoint, qualifiers where it is the value, the mentions it made, and gallery rows. Links in other entries that pointed at it keep their text and become unresolved. A title stays reserved while its entry is in the trash; it frees up on purge. Trashed entries are excluded from export.

### Media

- Dropped or pasted files are hashed, copied to `vault/media/<sha256>.<ext>`, deduplicated by hash, and a `media` row is created. Clipboard paste of an image is a first-class path, since Print Screen then Ctrl+V is the common capture flow for game entries.
- Images are converted to lossless WebP on ingest (decided 2026-11-08) so a vault of screenshots stays small without changing pixels; the hash is taken over the stored WebP bytes, so the same source image still deduplicates. The EXIF orientation is applied on ingest, so a phone photo is stored the way it was seen. WebP has no 16-bit mode, so a 16-bit PNG or TIFF is stored at 8 bits per channel. A setting for lossy WebP may follow; it would add a C dependency (libwebp), so it is a separate decision. Video ingest is not scheduled (`docs/plan.md`, Later); the renderer keeps its `.webm` handling so an archive that carries a clip still plays.
- Dropped files reach the app through the window's native drag-drop events as paths, and core reads them from disk, so a large file never passes through the webview. Pasted bytes travel as the raw body of an IPC request, which needs the webview's content security policy to allow `ipc:` and `http://ipc.localhost` (found 2026-11-08): without them the webview blocks the IPC fetch and Tauri falls back to its message channel, which turns bytes into a JSON array and the raw-body command refuses it. These are the webview's channel to the shell, not network access. Drag and drop inside the page (nesting tags) uses pointer events, because Tauri's native drop and HTML drag and drop cannot both be on under WebView2.
- Images live in the body as `![alt](media/...)` references. A drop or paste onto the editor inserts the reference where it lands (or at the caret); onto the rest of the entry page it is appended to the end of the body (decided 2026-11-08). An image alone on its line renders as a figure: a bordered thumbnail with the alt text as its caption, floated right so text wraps beside it; the image's title field moves it (`"left"`, `"center"`, `"full"`), and an image inside a sentence stays inline (decided 2026-11-08). Clicking an image opens a viewer over the page, after Wikipedia's: the body's images one at a time, previous and next, a counter, the caption from the alt text, and the file's details (decided 2026-11-08). The `entry_media` rows order the gallery; the first is the entry's primary image, which boards show. The first stored image a body references becomes the primary image on save when the gallery is empty (decided 2026-11-09), so a card has a picture without a further step; "Make primary image" in the viewer or the album overrides it. The entry page's Images button opens the album: every image of the entry as thumbnails, the primary marked, a click opening the viewer there (decided 2026-11-09).
- Formats (decided 2026-11-08): PNG, JPEG, BMP, TIFF, and still GIFs become lossless WebP on ingest; animated GIFs stay GIF (the pure Rust encoder writes still images only; animated WebP needs libwebp, the same later decision as lossy); SVG and WebP are stored as they are. The guide cover is set by dropping or pasting an image onto the guide page, which opens a crop box (drag to pan, a slider to zoom) choosing what the 2:3 poster shows; the crop is stored with the guide in thousandths of the picture and travels in the export (decided 2026-11-08). Guide cards are 2:3 posters with the title in one line below; the table view keeps its text columns. Controls that are only an action (New, More, the view toggle, panel and sidebar folding) are icons from the Lucide set with a tooltip and an accessible name (decided 2026-11-08).
- Media is vault-scoped, not guide-scoped, so the same screenshot can serve two guides. Export only includes media referenced by the exported entries.
- Media is never deleted automatically. "Clean up unused media" in Settings lists files that nothing refers to: no entry body (live or trashed), board note body, gallery, cover, or board image; it shows them with sizes and a total, and deletes them on confirmation. A non-modal notice appears when unreferenced media exceeds 250 MB (adjustable) or free space on the vault's drive drops below 1 GB.

## 4. Relations

This is the part that has to feel right, so the rules are spelled out.

### Definitions

- **Verb:** per-guide, user-defined. `label` (`DROPS`), optional `inverse_label` (`DROPPED BY`), `symmetric` flag (`ADJACENT TO`, inverse = self). Optional default tags for the subject and object side, used only as a suggestion when creating stubs.
- **Relation:** one stored edge `(subject, verb, object)` plus an ordered list of qualifiers. Direction is canonical: subject is the verb's grammatical subject.
- **Qualifier:** `label + value` attached to an edge. Labels are a per-guide pool with their own optional inverse (`mined with` / `mines`); labels and inverses must not collide within that pool, which is separate from the verb pool. Values are an entry reference, text, or number. The same label may appear more than once on one edge.

### Reading an edge from each side

Stored: `Stone Block DROPS Cobblestone Block`, qualifiers `[mined with: Pickaxe]`.

| Page | Shows |
|---|---|
| Stone Block | `DROPS Cobblestone Block · mined with Pickaxe` |
| Cobblestone Block | `DROPPED BY Stone Block · mined with Pickaxe` |
| Pickaxe | `MINES Stone Block → Cobblestone Block` |

Without an inverse on the verb, the object side shows `← DROPS Stone Block`. Without an inverse on the qualifier label, Pickaxe shows `mined with · Stone Block DROPS Cobblestone Block` under a "Referenced by" section. Inverses are display rules applied at read time; nothing is stored twice.

Whatever stands in the verb slot renders uppercase, as in these examples, so a verb typed as `drops` and a qualifier inverse typed as `mines` both read as verbs; qualifier labels in qualifier position render as typed. Further qualifiers on the edge follow the line, each separated by `·`. `core` builds these lines (`relations_for_entry`) along with the structured parts, so the UI and the exported HTML render them the same way.

### Entry flow

One input line on any entry page, staged with autocomplete:

1. **Verb.** Suggestions include labels and inverse labels from this guide. Picking an inverse label flips the direction before storing. Typing a new label prompts for an inverse (optional) and symmetric (default off).
2. **Target.** Suggestions from titles and aliases in this guide, then other guides marked as external. Enter on no match opens the stub prompt: title prefilled, suggested tag prefilled from the verb's default for that side, both editable, tag not required.
3. **Qualifiers.** `+` starts one. Label autocompletes from the pool; a new label prompts for an optional inverse. Value autocompletes against entries; anything that does not resolve is stored as text, or as a number if it parses as one.

`core` exposes `lookup_entry(guide, text)`, `suggest_names(guide, prefix)` for the autocomplete, and `create_relation(CreateRelation)` where the request carries the resolved verb id, direction, target id or stub spec, and qualifiers. The UI does staging and prompts; `core` does resolution, validation, and writes. Resolution rules are tested in `core`.

A new verb or qualifier label typed at the line is created when the relation is saved, not when it is typed, so cancelling the line leaves no vocabulary behind. Deleting an edge does not ask (decided 2026-11-11): `delete_relation` returns the removed edge with every qualifier, the notice offers Undo, and `restore_relation` puts it back with the same id, direction, creation time, and qualifier order; it merges into the same edge if that was entered again meanwhile and is refused if an end was purged. The Delete key on a selected row does the same. On an existing edge, qualifiers are edited in place, and the value field offers the relation line's suggestions (matching entries, use as text, create as a new entry); changing the verb or an endpoint is delete and re-enter, so an edge's identity never changes underneath the user. Display lines come from `core` as text plus segments, so the UI can link titles without re-deriving the wording.

### Rules

- Creating `(subject, verb, object)` that already exists merges qualifiers into the existing edge. No duplicates: a qualifier identical in label and value to one already on the edge is not added again, and new ones keep their order after the existing ones.
- A relation whose verb is symmetric stores whichever direction it was entered from; both pages render the same label, and the reverse edge counts as the same edge when merging.
- Cross-guide edges are allowed. The verb belongs to the guide of the entry the edge was created from. Both entries list the edge; views default to same-guide and have a "show external" toggle.
- Trashing an entry hides its edges and the qualifiers that point at it; restoring brings them back. Purging deletes them. A trashed entry cannot be the origin, target, or qualifier value of a new edge.
- Renaming a verb or qualifier label is a label change on the definition. Deleting one that is in use is blocked; the UI offers merge-into-another. Merging verbs moves the edges, and an edge that would duplicate one the target verb already has merges its qualifiers into that edge instead. Merging qualifier labels relabels the qualifiers and drops exact duplicates that result.
- Mentions (`[[wikilinks]]` in body text) are not relations. They are extracted on save into the `mention` table and shown as a thin, hideable "mentions" edge. They never appear in the relation list on an entry page.

### Starter vocabulary

None. A new guide has zero verbs and zero qualifier labels. Vocabulary arrives three ways: typing a new one at the input line, "copy verbs and qualifiers from guide X" when creating a guide, and import (verbs and qualifier labels travel with every export and go through the same mapping step as tags). Copying gives the words new ids, skips any whose label or inverse already exists in the target, and leaves default tags behind, since tags belong to a guide.

## 5. Browse views

- **Guides:** poster view (2:3 covers in rows from the left, the title below, a placeholder without a cover) and table view. Filter by name substring and by global tags.
- **Categories:** the tag tree of the current guide. Selecting a tag lists entries under it with ancestor inclusion. Filters apply live as the user types: free text runs FTS5 prefix queries against title, aliases, and body; tag chips narrow further; a "stubs only" toggle surfaces undescribed things.
- **Entry page:** rendered body, gallery, tags, and the relation list grouped by verb with both directions shown as in section 4.
- **Trash:** per guide. Restore or purge individual entries; empty trash.

### Interface principles

Decided with the owner on 2026-11-08, first in an interview and then over the UX review in `docs/ux-review.md`. Where a choice here differs from the review, the choice here wins.

- Audience: the owner first, friends through export second. Nothing waits on feedback from others.
- Shell: one top bar with breadcrumbs, a left sidebar, the content column, and a right panel. Column widths never change when the mode or the view changes; the sidebar and the right panel can each be collapsed by the user, and the choice is remembered.
- Sidebar inside a guide, two regions: search and the tag tree above, the entries under the selected tag below, filtered live by the search; an entry is reachable from the sidebar without leaving the page, the tree stays compact, and the entry list has one fixed home. Breadcrumbs sit above the content.
- Sidebar model (decided 2026-11-08 after comparing Bear, VS Code, Obsidian, Zotero, Apple Notes, and Notion): tags behave like Bear's, so an entry appears under every tag it has and the list covers the selected tag with its subtree; the tree has three fixed nodes, "All entries" at the top and "Untagged" and "Trash" at the bottom; each tag node shows its entry count including the subtree. The two regions are stacked sections in VS Code's style: a header each, each collapsible, a splitter between them whose position is remembered. The search narrows the tree to matching tags with their ancestors expanded and the list to matching entries in the selected scope; clearing it restores the expanded state. Opening an entry highlights it in the list and expands the tree to its first tag. A footer under the regions links to the Tag editor, and to Boards from Phase 7. The tree's keyboard model: Up and Down move, Home and End jump, Right expands, Left collapses or goes to the parent, typing a letter goes to the next row that starts with it (shipped with Phase 8).
- Right panel on an entry page: two tabs, "Contents" and "Relations", with Contents selected on arrival the first time; after that the panel keeps the last chosen tab and the open folds while the app runs (decided 2026-11-11). The relation line sits at the top of the Relations tab; "Mentioned by" and "Unresolved links" fold under the list, closed by default and absent when empty (decided 2026-11-08). While editing in Split the panel collapses; a setting keeps it open instead, and the rail's button opens it for that edit only.
- Line breaks: a single newline renders as a line break wherever CommonMark would soften it to a space (paragraphs, quotes, list items), as in Obsidian and GitHub comments (decided 2026-11-08). Quotes are one `>` per line; `>>>` nests three levels.
- Markdown dialect (decided 2026-11-08): markdown syntax first, raw HTML stays text; HTML support is considered only for a feature that markdown cannot express. In the dialect today: CommonMark, tables, strikethrough, task lists, horizontal rules, headings with anchors (`[text](#heading-id)` scrolls within the body), footnotes (`[^1]`), alerts in GitHub's form (`> [!NOTE]`, also TIP, IMPORTANT, WARNING, CAUTION), spoilers in Discord's form (`||hidden||`, revealed by a click or Enter, closed at the end of the block), `[[wikilinks]]`, and media. Planned, syntax to be confirmed: collapsible sections, highlight, superscript, subscript, icons, and LaTeX (see plan.md, Later).
- Heading anchors: core's renderer gives every heading an id from its text (`where-found`, repeats numbered), listed in the render result, so the Contents tab and exported HTML share the same anchors (decided 2026-11-08).
- Contents: built from the entry's headings; shown whenever the body has at least one heading; a display setting hides it always, off by default.
- Lists: selecting a row enables one fixed action row per list. No control appears or disappears on hover.
- Aliases: a muted line under the tags, present only when the entry has aliases, edited from the More menu.
- Density: a setting with a compact and a roomy preset.
- Theme (decided 2026-11-11): a setting with three choices, System (default), Light, and Dark, stored as `ui.theme` in `config.toml`. The UI sets `data-theme` on `<html>` for Light and Dark and removes it for System, so `color-scheme` follows. The shell sets the window theme from the same value when it creates the window and on every save, so the title bar, WebView2's preferred scheme, and the background shown before the first paint agree; an inline script is not possible under the CSP and is not needed. Colors are tokens in `ui/src/app.css`, each a `light-dark()` pair; components use tokens or a `light-dark()` pair, never a single color, except the image viewer's stage, which is black in both schemes. The viewer's bottom bar follows the theme. Transparent SVGs get no backing in dark.
- Dim images in dark (decided 2026-11-11): a setting, off by default, stored as `ui.dim_images`. While the dark scheme is in effect it lowers the brightness of the user's pictures in entries, galleries, covers, and boards. The image viewer is never dimmed, since that is where a picture is judged.
- Borders (decided 2026-11-11): three strengths in both schemes. `--border` is for dividers; `--control-border` edges buttons; `--field-border` edges text fields, selects, and switch tracks at 3:1 or more against the page.
- Scales (decided 2026-11-11): type sizes, spacing, and radii are tokens in `ui/src/app.css` (`--text-*`, `--space-*`, `--radius-*`), and component styles use them instead of typed values. Density changes the root font size and the space tokens, so spacing shrinks in Compact everywhere. Two exceptions keep typed values: sizes inside rendered markdown that must follow the text around them (inline code, the quote mark), and the board canvas, whose pixel sizes feed the card geometry in `ui/src/lib/board.ts`.
- Raised layers: menus, the palette, and dialogs sit on `--raised` with a shadow from the elevation tokens and a 1-pixel border, because a shadow alone does not show on a dark page.
- Editing: Read and Edit modes stay; Edit offers a toggle between the split live preview and a single pane that renders in place.
- Input: keyboard first, with a command palette for search, jump, and new entry on the shortcut each platform expects (Ctrl on Windows and Linux, Cmd on macOS); every action also reachable by mouse. The map (review 4.1) lives in `ui/src/lib/keys.ts`: Mod+K palette, Mod+N new entry (new guide on the guides list), Mod+Shift+N new guide, Mod+F search this guide, Mod+E Read or Edit, Mod+R the relation line, Mod+T add a tag, Mod+, Settings, Alt+Left and Alt+Right history, F2 rename; Escape belongs to the focused thing. The palette groups Entries, Commands with their shortcuts, and Guides, ends with a row that creates the typed entry, and takes `>` to limit itself to commands (decided 2026-11-08).
- Sidebar at every level: inside a guide as above; on the guides list the name filter and the global tags with guide counts; in Settings the sections with the one in view marked (decided 2026-11-08).
- Tag editor: the tree on the left and a detail pane on the right with name, parent, color, counts, Add child tag, Merge into..., and Delete tag; no control appears on hover; Up and Down, Left and Right, F2, and Delete work in the tree; drag to nest stays for the mouse (decided 2026-11-08).
- Attention: minimal chrome, no animation, fixed placement, confirmations only for destructive actions. Revisit after use.
- Recovery (decided 2026-11-11): a delete that can be reversed does not ask; it shows a notice with Undo. A step that cannot be reversed asks, names what is lost, and starts with focus on Cancel. Merging tags is such a step: in Merge into..., Enter takes only an exact path or a row reached with the arrow keys, and the confirm names both tags and counts the entries and child tags that move. A click on a dialog's backdrop closes it only when nothing in it changed. In Trash, Enter restores the focused row and Delete asks to delete it for good.
- Web links (decided 2026-11-11): a click on an `http://` or `https://` link in a body or a board note opens a confirm that shows the whole address, with focus on Cancel; "Open in browser" asks the operating system to open it. Field Guide makes no request itself, so the no-network rule holds. The scheme is checked in core (`entry::web_link`), the address is passed as one argument and never as a command line, and the opener plugin is called from Rust only: it is not registered, so the webview gains no command and no permission. Any other scheme does nothing except a notice.
- Editor line numbers (decided 2026-11-11): off by default; a switch in Settings, Editor (`ui.editor_line_numbers`), followed without a reload.
- Page headers (decided 2026-11-11): the title has its own line, wraps, and is never cut off; the controls sit on a toolbar line below it, and a save state keeps a fixed width so nothing shifts between Read and Edit. The entry title is serif (`--font-serif`), `--text-3xl`, weight 400, with a rule under the title block; body headings step down from it, so none outranks the title.
- Notices (decided 2026-11-11): one notice at a time floats over the bottom edge of the content column and moves nothing. It stops short of the right panel, keeps "1 of n", Next, its action, and dismiss, and never times out.
- Focus (decided 2026-11-11): after a route change focus goes to the page heading, unless a command, a dialog, or the page placed it. The palette does not hand focus back to its opener. Each sidebar list is one Tab stop with arrow keys inside. One row is selected per tree, on `--accent-soft` with an accent bar; the focus ring covers the whole row.
- Sidebar search (decided 2026-11-11): with a tag as the scope and text in the search, a fixed row under the list says how many more matches All entries holds; a click or Enter widens the scope. The Tags and Entries regions fold from their heads, and stay folded while the app runs.
- Empty states: an icon, one sentence, one muted line, and the action, centered in the region. An action row is hidden while its list is empty (decided 2026-11-11).
- Guide page (decided 2026-11-11): an image pasted on the guide page, or dropped anywhere but the cover, opens New entry with the image attached. The cover is set by a drop on the cover slot or from More, "Set cover...". The table view has a selected row: Up, Down, Home, and End move it, and Enter opens the entry.
- Rendered markdown (2026-11-11): links carry no underline until hover or focus; task lists have no bullet and a drawn box; "stub" is a status chip (a dashed pill), not a tag chip; alert titles use `--alert-*` tokens that pass contrast in both schemes.
- Link suggestions (decided 2026-11-11): typing `[[` in the editor lists this guide's titles and aliases by prefix, one row per entry, with the matched alias or "stub" muted; with nothing typed it lists the first titles in order. A last row, "Link to a new entry "X"", keeps typed text that matches nothing exactly. Up and Down move, Enter or Tab accepts, Escape closes. Accepting writes the title and the closing `]]` without doubling brackets and keeps any `|shown text`. `|` or `]` closes the list. It never opens inside code, and it has no delay. Board notes do not have it yet.
- Vocabulary page (decided 2026-11-11): `#/guide/<id>/vocabulary`, reached from the sidebar footer beside Tag editor and from the palette. Two tabs, Verbs and Qualifier labels, each a list and a detail pane like the Tag editor: label, inverse, symmetric, default tags, and the count of uses, with Rename (F2), Merge into..., and Delete. Counts and merge previews come from core (`verb_usage`, `qualifier_label_usage`, `preview_merge_verbs`, `preview_merge_qualifier_labels`; a preview runs the real merge in a savepoint and rolls it back, so its numbers are exact). A merge cannot be undone: Enter takes only an explicit choice, and the confirm names both words and the counts. Delete is refused while anything uses the word, relations with a trashed end included, and the dialog offers Merge into... instead.
- Controls (decided 2026-11-11): a pressed toggle is `aria-pressed` on `--accent-soft` with accent text and edge. A segmented control is a `--chip-bg` track, and its chosen segment is raised and lighter than the track in both schemes. Settings switches draw a switch; a radio choice is a tile, one Tab stop with arrow keys. Icons are Lucide at stroke 1.75, sized only by the `--icon*` tokens through CSS; they are muted, and accent only when pressed or primary. Color choices, tag colors included, are swatches. Every sidebar list, the footer included, is one Tab stop. The editor shows the middle of a media hash as an ellipsis except on the line being edited; the stored text never changes.
- Messages from core are shown in sentence case; the UI capitalizes the first letter where it turns an error into a message.
- Settings: appearance choices are toggles, each with a small picture of what it changes, so a choice can be made without trying it first. The theme pictures draw a small window in each scheme's real tokens by setting `color-scheme` on the picture.

## 6. Board

A board is a cork board the user fills by hand (decided 2026-11-08; it replaces the filter-driven board of the first design). Scope: one guide. A guide can have several named boards; names are unique per guide, case-insensitively.

- Membership is manual. Entries are added one at a time or in bulk through a picker that offers the guide's search and tag filters, the same filters the browse views use, and removed from the board without touching the entry. `board_node` holds membership and position; `filters_json` holds the picker's last filters and the display toggles, never a membership rule.
- A card is an instant photo: the entry's primary image as the picture, the title as a caption along the bottom edge, proportions following the image rather than a fixed ratio. An entry without an image is an index card: paper with the title and one red rule under it (decided 2026-11-11). Picture cards and board images share one frame treatment. The primary image is the first gallery item; "Make primary image" in the viewer sets it.
- Edges: every relation whose endpoints are both on the board is drawn as members arrive, labeled with the verb, qualifiers on a second line. Mentions are a dashed, hideable layer. Edges to entries in other guides show when the external toggle is on, and a card from another guide may sit on the board for that reason.
- Layout: a pan-and-zoom canvas. The wheel zooms around the pointer, dragging the background pans, Fit shows every card, and the zoom readout returns to 100%. Dragging a card writes `(x, y)` when the pointer lifts; arrow keys nudge a focused card and the write follows 150 ms after the last press. Enter or Space on a card opens the entry, Delete takes it off the board and moves focus to the canvas; with the canvas itself focused, arrows pan and plus and minus zoom. The panel's fold is the same remembered choice as the entry page's right panel.
- Placement (decided 2026-11-08): a card added from the picker lands on the middle of the window, or on the nearest free cell of a grid around it; several added at once fill the cells outward. The picker has no drop point of its own, and the middle of the window is where the user is looking. The grid leaves 120 pixels across and 140 down between cards, room for a line's verb and qualifiers to be read (2026-11-11). A bulk add shows a notice with Undo that takes off exactly the cards it added. In the picker, the name of an entry already on the board selects its card and pans it into view.
- Toggles: the mention layer starts shown and other guides hidden; both choices are saved with the board. Hiding other guides hides their cards and every edge that reaches them.
- A member in the trash leaves the board with its edges until it is restored; its position is kept. An entry in the trash cannot be added to a board (the call is refused, as a new relation to it is), while one already placed can still have its position written, by a move or by an import. Purging an entry drops it from every board. Removing a card offers Undo; deleting a board asks first, since the entries stay and there is nothing else to reverse. A board file in an archive names live members only; a member in the trash is left out like any trashed entry (section 7).
- Rendering (decided 2026-11-08): the pan-and-zoom layer is a CSS transform. Cards are HTML, so titles wrap and pictures load like any other image; the edges are one SVG under them, each a line clipped to the two card borders, an arrowhead unless the verb is symmetric. A line's label is horizontal, in a small pill at the line's midpoint, the verb first and the qualifiers on a second line, which is dropped when the line is too short for it (decided 2026-11-11; a label along the line was clipped and hard to read at steep angles). Strings label the same way. Relation lines are drawn in a neutral ink (`--edge`) so they read on cork; strings are 3 pixels with a soft shadow, and a selected string shows a thin halo and end dots. Edges between the same two cards fan out. "SVG" in the first design meant the drawing, not the cards.
- Entry points (decided 2026-11-11): a "Boards" row in the sidebar tree, above Trash, expands to the guide's boards; a board's row opens it, and the Boards row itself and the palette command "Boards" open the board list, which creates, renames, and deletes boards. A board's header holds the name, the count, the plus that makes a note, a View menu (the Mentions and Other guides layers and the background), and More with Rename, New board, and Delete, so the name is never cut off. Zoom out, the readout, zoom in, and Fit are a cluster at the canvas's bottom right, one notice row above the edge so a notice never covers it. Entries are added from the picker panel, which the panel's own fold button opens and closes. On a board, Mod+F puts the caret in the picker's search, and the palette offers Add entries, New note, Fit board, and Show or hide the right panel. Each board's pan and zoom come back when it is reopened while the app runs. The panel is two fixed halves, the selection above and the picker below, so a new selection moves nothing. A selected card shows Open entry, Tie a string, and Remove from board, then two tabs: Entry, the rendered body, read-only, and Relations, with the relation line above the list, so a relation can be recorded without leaving the board and its line is drawn at once (decided 2026-11-11). Mod+R puts the caret in that line.
- Notes (decided 2026-11-09): free-standing markdown notes are board-only objects (`board_note`): a body, a position, a width and a height, and one of five colors (yellow, pink, blue, green, white). Core renders them in the guide's context, so wikilinks resolve. A click on an unresolved link in a note creates the entry, as on an entry page, and does not place a card (decided 2026-11-11); the notice offers Undo. Check boxes in a note work: a click flips the marker in the note's markdown (`toggle_note_task`). Entry bodies keep inert check boxes for now: boards are where puzzles and tasks unfold with the entries' help (decided 2026-11-09). Sizing and editing follow Lucidchart (decided 2026-11-09): a note starts square, 200 by 200, and is resized on either axis from eight handles, Shift on a corner keeping the proportions; double-click or Enter edits, the text field fills the note, a click anywhere else saves and ends the edit, and so do Ctrl+Enter and Escape (decided 2026-11-11: nothing in the app drops typed text, and the field's own undo takes typing back); no Save button, no hint text; content taller than the note scrolls inside it.
- Images (decided 2026-11-09): an image dropped or pasted onto the canvas is stored as media and placed as a board image (`board_image`) at the drop point, 200 wide with the height from its proportions, in a polaroid frame (white in light, warm off-white in dark, a deeper band at the bottom), with no text. It is selected, dragged, tied with strings, deleted with Undo, and resized from its corners keeping its proportions (Shift frees them). A board image is a reference to its media, so cleanup never removes the file, and it travels in the archive like gallery media.
- Strings (decided 2026-11-09): a string ties two items, each a card or a note (`board_string`), with an optional label and one of four colors (red, blue, green, black). It is drawn as twine, a curve with a little sag, above the relation edges. The selected item shows a pin; dragging the pin onto another item ties a string, and "Tie a string" in the panel does the same from the keyboard: the next item given Enter is the other end. Keyboard focus selects a card, note, image, or string as a press does, so the panel's actions reach it without a pointer; a string just tied takes focus as well as the selection, so Delete acts on what the panel shows. Selecting a string shows its label and color in the panel; Delete removes it with Undo. A string goes with its end: removing a card from the board or deleting a note deletes its strings (database triggers).
- Backgrounds (decided 2026-11-09): per board, saved with the display toggles: dots (default), grid, cork, blank, each following light and dark.
- The header's plus creates a note at the middle of the window and opens it for editing (decided 2026-11-09); entries are added from the picker panel, which has its own buttons.
- A board file in an archive carries the notes and the strings; a string whose end is a trashed entry is left out like the node.
- Later (`docs/plan.md`, Later): icon push pins, drawings.
- Canvas rendering is a later optimization if SVG falls over past a few hundred cards.

## 7. Export

Export produces a `.zip`. Scopes: vault, guide, tag subtree, single entry. Every archive is self-contained: it carries the verbs, qualifier labels, and tags that its relations and entries reference, and the media its entries use. Trashed entries are not exported.

```
manifest.json                     format_version, app_version, exported_at, scope, root ids
guides/<slug>/
  guide.json                      id, name, description, cover (media path), cover_crop (x, y, w, h in thousandths), tags (global tag paths)
  tags.json                       tree: [{id, name, color, children: [...]}]
  verbs.json                      [{id, label, inverse_label, symmetric, subject_default_tag, object_default_tag}]
  qualifiers.json                 [{id, label, inverse_label}]
  entries/<slug>.md                 front matter + body
  relations.jsonl                 one edge per line
  boards/<slug>.json              layout and filters
media/<sha256>.<ext>              raw files, no base64
```

Slugs are lowercase ASCII, `[a-z0-9-]`, capped at 64 characters, with a numeric suffix on collision. Other scripts are transliterated (`Café` to `cafe`, Cyrillic and CJK to Latin approximations) so file names stay readable; a title with nothing printable falls back to `entry-<id prefix>`. Two entries whose titles differ only by case or accents get distinct slugs. This keeps archives usable on case-insensitive and case-sensitive filesystems alike and inside Windows path limits. The archive writer uses ZIP64 so exports over 4 GB work on FAT32 thumb drives. Entries are written in a stable order (sorted by id) with a fixed timestamp, so two exports of the same vault differ only in the manifest's `exported_at`. Text entries are deflated; media entries are stored uncompressed, since those formats are already compressed.

A relation is written in the guide that owns its verb; an edge whose verb belongs to a guide outside the scope is not in the archive. Full guide and vault exports carry the whole tag tree, vocabulary, and boards. A tag or entry export carries only the tags its entries use (with their ancestors) and the vocabulary its relations use, and no boards, since a board's entries may lie outside the scope. Trashed entries, and edges and qualifiers that touch them, are left out.

Entry front matter:

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
{"id":"018f...","subject":{"id":"...","title":"Stone Block"},"verb":{"id":"...","label":"DROPS"},"object":{"id":"...","title":"Cobblestone Block"},"qualifiers":[{"label":"mined with","entry":{"id":"...","title":"Pickaxe"}}]}
```

Endpoints outside the exported scope are written as `{"external": true, "guide": "<name>", "title": "..."}` with the id when known; qualifier entry values use the same shape. Qualifier lines carry `label_id` beside the label so import can match by id before falling back to the label.

Media is stored raw inside the zip. Base64 would be a third larger and harder to open; a zip already handles binary.

## 8. Import

- Source: any archive at any scope. Target: a new guide, an existing guide, or a tag within a guide (imported entries get that tag added).
- Guide names are unique per vault, case-insensitively, so an archive's guide matches at most one local guide. When it matches, the target picker defaults to "Import into existing <name>" with "Import as new" as the alternative; when nothing matches it defaults to "Import as new". "Import as new" into a vault that already holds that name creates the guide as `<name> (imported)`, mirroring the entry-title rule.
- An archive whose `format_version` is newer than the app understands is refused with a message to update.
- **Mapping step, before anything is written.** For each incoming tag path, verb, and qualifier label:
  - exact match after normalization (case, whitespace, trailing `s`) maps automatically
  - otherwise the top three local candidates by string similarity are offered, with "create new" and, for tags, "create under ..." as alternatives. The metric is the `similarity::Similarity` trait; the default is a normalized edit distance with no dependency, chosen until real archives show whether a prefix-weighted metric ranks better
  - the user can accept all suggestions at once or go through them
- Entry identity: same id updates in place if the incoming `updated` is newer. Different id with a title collision keeps both and renames the incoming entry `Title (imported)`. Both cases appear in the import report. Imported entries keep their ids and timestamps; tags, vocabulary, and boards are matched by name and get new ids. A board is identified by its name within the target guide, so importing again replaces its saved filters and positions instead of adding a copy. Board filters travel as tag paths.
- Wikilinks in imported bodies resolve first against the archive's own entries (by the title at export time), then against the target guide's titles and aliases.
- Relations whose external endpoint resolves (by id, or by guide name and title) are created. Unresolved ones are listed in the report with an option to create stubs for them. With stubs requested, qualifier values that point at missing entries become stubs too; without, they are kept as the title text and counted in the report. Every guide's entries are imported before any relation, so edges between two archive guides resolve.
- A word that collides on import reuses the local word when the label matches; when only its inverse collides, it is created without the inverse. Both are rare, since the mapping step has already matched labels.
- Entry identity is per guide. An archive entry whose id already lives in another guide gets a fresh id in the target guide, so one archive can be imported as two guides and the archive's relations and boards follow each guide's own copies.
- Verb mapping considers local inverse labels too: an archive's `DROPPED BY` maps onto a local `DROPS`, and every edge that used it is stored the other way round, since the canonical edge always runs from the verb's subject.
- With stubs requested, an endpoint whose title a trashed entry still holds is reported as unresolved rather than stubbed; the stub would collide with the reserved title.
- Media entries must be named `media/<sha256>.<ext>` with an extension of one to sixteen letters or digits. Anything else is refused before a byte is written, because the extension is spliced into a path.
- The whole import is one transaction. Media files are extracted to a staging folder beside `media/`, checked against their hashes, and moved into place only as the transaction commits; a failure removes what was moved. Any failure leaves the vault and its media untouched.

## 9. Update check

A menu item, "Check for updates", fetches `latest.json` from the release URL, compares semver with the running version, and shows a dialog with the version and a link. It never downloads, never runs on a schedule, and there is no setting to make it automatic. The Tauri HTTP capability is scoped to that one URL.

## 10. Distribution

- Windows: a bare `.exe` for machines with the WebView2 runtime (all Windows 11, nearly all Windows 10), and a separate zip bundling a fixed WebView2 runtime for machines without it.
- macOS: a zipped `.app`. Unsigned; the README documents right-click, Open for the first launch. Signing and notarization need an Apple Developer account and can be added later without changing anything else.
- Linux: AppImage.
- macOS and Linux builds come from a GitHub Actions workflow with `tauri-action`, since neither can be built from Windows.

## 11. Open questions

- An unknown `ui.theme` or `ui.density` value, such as one written by a newer build, fails the config load rather than falling back to the default.
- Whether qualifier values should support dates as a distinct type.
- Whether to offer lossy WebP as a setting beside the lossless default (section 3). Lossy needs `libwebp` through a C binding; lossless is pure Rust.
- Whether export should offer a hop-depth scope (an entry plus everything within N edges) with a preview of what it will include (phase 9). The archive format needs no change for it.
- Whether import into an existing guide should ask per field (cover, description, guide tags) before overwriting (phase 9). Today the cover is set only when the local guide has none (decided 2026-11-08: a local cover and its crop stay, and a test pins it) and the description is left alone.

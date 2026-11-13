# Design: Trash for boards and guides

Date: 2026-11-11. Base: HEAD 06b6638, schema version 6. Solves UX review 2026-11-10, Failing item 9.

## Summary

- Deleting a board or a guide sets `deleted_at`, as deleting an entry does. No dialog. The notice offers Undo.
- A trashed guide hides everything in it from every other guide: relations, qualifier values, board cards.
- Names stay reserved while in Trash, so restore never collides and no unique index changes.
- Purge is the one irreversible step. It asks, and the dialog states counts that core computes.
- "Add all" returns the ids it added, and its notice offers Undo that takes exactly those cards off.

## What exists today

Schema and core:

- `guide.name_norm` is a column-level `UNIQUE` (`core/migrations/0001_initial.sql:24`). SQLite cannot drop it without a table rebuild.
- `board` has `UNIQUE (guide_id, name_norm)` (`0003_boards.sql:13`). Neither table has `deleted_at`.
- Entry Trash: `trash_entry`, `restore_entry`, `purge_entry`, `list_trash`, `empty_trash` (`core/src/entry.rs:400-448`). Purge requires the entry to be trashed (`Error::NotTrashed`).
- `delete_guide` deletes the row and cascades everything (`core/src/guide.rs:191-199`). Test: `core/tests/behavior/guides.rs:138`.
- `delete_board` deletes the row; notes, images, strings, and nodes cascade (`core/src/board.rs:338-346`).
- `list_guides` and `list_boards` have no live filter (`guide.rs:114`, `board.rs:305`).
- Cross-guide reads decide "trashed" from `entry.deleted_at` only: `lookup_entry_any_guide` (`entry.rs:367-376`), `relations_for_entry` (`relation.rs:351`), the `EntryRef` loader (`relation.rs:595`), `board_view` cards and edges (`board.rs:377`, `451`, `478`).
- `add_board_nodes` and `set_board_positions` call `entry::load_summary` and accept a trashed entry (`board.rs:544`, `585`).
- Media cleanup counts gallery rows, guide covers, board images, and every entry body, trashed or not (`core/src/media.rs:471-502`). Board note bodies are not scanned.
- Backups copy the whole database through the SQLite backup API and leave media out (`core/src/backup.rs:1`).

Export and import:

- Export leaves out trashed entries, edges that touch them, and board nodes and strings on trashed members (`core/src/export.rs:141`, `169`, `367-430`, `570`).
- Vault export walks `list_guides` (`export.rs:106`). Boards are written for full guides only (`export.rs:362`).
- Import matches a local guide by name through `list_guides` (`import.rs:376-392`) and picks a free name the same way (`import.rs:733-742`).
- Import matches a board by name through `list_boards` and replaces its nodes, notes, images, and strings (`import.rs:1251-1271`).
- Import resolves an endpoint by id with `entry.deleted_at` only, and by guide name through `list_guides` (`import.rs:1204-1224`).

UI:

- Entry delete is "Move to Trash" with Undo in the notice (`ui/src/views/EntryView.svelte:300-318`, `514-517`).
- Board delete is a `ConfirmDialog` saying "This cannot be undone." (`ui/src/views/BoardsView.svelte:145-152`, `ui/src/views/BoardView.svelte:866-873`).
- Guide delete is a `ConfirmDialog` that advises an export with no interface yet (`ui/src/views/GuideView.svelte:210-219`, `349-358`).
- The guide's Trash view lists entries only, with Restore, Purge..., and Empty trash (`ui/src/views/TrashView.svelte`). Route `#/guide/<id>/trash` (`ui/src/lib/router.ts:7`).
- The sidebar's Trash node counts trashed entries (`ui/src/components/Sidebar.svelte:323-326`).
- "Add all" calls `addBoardNodes`; the notice has no action (`ui/src/views/BoardView.svelte:285-302`, `ui/src/components/BoardPanel.svelte:327`). `add_board_nodes` returns a count, not ids.
- Notices take one `action` (`ui/src/lib/session.svelte.ts:101`).
- There is no "recent" list anywhere in the UI or core. The palette lists guides with `listGuides` and searches the current guide only (`ui/src/components/Palette.svelte:129`, `144`).
- Global tag counts on the guides list come from `listGuides` in `GuidesSidebar.svelte:29`.

## Design

### Schema: migration 0007

```sql
-- Field Guide schema, version 7: boards and guides go to the Trash.
-- deleted_at is set by trash and cleared by restore; purge deletes the row. A name stays reserved
-- while its holder is in the Trash, so the unique rules of 0001 and 0003 are unchanged.
-- See docs/design.md section 3, Trash.

ALTER TABLE guide ADD COLUMN deleted_at TEXT;
ALTER TABLE board ADD COLUMN deleted_at TEXT;
CREATE INDEX guide_live ON guide (deleted_at);
CREATE INDEX board_guide_live ON board (guide_id, deleted_at);
```

Reasons:

- A `deleted_at` column matches entries. A separate trash table would duplicate rows and break foreign keys.
- Reserving names needs no index change. Freeing names would need partial unique indexes, and `guide.name_norm` is a column constraint, so the guide table would need a rebuild.
- With reserved names, a restore can never meet a taken name. Rename and create already go through the unique rule.

### Rules

Names:

- A trashed guide keeps its name. Creating or renaming a guide to that name fails with `GuideNameTaken { name, in_trash: true }`. The message says "is in the Trash; restore it or purge it first".
- A trashed board keeps its name in its guide. `BoardNameTaken` gains `in_trash` the same way.
- The name frees on purge.

A trashed guide:

- Its entries, tags, aliases, verbs, qualifier labels, relations, boards, mentions, gallery rows, and global tag links are kept unchanged.
- Entries keep their own `deleted_at`. An entry trashed before the guide stays trashed after the guide is restored.
- An entry is hidden when its own `deleted_at` is set or its guide's `deleted_at` is set. Core calls this "hidden".
- Relations from other guides that touch a hidden entry are hidden on both pages, as for a trashed entry. Restore brings them back.
- Qualifier values that point at a hidden entry are left out of display lines (`without_trashed_values`).
- Wikilinks resolve within one guide only, so no link from another guide reaches into it.
- A card for a hidden entry on another guide's board is left out with its edges and strings. Its position is kept.
- `list_guides` leaves it out. So the guides list, the palette, the new-guide "copy vocabulary from" list, and global tag counts leave it out with no further change.
- `lookup_entry_any_guide` leaves out its entries. `create_relation` refuses a hidden origin, target, or qualifier value.
- Creates inside it are refused with `Error::InTrash { kind: "guide", name }`: entries, tags, verbs, qualifier labels, boards, import targets.
- `get_guide`, `get_entry`, and `get_board` still answer, with `deleted_at` set, so a stale page can say where the item went.

A trashed board:

- Its nodes, notes, images, and strings are kept unchanged.
- `list_boards` leaves it out. `board_view` still answers, and `Board.deleted_at` is set.
- Writes to a trashed board are refused with `Error::InTrash { kind: "board", name }`.
- Purging an entry still removes its node from every board, trashed boards included.

Media:

- A trashed guide's cover, a trashed board's images, and the bodies and galleries of a trashed guide's entries still count as references. Cleanup keeps them. This matches trashed entries today.
- After a purge, those files become unreferenced. They go only through "Clean up unused media", which lists them.

Search, palette, counts, recent:

- Search is per guide (`list_entries`), and a trashed guide cannot be opened from any list, so nothing changes.
- The palette's guide rows come from `list_guides`, so a trashed guide is left out.
- Global tag counts come from `list_guides`, so they drop the trashed guide.
- There is no "recent" list today. Any future one must use `list_guides` and `list_boards`.

Visibility in SQL:

- One helper in core, `entry::hidden_sql(alias)`, returns `(alias.deleted_at IS NOT NULL OR EXISTS (SELECT 1 FROM guide g WHERE g.id = alias.guide_id AND g.deleted_at IS NOT NULL))`.
- The `EntryRef` loader returns `coalesce(e.deleted_at, g.deleted_at)` as `deleted_at`. Its doc says "set when the entry or its guide is in the Trash". Existing checks on `EntryRef.deleted_at` then cover relations, qualifier values, and board edges.
- `EntrySummary.deleted_at` and `Entry.deleted_at` stay the entry's own value, so `restore_entry` and the guide's Trash view keep working.
- Places to change: `entry.rs:370`, `relation.rs:595`, `board.rs:377`, `board.rs:478`, `board.rs:544`, `board.rs:585`, `export.rs:373`, `export.rs:570`, `import.rs:1206`, `import.rs:1214`. A leak test (Tests) guards against a missed query.

Export, import, backups:

- Trashed guides and trashed boards are left out of every archive, as trashed entries are. Vault export skips trashed guides through `list_guides`.
- Exporting a trashed guide, or a tag or entry inside one, is refused: "restore it before exporting". This matches `export.rs:141`.
- A live guide's relation into a hidden entry is left out. A live board's node and strings for a hidden entry are left out.
- The archive format does not change. `format_version` stays.
- Import never matches a trashed guide. `propose_import` defaults to "Import as new", and the free name counts trashed names, so it becomes `<name> (imported)`.
- `apply_import` with `GuideTarget::Existing` naming a trashed guide fails with `InTrash` before anything is written.
- Import never matches a trashed board. An incoming board whose name a trashed board holds is created as `<name> (imported)`. The report lists it under `boards_renamed`.
- An endpoint that resolves by id or by guide name to a hidden entry counts as unresolved. With stubs requested, it is reported, not stubbed, as for a trashed title today.
- Backups copy the database file, so trashed boards and guides are in every backup. No change.

### Core API

New, on `Vault`, in `guide.rs` and `board.rs`:

```rust
pub fn trash_guide(&mut self, id: &GuideId) -> Result<Guide>;            // idempotent, like trash_entry
pub fn restore_guide(&mut self, id: &GuideId) -> Result<Guide>;          // Err(NotTrashed) when live
pub fn purge_guide(&mut self, id: &GuideId) -> Result<()>;               // Err(NotTrashed) when live
pub fn list_trashed_guides(&self) -> Result<Vec<Guide>>;                 // newest first
pub fn empty_guide_trash(&mut self) -> Result<usize>;                    // purges every trashed guide
pub fn guide_contents(&self, id: &GuideId) -> Result<GuideContents>;     // counts for the purge dialog

pub fn trash_board(&mut self, id: &BoardId) -> Result<Board>;
pub fn restore_board(&mut self, id: &BoardId) -> Result<Board>;          // refused while its guide is trashed
pub fn purge_board(&mut self, id: &BoardId) -> Result<()>;
pub fn list_trashed_boards(&self, guide_id: &GuideId) -> Result<Vec<Board>>;
pub fn board_contents(&self, id: &BoardId) -> Result<BoardContents>;

pub fn remove_board_nodes(&mut self, id: &BoardId, entry_ids: &[EntryId]) -> Result<usize>;
```

Plain structs:

```rust
pub struct GuideContents {
    pub entries: usize,              // live entries
    pub entries_in_trash: usize,
    pub tags: usize,
    pub verbs: usize,
    pub qualifier_labels: usize,
    pub relations: usize,            // edges with an end in this guide
    pub relations_with_other_guides: usize,
    pub boards: usize,               // live and trashed
    pub notes: usize,
    pub images: usize,               // board images
    pub strings: usize,
    pub cards_on_other_boards: usize, // this guide's entries placed on other guides' boards
}

pub struct BoardContents {
    pub cards: usize,
    pub notes: usize,
    pub images: usize,
    pub strings: usize,
}

pub struct TrashEmptied {
    pub entries: usize,
    pub boards: usize,
}
```

Changes:

- `Guide` and `Board` gain `deleted_at: Option<String>`.
- `delete_guide` goes away; `purge_guide` replaces it and requires the Trash first.
- `delete_board` goes away; `purge_board` replaces it and requires the Trash first.
- `empty_trash(guide_id)` returns `TrashEmptied` and purges trashed boards as well as trashed entries.
- `add_board_nodes` returns `Vec<EntryId>`: the ids it added, in request order. Entries already on the board are not in the list. Hidden entries are refused.
- `Error::NotTrashed` becomes `NotTrashed { kind: &'static str }`. The message says "this {kind} is not in the Trash". The app's error code stays `not_trashed`.
- New `Error::InTrash { kind: &'static str, name: String }`, app code `in_trash`.
- `GuideNameTaken` and `BoardNameTaken` become structs with `name` and `in_trash`.
- `guide::ensure_guide_exists` stays for reads. New `guide::ensure_guide_live` is used by every create path. New `board::ensure_board_live` is used by every board write.
- Commands in `app/src/commands.rs` mirror the new functions one to one; `delete_guide` and `delete_board` commands go.

### UI

Delete becomes "Move to Trash":

- Board list row menu and board header More: "Move to Trash" (no ellipsis, no dialog).
- Notice: `Moved the board "Quest map" to Trash.` with Undo. Undo calls `restoreBoard`.
- Guide page More: "Move guide to Trash". It navigates to the guides list.
- Notice: `Moved the guide "Minecraft" to Trash.` with Undo. Undo calls `restoreGuide` and opens the guide page.
- The two `ConfirmDialog` uses for board delete and the one for guide delete are removed.

Where trashed items are listed:

- Trashed boards: the guide's Trash view gains a second list, "Boards", under "Entries". Columns: Name, Deleted, Contents ("3 notes, 2 images, 5 strings, 12 cards"). The one action row acts on the selected row in either list.
- The guide sidebar's Trash count becomes trashed entries plus trashed boards. `entry_counts` gains `trashed_boards`.
- Trashed guides: the guides list sidebar gains a fixed "Trash" node at the bottom, as the guide sidebar has. Route `#/trash`. The view lists guides with Name, Deleted, Entries. Restore, Purge..., and Empty trash, as in the guide's Trash view.
- A stale route to a trashed board or guide shows "This board is in the Trash." or "This guide is in the Trash." with a Restore button.
- An entry page whose guide is trashed shows "This entry's guide is in the Trash." with a link to `#/trash`.

Purge asks, with exact counts:

- Board: `Purge the board "Quest map"?` Body: `3 notes, 2 images, and 5 strings are removed for good. 12 cards come off the board. The entries stay in the guide.`
- Guide: `Purge the guide "Minecraft"?` Body lines, each present only when its count is not zero:
  - `240 entries, 12 of them already in Trash, are removed for good.`
  - `18 tags, 9 verbs, 4 qualifier labels, and 310 relations go with them.`
  - `7 relations with entries in other guides are removed.`
  - `2 boards with 10 notes, 3 images, and 14 strings are removed.`
  - `5 cards come off boards in other guides.`
  - `Images stay on disk until you run "Clean up unused media" in Settings.`
- Empty trash in a guide: `14 entries and 2 boards are removed for good.` plus the board contents summed.
- Empty trash on the guides list: the summed `GuideContents` of every trashed guide, same lines.
- Focus starts on Cancel in every purge dialog. Today `ConfirmDialog` focuses the danger button (`ConfirmDialog.svelte:27`, UX review item 9). Add a prop `focus="cancel"`.
- Purge has no Undo. That is why it asks.

Add all gets Undo:

- `addEntries` keeps the ids `addBoardNodes` returns.
- Notice: `Added 240 entries to the board.` with Undo. Undo calls `removeBoardNodes(board, added)` once and reloads.
- Undo removes only cards that call added. Cards the user had before stay. A string tied to an added card since then goes with it (trigger), as with a single removal.
- A single add keeps no notice. Delete on the card is its undo.

## Owner decisions

| # | Decision | Options | Recommendation and reason |
|---|---|---|---|
| 1 | Names of trashed boards and guides | (a) reserved until purge; (b) freed on trash, restore renames to "Name (restored)" on collision | (a). It matches entry titles, needs no index rebuild, and restore never fails. |
| 2 | Trashed boards and guides in archives | (a) left out, as trashed entries are; (b) carried with `deleted_at`, a format change | (a). One rule for all Trash. Backups still hold them. |
| 3 | Import meets a name a trashed guide or board holds | (a) import as new with "(imported)", trashed item untouched; (b) refuse and ask to restore first; (c) restore it and import into it | (a). Nothing in the Trash changes without the user asking, and the import still succeeds. |
| 4 | Where trashed boards are listed | (a) the guide's Trash view, a Boards list under Entries; (b) a Trash section on the Boards page | (a). One Trash per guide, and the sidebar node already leads there. |
| 5 | Trash for guides | (a) a "Trash" node in the guides list sidebar, route `#/trash`; (b) a "Show trashed" toggle on the guides list | (a). It mirrors the guide sidebar's fixed Trash node. |
| 6 | Empty trash scope | (a) a guide's Empty trash covers entries and boards, and the guides Trash has its own Empty trash; (b) per-item purge only for boards and guides | (a). The dialog states every count, so the bulk step stays explicit. |
| 7 | Restoring a board while its guide is in Trash | (a) refused, restore the guide first; (b) allowed, the board stays hidden with the guide | (a). The board is not reachable until the guide is back, so a silent restore would look like a failure. Unreachable from the UI in practice. |

## Build batches

Batch 1. Schema and guide Trash in core. Effort M.

- Files: `core/migrations/0007_trash_boards_guides.sql`, `core/src/db.rs` (migration list), `core/src/guide.rs`, `core/src/error.rs`, `core/src/entry.rs`, `core/src/relation.rs`, `core/src/board.rs` (hidden entries on boards), `core/src/tag.rs`, `core/src/vocabulary.rs` (live-guide check on create).
- Tests, `core/tests/behavior/trash.rs` and `guides.rs`:
  - `a_trashed_guide_hides_its_entries_from_every_other_guide`: an edge from guide A to an entry in B, B's entry on A's board, a qualifier value in B. Trash B. A's relation list, board view, and `lookup_entry_any_guide` show none of it. Restore B. All of it is back, positions included.
  - `restoring_a_guide_keeps_entries_that_were_trashed_before_it`.
  - `a_trashed_guide_keeps_its_name_and_purge_frees_it`.
  - `purge_requires_the_trash_and_removes_everything_in_the_guide`: replaces `deleting_a_guide_removes_its_entries_and_tags_and_nothing_else`.
  - `nothing_can_be_created_in_a_trashed_guide`: entry, tag, verb, board, relation target.
  - `guide_contents_counts_what_purge_removes`: counts before purge equal rows gone after purge.
  - Leak test, property style: build two guides with edges, boards, and qualifiers both ways; trash one; every public read that takes the other guide returns no id from the trashed guide.
  - `schema.rs`: version is 7.

Batch 2. Board Trash in core. Effort S.

- Files: `core/src/board.rs`, `core/src/entry.rs` (`empty_trash` returns `TrashEmptied`), `core/src/error.rs`.
- Tests, `core/tests/behavior/boards.rs`:
  - `a_trashed_board_keeps_notes_images_and_strings_and_restore_returns_them`.
  - `a_trashed_board_keeps_its_name_in_the_guide`.
  - `writes_to_a_trashed_board_are_refused`.
  - `board_images_on_a_trashed_board_are_kept_by_media_cleanup` (in `media_cleanup.rs`).
  - `empty_trash_purges_trashed_boards_and_entries`.
  - `add_board_nodes_returns_only_the_ids_it_added_and_refuses_hidden_entries`.
  - `remove_board_nodes_takes_off_exactly_the_given_cards`: Add all then Undo leaves the board as before, earlier cards and their strings intact.

Batch 3. Export and import. Effort M.

- Files: `core/src/export.rs`, `core/src/import.rs`.
- Tests, `core/tests/behavior/export_import.rs`:
  - `a_vault_export_leaves_out_trashed_guides_and_boards`: export, import into an empty vault, compare with the live set only.
  - `an_edge_into_a_trashed_guide_is_not_exported`.
  - `exporting_a_trashed_guide_is_refused`.
  - `import_never_matches_a_trashed_guide`: the plan says New, the name gets "(imported)", the trashed guide is unchanged.
  - `import_into_a_trashed_guide_is_refused_and_writes_nothing`.
  - `an_incoming_board_whose_name_a_trashed_board_holds_is_created_renamed`.
  - `an_endpoint_in_a_trashed_guide_stays_unresolved`.
  - Round trip: create guide with boards, trash one board, export, import as new, compare. The trashed board is absent and everything else is equal.

Batch 4. Commands and UI. Effort M.

- Files: `app/src/commands.rs`, `app/src/main.rs`, `app/src/error.rs` (codes `in_trash`), `ui/src/lib/api.ts`, `ui/src/lib/router.ts` (`#/trash`), `ui/src/views/TrashView.svelte` (boards list), new `ui/src/views/GuideTrashView.svelte`, `ui/src/components/GuidesSidebar.svelte`, `ui/src/components/Sidebar.svelte`, `ui/src/views/BoardsView.svelte`, `ui/src/views/BoardView.svelte`, `ui/src/views/GuideView.svelte`, `ui/src/views/EntryView.svelte` (banner), `ui/src/components/ConfirmDialog.svelte` (`focus` prop), `ui/src/components/BoardPanel.svelte`.
- Tests, Vitest: `router.test.ts` parses `#/trash`. A pure helper `purgeLines(contents)` in `ui/src/lib/trash.ts` with tests: zero counts drop their line, singular and plural are right.
- Docs: `docs/design.md` sections 3 (Trash), 5 (Trash browse view), 6 (line 225 "deleting a board asks first" becomes Trash), 7, 8. `docs/plan.md` if the phase list names these.

## Risks

- A missed cross-guide query leaks a trashed guide's entries. The leak test is the guard. Any new cross-guide query must use `hidden_sql` or `EntryRef`.
- Reserved names surprise a user who trashed "Minecraft" and makes a new "Minecraft". The error names the Trash and both ways out.
- Undo for Add all also removes strings the user tied to the new cards in the meantime. The notice is short-lived, so this is rare.
- Import replacing a live board by name is unchanged: it still drops local notes added since the export (`import.rs:1251-1271`). Out of scope here, but it is the same class of loss as item 9.
- Purge plus media cleanup plus a backup restore loses image files, since backups hold no media. The purge dialog's last line points at cleanup; Settings should say backups do not hold media.
- `empty_trash` changes its return type. The UI and any test using the count must change in the same batch.

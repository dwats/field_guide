# T2 plan: Trash for boards and guides

Status: approved 2026-11-13 (decisions 17 to 20, all (a)). Written at head 0b718e4 (last code commit ce8a1cc, schema version 6). Batches 1 to 4 are built, reviewed, and pushed at 0286726. Batch 5, the review follow-up: part A is built (e4dc7c8); the tests for parts B, C, and D are written with their stubs, not committed, and wait for the owner's approval (decision 28). The Progress list at the end says where the work stands.

The design is `.claude/work/trash-boards-guides/design.md`. This plan does not repeat it. It says what is built in what order, what differs from the design, and which tests gate each batch.

## Goal

Deleting a board or a guide sets `deleted_at` and shows a notice with Undo, as deleting an entry does. Purge is the one step that cannot be reversed. It asks, and it states what is lost with counts that core computed.

## The task in my words

- Core gains Trash, restore, purge, and a contents count for guides and for boards. Migration 0007 adds the two columns.
- A trashed guide hides its entries from every other guide: relations, qualifier values, board cards, cross-guide lookup. Restore brings all of it back.
- A trashed board keeps its notes, images, and strings. It leaves `list_boards`. Writes to it are refused.
- Names stay reserved in Trash (decision 5). Archives leave trashed guides and boards out (decision 6). Import never matches a trashed name and creates "Name (imported)" (decision 7).
- The UI replaces the three delete confirm dialogs with "Move to Trash" and a notice with Undo. Trashed boards are listed in the guide's Trash (decision 8). Trashed guides are listed on a new `#/trash` page reached from the guides sidebar (decision 9). Empty trash covers boards and guides (decision 10). A board cannot be restored while its guide is in Trash (decision 11).

## What differs from the design

The design was written at 06b6638. Checked against the code on 2026-11-13:

| Design says | Code today | Plan |
|---|---|---|
| Add all gets Undo through a new `remove_board_nodes`, and `add_board_nodes` returns ids | Undo is built in the UI: `takeOff` in `ui/src/views/BoardView.svelte` calls `removeBoardNode` once per card. `add_board_nodes` returns a count. | Left as built (decision 18). |
| `add_board_nodes` must refuse hidden entries | `check_joinable` in `core/src/board.rs` refuses an entry whose own `deleted_at` is set | Extend `check_joinable` to an entry whose guide is in Trash. |
| `ConfirmDialog` needs a `focus="cancel"` prop | It has `destructive`, on by default, which starts focus on Cancel | No change to `ConfirmDialog`. |
| Board note bodies are not scanned by media cleanup | They are scanned since 8b15673 | No change. The cleanup test for a trashed board still gets written. |
| `delete_guide` and `delete_board` go in batches 1 and 2 | The app commands and the UI call them | They stay until batch 4, so every commit on `main` leaves a working app. Batch 4 removes them with the commands. |
| Line numbers | All moved | The plan names functions, not lines. |

## Readings I am using

Each follows the design's wording. Say so if one is wrong.

- In a trashed guide, creates are refused. Edits to things that already exist are not refused (an entry body, a rename, a cover). The UI gives no way to reach them. Wrong, found by the review on 2026-11-13: Back and the guide sidebar reach the Tag editor, Vocabulary, Boards, a board, and the guide's Trash of a trashed guide (decision 24).
- `ensure_board_live` checks the board's own `deleted_at`. A live board inside a trashed guide is unreachable from the UI and is not refused in core. Wrong in the same way: Back reaches it (decision 24).
- `Guide.deleted_at` and `Board.deleted_at` are serialized to the UI. `EntrySummary.deleted_at` and `Entry.deleted_at` stay the entry's own value. `EntryRef.deleted_at` is set when the entry or its guide is in Trash.
- `ImportReport` gains `boards_renamed`, a list of from and to names, like `entries_renamed`.
- `EntryCounts` gains `trashed_boards`.
- The archive format does not change.

## Approach

Four batches, as in the design. Each core batch runs the cycle AGENTS.md asks for: tests first, shown, approved, committed, then code until they pass. Each batch ends with all gates passing and one commit, so a new session can start at a batch boundary.

### Batch 1: schema and guide Trash in core

Files: `core/migrations/0007_trash_boards_guides.sql`, `core/src/db.rs`, `core/src/guide.rs`, `core/src/error.rs`, `core/src/entry.rs`, `core/src/relation.rs`, `core/src/board.rs` (hidden entries on other guides' boards), `core/src/tag.rs`, `core/src/vocabulary.rs`, `core/src/lib.rs`, `app/src/error.rs` (the changed error variants must still compile; new code `in_trash`).

Code:

- Migration 0007 as in the design. `SCHEMA_VERSION` becomes 7.
- `trash_guide`, `restore_guide`, `purge_guide`, `list_trashed_guides`, `empty_guide_trash`, `guide_contents`, and the struct `GuideContents`.
- `list_guides` leaves out trashed guides. `get_guide` still answers, with `deleted_at` set.
- `guide::ensure_guide_live` on every create path: entry, stub, tag, verb, qualifier label, board, vocabulary copy target.
- `entry::hidden_sql(alias)` and its use in `lookup_entry_any_guide`, the `EntryRef` loader, and the `board_view` card and mention queries. `create_relation` refuses a hidden origin, target, or qualifier value through `EntryRef.deleted_at`.
- Errors: `NotTrashed { kind }`, `InTrash { kind, name }`, `GuideNameTaken { name, in_trash }`.

Tests, in `core/tests/behavior/guides.rs`, `trash.rs`, and `schema.rs`:

| Test | Done criterion |
|---|---|
| `a_version_6_vault_migrates_to_7_with_nothing_trashed` | 1 |
| `a_trashed_guide_hides_its_entries_from_every_other_guide` | 2 |
| `restoring_a_guide_keeps_entries_that_were_trashed_before_it` | 2 |
| `no_read_of_a_live_guide_returns_anything_from_a_trashed_guide` (the leak test: two guides with edges, qualifier values, mentions, and board cards both ways; trash one; every public read of the other returns no id from it) | 2 |
| `a_trashed_guide_keeps_its_name_and_purge_frees_it` | 5 (names) |
| `purging_a_guide_requires_the_trash_and_removes_everything_in_it` | 2, 8 |
| `guide_contents_counts_what_purge_removes` | 7 |
| `restoring_a_live_guide_is_an_error_and_trashing_twice_is_not` | 2 |
| `nothing_can_be_created_in_a_trashed_guide` | 4 |
| `empty_guide_trash_purges_every_trashed_guide_and_no_live_one` | 8 |

Existing tests that change in this batch, pattern only: `trash.rs` (`Error::NotTrashed` becomes `Error::NotTrashed { .. }`), `guides.rs` (`GuideNameTaken(name)` becomes `GuideNameTaken { name, .. }`).

### Batch 2: board Trash in core

Files: `core/src/board.rs`, `core/src/entry.rs` (`empty_trash`), `core/src/tag.rs` (`trashed_boards`), `core/src/error.rs`, `app/src/error.rs`, `app/src/commands.rs` (the `empty_trash` return type), `ui/src/lib/api.ts` and `ui/src/views/TrashView.svelte` (the one notice line that reads the count).

Code:

- `trash_board`, `restore_board` (refused while its guide is in Trash), `purge_board`, `list_trashed_boards`, `board_contents`, and the struct `BoardContents`.
- `list_boards` leaves out trashed boards. `get_board` and `board_view` still answer, with `deleted_at` set.
- `board::ensure_board_live` on every board write: rename, filters, nodes, positions, notes, images, strings.
- `empty_trash` returns `TrashEmptied { entries, boards }` and purges trashed boards too.
- `BoardNameTaken { name, in_trash }`.

Tests, in `core/tests/behavior/boards.rs`, `trash.rs`, and `media_cleanup.rs`:

| Test | Done criterion |
|---|---|
| `a_trashed_board_keeps_notes_images_and_strings_and_restore_returns_them` | 3 |
| `a_trashed_board_keeps_its_name_in_the_guide_and_purge_frees_it` | 5 (names) |
| `writes_to_a_trashed_board_are_refused` | 4 |
| `board_images_on_a_trashed_board_are_kept_by_media_cleanup` | 3 |
| `purging_a_board_requires_the_trash_and_board_contents_counts_what_goes` | 7 |
| `empty_trash_purges_trashed_boards_and_entries` | 8 |
| `restoring_a_board_is_refused_while_its_guide_is_in_the_trash` | 8 |
| `an_entry_in_a_trashed_guide_cannot_join_a_board` | 2 |

Existing tests that change: `boards.rs` (`BoardNameTaken(_)` pattern), `trash.rs` (`empty_trash_purges_only_the_guides_trashed_entries` compares a struct, not a number).

### Batch 3: export and import

Files: `core/src/export.rs`, `core/src/import.rs`.

Code:

- Export of a trashed guide, or of a tag or entry inside one, is refused: "restore it before exporting". Vault export skips trashed guides and boards through the list functions. Edges, nodes, and strings that touch a hidden entry are left out.
- Import: the free guide name counts trashed names. `GuideTarget::Existing` on a trashed guide fails with `InTrash` before anything is written. A board whose name a trashed board holds is created as "Name (imported)" and listed in `boards_renamed`. An endpoint that resolves to a hidden entry counts as unresolved.

Tests, in `core/tests/behavior/export_import.rs`, each a round trip or an import against a prepared vault:

| Test | Done criterion |
|---|---|
| `a_vault_export_leaves_out_trashed_guides_and_boards` | 5 |
| `a_guide_with_a_trashed_board_round_trips_without_it` | 5 |
| `an_edge_into_a_trashed_guide_is_not_exported` | 5 |
| `exporting_a_trashed_guide_is_refused` | 5 |
| `import_never_matches_a_trashed_guide` | 5 |
| `import_into_a_trashed_guide_is_refused_and_writes_nothing` | 5 |
| `an_incoming_board_whose_name_a_trashed_board_holds_is_created_renamed` | 5 |
| `an_endpoint_in_a_trashed_guide_stays_unresolved` | 5 |

### Batch 4: commands, UI, docs

Files: `app/src/commands.rs`, `app/src/main.rs`, `ui/src/lib/api.ts`, `ui/src/lib/router.ts`, new `ui/src/lib/trash.ts`, `ui/src/views/TrashView.svelte`, new `ui/src/views/GuideTrashView.svelte`, `ui/src/components/GuidesSidebar.svelte`, `ui/src/components/Sidebar.svelte`, `ui/src/views/BoardsView.svelte`, `ui/src/views/BoardView.svelte`, `ui/src/views/GuideView.svelte`, `ui/src/views/EntryView.svelte`, `ui/src/App.svelte` (the new route), `core/src/guide.rs` and `core/src/board.rs` (remove `delete_guide` and `delete_board`), and the core tests that still call them.

Code:

- Commands mirror the new core functions one to one. `delete_guide` and `delete_board` go, from the app and from core.
- "Move to Trash" on the board list row, the board header More, and the guide page More. Each shows a notice with Undo and no dialog. The copy is in the design, section UI.
- The guide's Trash gains a Boards list under Entries, with Name, Deleted, and Contents. Restore and Purge act on the selected row in either list.
- The guide sidebar's Trash count is trashed entries plus trashed boards.
- The guides sidebar gains a fixed Trash row. `#/trash` lists trashed guides with Restore, Purge, and Empty trash.
- A page for a trashed board or guide says so and offers Restore. An entry page in a trashed guide says so and links to `#/trash`.
- Purge and Empty trash dialogs state the counts from `board_contents` and `guide_contents`. The line builders are pure functions in `ui/src/lib/trash.ts`.

Tests, Vitest:

- `router.test.ts`: `#/trash` parses and round-trips.
- `trash.test.ts`: `purgeLines(GuideContents)` drops a line whose counts are zero and gets singular and plural right. The same for the board line and the Empty trash lines.

Existing core tests that change in this batch: four calls of `delete_board` and three of `delete_guide` in `boards.rs`, `guides.rs`, and `relations.rs` become Trash then purge. `deleting_a_guide_removes_its_entries_and_tags_and_nothing_else` is removed; the batch 1 purge test covers it.

Docs:

- `docs/design.md` sections 3 (Trash, schema sketch), 5 (Trash browse view, Recovery), 6 (the line "deleting a board asks first"), 7, and 8.
- `docs/checklist.md`: the lines "Delete guide asks first" and "Delete board (asks first)" are rewritten, and lines for the new Trash behavior are added, unchecked.
- `tools/walkthrough/board.ps1` step 10 clicks "Delete board...". It is changed to "Move to Trash". The script is parsed, not run.

### Batch 5: review follow-up (decisions 24 to 27, all answered (a) on 2026-11-13)

Not started. Four parts. Parts B, C, and D change core, so each needs its tests first, with stubs, shown to the owner before they are committed. They can be shown together in one stop. Part A is UI only.

Part A, decision 24: every page inside a trashed guide says so.

- Rule, already in `docs/design.md` section 3: every page inside a trashed guide shows "This guide is in the Trash" with Restore in place of its content, and the guide sidebar is not drawn.
- Today only `GuideView` and `EntryView` check `guide.deleted_at`. `EntryView` shows a link to `#/trash`, not Restore. `BoardView` checks the board only.
- To change: `ui/src/views/TagsView.svelte`, `VocabularyView.svelte`, `BoardsView.svelte`, `TrashView.svelte`, `BoardView.svelte` (a live board in a trashed guide), `EntryView.svelte` (Restore, to match), and `ui/src/App.svelte` (no `Sidebar` while `session.current.guide?.deleted_at` is set).
- Suggested shape: one small component, `ui/src/components/TrashedGuide.svelte`, with the heading, the muted line, and Restore, which calls `api.restoreGuide` and then `touch()`. Each view renders it when its loaded guide has `deleted_at`. Do not decide in `App.svelte` from `session.current` alone: the entry and board routes carry no guide id, and a stale `session.current` would block the next page from loading.
- Open detail, list under Guesses: what the sidebar column shows on such a page. Suggested: the guides sidebar, since every view has the sidebar column.
- No Vitest test fits; add one line to `docs/checklist.md` and report it as not judged.
- Built 2026-11-13 (e4dc7c8). `TrashedGuide.svelte` holds the heading, the muted line, and Restore. The seven views render it when the guide they loaded has `deleted_at`. The Tag editor, Vocabulary, and the entry page do not follow `session.revision`, so they pass `onrestored` and reload themselves. `App.svelte` uses `session.current` for the sidebar column only.
- Guesses made while building, reported to the owner: the sidebar column shows the guides list's sidebar; the entry page shows the same message as the others (it no longer names the entry or links to `#/trash`); on a board in the Trash inside a guide in the Trash, the guide's message comes first (decision 11); the palette lists no entries of a trashed guide and offers no "create entry" row; New (Mod+N) opens New guide there; the breadcrumbs still name the guide and the page.

Part B, decision 25: the purge dialog names what a guide purge removes from other guides.

- `GuideContents` gains two counts. Suggested names: `values_in_other_guides`, the qualifier values on relations with no end in this guide that point at this guide's entries; and `strings_on_other_boards`, the strings on boards of other guides with an end on a card of this guide's entries.
- The foreign key `relation_qualifier.value_entry_id` cascades, and the trigger `board_string_follows_node` deletes the strings. The reviewer's scratch run showed both.
- Tests first, in `core/tests/behavior/trash.rs`: a new test with its own fixture, `guide_contents_counts_what_purge_removes_from_other_guides`. The approved test `guide_contents_counts_what_purge_removes` gains the two fields in its struct literals, both 0; the owner allowed this change (decision 25).
- UI: `ui/src/lib/api.ts` (the type), `ui/src/lib/trash.ts` (two lines in `guidePurgeLines`), `ui/src/lib/trash.test.ts`.
- Docs: `docs/design.md` section 3, the Purge bullet.

Part C, decision 26: Empty trash on the guides list states exact counts.

- Today `emptyGuideTrashLines` sums each guide's `GuideContents`, so a relation or a card between two trashed guides is counted twice, and "other guides" includes guides that are going too. Two empty guides read "2 guides are removed for good. The guide is empty."
- New core function, suggested `guide_trash_contents() -> Result<GuideContents>`: the counts for every trashed guide taken together, each row once, where "other guides" means guides that are not in the Trash. `guide_contents` and it can share one set of queries over "the guide is in this set".
- Test first: two trashed guides joined by a relation, a qualifier value, and a card on each other's board, beside a live third guide; the expected struct is exact, and the rows are gone after `empty_guide_trash`.
- App: a command `guide_trash_contents`. UI: `emptyGuideTrashLines` takes the guide count and the combined contents; `sumGuides` goes; the empty-guide line reads right for several guides.

Part D, decision 27: two small gaps in core.

- `relation::insert_qualifiers` with `LabelSpec::New` creates a qualifier label without `guide::ensure_guide_live`, so `add_qualifier` and `replace_qualifiers` on a relation of a trashed guide create one. Test first: `a_new_qualifier_label_cannot_be_created_through_a_relation_of_a_trashed_guide`, expecting `Error::InTrash { kind: "guide", .. }`.
- `verb_usage` counts "in trash" from the ends' own `deleted_at` (`core/src/vocabulary.rs`, near the `coalesce(sum(...))`), so a relation into a trashed guide is counted as in use with no Trash note, and Delete is refused with no reason shown. Use `entry::hidden_sql`. Check `qualifier_label_usage` for the same. Test first: `verb_usage_counts_a_relation_into_a_trashed_guide_as_in_trash`.
- Checked 2026-11-13: `QualifierLabelUsage` has no "in trash" count at all, for trashed entries or trashed guides, so there is nothing to make consistent there. A stub value (`QualifierValue::Stub`) on a relation of a trashed guide is already refused, through `entry::insert_entry`.

The tests of parts B, C, and D as written 2026-11-13 (not committed; decision 28):

| Test | File | Part | Fails with the stubs at |
|---|---|---|---|
| `guide_contents_counts_what_purge_removes_from_other_guides` | `trash.rs` | B | the first struct comparison: both new counts are 0, expected 2 and 3 |
| `guide_trash_contents_counts_every_trashed_guide_together_and_each_row_once` | `trash.rs` | C | the struct comparison: the stub returns all zeros |
| `a_new_qualifier_label_cannot_be_created_through_a_relation_of_a_trashed_guide` | `trash.rs` | D | `add_qualifier` returns `Ok`, expected `InTrash` |
| `verb_usage_counts_a_relation_into_a_trashed_guide_as_in_trash` | `vocabulary.rs` | D | `in_trash` is 0, expected 1 |

The stub layer: `GuideContents` gains `values_in_other_guides` and `strings_on_other_boards`, both set to 0 in `guide_contents`; `guide_trash_contents` returns `GuideContents::default()`. The approved test `guide_contents_counts_what_purge_removes` gains the two fields at 0 in its two struct literals and changes in nothing else (decision 25). Helpers added to `trash.rs`: `note_end`, `tie`.

Build notes for after approval:

- Part D, label: check `guide::ensure_guide_live` only when a label would really be created. A `LabelSpec::New` whose text names a label that already exists is an edit, which core allows by the rule in `docs/design.md` section 3. Report as a guess.
- Parts B and C, dialog copy, to report as guesses: "N qualifiers on relations in other guides name these entries and are removed." and "N strings tied to those cards go with them." For several guides the empty line reads "The guides are empty."

After Batch 5: every gate, the `reviewer` on the diff from 0286726, the wrap-up of T2 in `.claude/tasks.md` (status, done criteria, the list of not judged checks), commit, push.

## Gates

After every batch: `cargo test --workspace`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all`, `npm test`, `npm run check`. After batch 4, the `reviewer` subagent reads the whole diff from 0b718e4.

The app is not launched. Done criteria 6 and 7 are about what a window shows. Their logic is covered by the Vitest tests above. What only a window can prove is listed at the end as "not judged" for the owner to check.

## Done criteria

From T2 in `.claude/tasks.md`, with the batch that closes each:

1. Migration 0007, schema version 7, nothing trashed after migrating. Batch 1.
2. A trashed guide hides its entries, relations, qualifier values, and board cards from every other guide; restore returns all of it, positions included. Batch 1, with one test in batch 2.
3. A trashed board keeps its notes, images, and strings; restore returns them; media cleanup keeps its images. Batch 2.
4. Nothing can be created in a trashed guide; writes to a trashed board are refused. Batches 1 and 2.
5. Export, import, and name rules follow decisions 5 to 7, each with a round-trip test. Batches 1 to 3.
6. Delete board and Delete guide show a notice with Undo and no dialog. Batch 4.
7. Trashed boards and guides are listed where decisions 8 and 9 say, with Restore and Delete for good; purge names the counts core computed. Batch 4.
8. Empty Trash and restore follow decisions 10 and 11. Batches 1, 2, and 4.
9. `docs/design.md` sections 3, 5, 6, 7, and 8 are updated. Batch 4.

## Risks

- A cross-guide query that the plan missed leaks a trashed guide's entries. The leak test is the guard.
- T2 is four batches. It will likely not fit in one session. Each batch ends in a commit with passing gates, and this file records which batch is done.
- Between batch 1 and batch 4, core can trash a guide but the UI still deletes for good. Nothing reaches the new functions until batch 4.
- Purge, then media cleanup, then a backup restore loses image files, because backups hold no media. Known, out of scope, already in Carry-over.

## Answers (2026-11-13)

- The plan is approved (decision 17).
- `add_board_nodes` and `remove_board_nodes` stay as they are. Add all with Undo is already built in the UI (decision 18).
- The core tests are approved once per core batch (decision 19).
- A new test is committed with stubs, the way the Rust book does it (chapter 12.4, listings 12-15 and 12-16): just enough code that the test compiles and does not panic inside the new function, so it fails at its own assertion. A stub returns an empty or unchanged value. It never calls `unimplemented!` or `todo!` (decision 20).

## The stub layer of a batch

What goes in with the tests, before the code:

- The new public functions with their final signatures. Each body returns an unchanged or empty value: `trash_guide` returns the guide as it is, `list_trashed_guides` returns an empty list, `guide_contents` returns zeros.
- The new struct fields, set to `None`, and the new structs.
- The error variants in their final shape, with their messages. `in_trash` is always false until the code exists.
- The existing tests that match a changed variant, changed in pattern only.

The migration is not part of the stub layer. It is code.

## Deviations

- The leak test reads lists, searches, and drawings. It leaves out reads by id of a thing itself (`get_guide`, `get_entry`, `get_relation`) and the raw saved positions (`board_layout`); they answer for a thing in the Trash on purpose. Its fixture ties no string to a card from the other guide: `board_view` returns every string and the page leaves out those whose card is hidden, as it does today for a trashed member.
- The test `restoring_a_guide_is_refused...` for decision 11 is in batch 2, where `restore_board` is.
- Batch 3: a second import of the same archive, while a board in the Trash still holds the name, makes one more board, "Name (imported 2)". Matching a renamed board on a later import is part of decision 12 and out of scope.
- Batch 3: an endpoint that resolves to an entry in a trashed guide is reported with a reason that names the Trash, whether or not stubs were asked for. An endpoint whose own entry is in the Trash keeps today's rule.

## Progress

- [x] Plan approved (2026-11-13)
- [x] Batch 1 tests approved and committed (2026-11-13, decision 21). With the stub layer, the 10 new tests fail at their assertions, and the 151 old behavior tests and 31 unit tests pass.
- [x] Batch 1 built, gates pass, committed (2026-11-13). The approved tests were not edited. Gates: 31 unit, 161 behavior, 159 UI tests, clippy clean, format clean, svelte-check clean. `check_joinable` is not extended yet; its test is in batch 2.
- [x] Batch 2 tests approved and committed (2026-11-13, decision 22). With the stub layer, the 8 new tests fail at their assertions and the 161 other behavior tests pass. The stub layer changes `empty_trash` to return `TrashEmptied`, so `app/src/commands.rs`, `ui/src/lib/api.ts`, and one line of `ui/src/views/TrashView.svelte` are in it.
- [x] Batch 2 built, gates pass, committed (2026-11-13). The approved tests were not edited. Gates: 31 unit, 169 behavior, 159 UI tests, clippy clean, format clean, svelte-check clean.
- [x] Batch 3 tests approved and committed (2026-11-13, decision 23). The stub layer is one report field, `ImportReport.boards_renamed`. 7 new tests fail for the expected reason; `a_guide_with_a_trashed_board_round_trips_without_it` passes already and stays as a pin.
- [x] Batch 3 built, gates pass, committed (2026-11-13). The approved tests were not edited. Gates: 31 unit, 177 behavior, 159 UI tests, clippy clean, format clean, svelte-check clean.
- [x] Batch 4 built, gates pass, reviewer run, committed (2026-11-13, ca94215). Gates: 31 unit, 176 behavior, 176 UI tests, clippy clean, format clean, svelte-check clean.
- [x] Review findings that needed no decision: fixed and pushed (2026-11-13, 0286726).
- [x] Decisions 24 to 27 answered (a) (2026-11-13). Decision 24 is recorded in `docs/design.md` section 3.
- [x] Batch 5, part A: pages inside a trashed guide (UI). Built and committed 2026-11-13 (e4dc7c8). svelte-check clean, 176 UI tests pass. Not seen in a window; one unchecked line in `docs/checklist.md`.
- [x] Batch 5, parts B, C, D: tests with stubs written and shown (2026-11-13). The 4 new tests fail at their own assertions; the 176 other behavior tests and 31 unit tests pass; clippy and format are clean.
- [ ] Batch 5, parts B, C, D: tests approved (decision 28) and committed
- [ ] Batch 5, parts B, C, D: built, gates pass, committed
- [ ] Reviewer run on the diff from 0286726
- [ ] T2 wrapped up in `.claude/tasks.md`: status, done criteria, not judged checks; pushed

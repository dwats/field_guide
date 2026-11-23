# T3 plan: Pill toggle on lines

Status: approved 2026-11-23 (decisions 33 to 36, all (a)). Written at head 6840717 (schema version 7, archive `format_version` 1). The Progress list at the end says where the work stands.

## Goal

A string's label can be drawn without its pill, one string at a time, so a busy board stays readable. The choice is saved with the string and travels in the archive.

## The task in my words

- "Line" in this task means a string. Decision 3 was answered (a): strings only. Relation lines and mention lines keep the pill always; they are derived and have no row to hold a choice.
- A string gains one stored yes or no value, "label background", on by default.
- The board panel's section for a selected string gains a "Label background" switch.
- With the value off, the string's label is drawn as text with no box behind it.
- Export writes the value; import reads it; an archive without it means on.
- `docs/design.md` section 6 records the rule; section 7 records the field.

## Readings I am using

Each follows the task text or the code. Say so if one is wrong.

- A new string starts with the label background on. `create_board_string` keeps its signature.
- Every string that exists when the vault migrates has the label background on.
- The value is kept when the label is empty. A string with no label draws no label either way.
- A change to the label or the color alone does not change the value, and the reverse.
- A change to the value on a board in the Trash is refused, as every write to a trashed board is (`ensure_board_live`, already in `update_board_string`).
- Undo of "Deleted the string", and Undo of a deleted note, image, or removed card that took strings with it, ties each string again with the value it had. Today `retie` in `ui/src/views/BoardView.svelte` restores the label and the color only.

## Approach

One batch. Core first, by the cycle in AGENTS.md: tests with stubs, shown, approved, committed, then code until they pass. Then `app` (nothing to change beyond a rebuild; the command forwards `BoardStringUpdate`), then `ui`, then docs.

### Core

Files: `core/migrations/0008_string_label_background.sql`, `core/src/db.rs`, `core/src/board.rs`, `core/src/archive.rs`, `core/src/export.rs`, `core/src/import.rs`.

- Migration 0008: `ALTER TABLE board_string ADD COLUMN label_background INTEGER NOT NULL DEFAULT 1;`. No table rebuild, so the three triggers stay as they are. `SCHEMA_VERSION` becomes 8.
- `BoardString` gains `label_background: bool`. `BoardStringUpdate` gains `label_background: Option<bool>` with `#[serde(default)]`.
- `STRING_COLUMNS`, `string_from_row`, `insert_string`, and the `UPDATE` in `update_board_string` carry the column.
- `BoardStringFile` gains `label_background: bool` (decision 34 (a): written only when `false`; absent means `true`).
- Export copies the value. Import copies it into the row it inserts.

Stub layer for the tests (decision 20's way): the two struct fields exist; reads return `true`; `update_board_string` ignores the new field; no migration file; `SCHEMA_VERSION` stays 7; export and import do not carry the value. Each new test compiles and fails at its own assertion, except where noted.

### UI

Files: `ui/src/lib/api.ts`, `ui/src/components/Switch.svelte` (new), `ui/src/components/BoardPanel.svelte`, `ui/src/views/BoardView.svelte`, `ui/src/components/BoardCanvas.svelte`.

- `api.ts`: `BoardString.label_background: boolean`; `BoardStringUpdate.label_background?: boolean`.
- `Switch.svelte`: a `button` with `role="switch"` and `aria-checked`, the label at the left and the 36 by 20 track at the right, drawn as `PrefCard.svelte` draws its track. `PrefCard` is not changed.
- `BoardPanel.svelte`: the switch "Label background" sits between the color swatches and "Delete string". A new prop `onstringbackground(id, on)`.
- `BoardView.svelte`: the handler calls `updateString(id, { label_background })`. `retie` also restores the value when it is off.
- `BoardCanvas.svelte`: a string's label gets the class `bare` when the value is off. Decision 33 (a): `bare` is text with no box and no border, with a thin outline in the canvas color (`--bg`, or `--cork` on cork).

### Docs

- `docs/design.md` section 6, Strings bullet: the rule and its reason. Section 7: the field on a string in a board file.
- `docs/plan.md`, Phase 7b: one sentence, schema version 8.
- `docs/checklist.md`, Boards: one new unchecked line for the switch and for reading the bare label on four backgrounds in both schemes. Line 96 ("A line's label is horizontal in a pill...") stays as written for relation lines.

## Tests

Core, named by behavior. Each maps to a done criterion of T3 in `.claude/tasks.md`.

| Test | File | Done criterion | With the stub layer |
|---|---|---|---|
| `a_new_string_has_its_label_background_and_the_choice_is_kept_per_string` | `core/tests/behavior/boards.rs` | 1, 2 | fails: turning it off reads back on |
| `a_change_to_a_strings_label_or_color_keeps_its_label_background` | `boards.rs` | 2 | fails: the off value is not kept |
| `a_strings_label_background_survives_closing_and_reopening_the_vault` | `boards.rs` (temp directory) | 2 | fails |
| `a_version_7_vault_migrates_with_every_label_background_on` (a version 7 file built with SQL, one labeled string; after opening: the string's ends, label, and color are unchanged, the value is on, the string still goes when its note is deleted, and turning the value off reads back off) | `core/tests/behavior/schema.rs` | 2 | fails at the last step |
| `a_strings_label_background_round_trips_through_export_and_import` (one string off, one on; guide scope and vault scope; import as new; both values compare equal) | `core/tests/behavior/export_import.rs` | 2 | fails |
| `a_board_file_names_the_label_background_only_when_it_is_off` (reads the board file out of the archive) | `export_import.rs` | 2 | fails |

Two existing, approved tests must change for the code to compile or pass (decision 35):

- `core/tests/behavior/boards.rs`, about line 650: a `BoardStringUpdate` literal that names every field. It gains `..BoardStringUpdate::default()`.
- `core/tests/behavior/schema.rs`, `a_version_6_vault_migrates_to_7_with_nothing_trashed`: it asserts `SCHEMA_VERSION == 7` and `schema_version() == 7`. As made: the two lines became one, `schema_version() == SCHEMA_VERSION`, because comparing `SCHEMA_VERSION` with itself asserts nothing. The new version 7 test does not pin the number 8 either.

UI: no new pure logic is planned, so no new Vitest test is planned. If the bare label needs a width or position rule in `ui/src/lib/board.ts`, it gets a test in `board.test.ts`.

## Done criteria, and how each is checked

1. Switch in the selected string's section, on by default: core test 1; seen in a window (needs the walkthrough or the owner's eyes).
2. Survives a reload and an export and import: core tests 2 to 6.
3. The bare label reads on dots, grid, cork, and blank, in Light and in Dark, or the task says where it does not: contrast figures computed from the tokens in `ui/src/app.css`, written into T3's Notes; then seen in a window. Not judged until then.
4. `docs/design.md` section 6 records the rule: read the diff.

## Stops

1. Now: this plan and decisions 33 to 36.
2. After the core tests are written and fail: the tests are shown; approval; commit.
3. After the build, the gates, and the `reviewer`: who looks at the window (the agent takes the keyboard and mouse and adds steps to `notes.ps1`, or the owner checks by eye).

## Risks

- The archive gains a field. An older build ignores a field it does not know and draws the pill. No build has been released, so no older build exists outside this machine.
- T7 (recipes in core) also needs a migration. T3 is built first and takes 0008.
- `tools/walkthrough/seed/` builds against `core` outside the workspace. It calls `create_board_string` only, whose signature does not change.
- A bare label on cork sits on the grain, which is darker and lighter in patches. The token contrast is high (about 7 to 1 in Light and about 9 to 1 in Dark, text on the flat cork color; figures by hand, to be computed again during the build). The grain cannot be computed; it is judged in a window.

## Guesses

Small choices made here without asking. Each can be reversed.

- Names: the column, the struct field, and the archive field are all `label_background`.
- The switch sits between the color swatches and "Delete string".
- The switch is shown and enabled when the string has no label.
- A new small component, `Switch.svelte`, instead of a copy of the track's CSS in `BoardPanel.svelte`.
- A bare label drops the colored left edge that the pill has. The label sits on its own string, so the edge adds nothing there.
- The flip shows no notice and has no Undo; flipping the switch back is the undo.
- No palette command for the switch.
- `docs/plan.md` gets one sentence under Phase 7b, not a new section.

## Open questions

None. Decisions 33 to 36 were answered (a) on 2026-11-23.

## Progress

- [x] Plan approved (decision 36), 2026-11-23
- [x] Core tests written, failing, shown, approved, committed. Written and failing as planned 2026-11-23 (6 fail at their own assertions, 182 older tests pass); approved by decision 37.
- [ ] Core built; the approved tests pass unedited
- [ ] UI built
- [ ] Docs updated
- [ ] Gates run; `reviewer` called
- [ ] Window check (walkthrough or owner)
- [ ] `tasks.md` wrapped up

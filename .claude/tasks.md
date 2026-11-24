# Tasks: Field Guide

<!--
Lives at .claude/tasks.md. The near-term queue inside the current phase of
docs/plan.md, and the handoff between sessions.

CUTOVER prompt (old session, once):
  "Read .claude/tasks.md. Fill in every <fill in> from what you know: current
   phase, each task's goal, inputs, and done criteria, and the Carry-over
   section. Do not change code. Check your account against git log and
   git status. Tell me anything you know that has no place in this file."

TASK SESSION prompt (fresh session each time):
  "Read CLAUDE.md and .claude/tasks.md. Do task T<n>."
-->

## Current phase

Between numbered phases. Phases 0 to 8 and 7b are built. The section
"Dark mode and tokens" in docs/plan.md (it sits between Phase 10 and Later)
is built. Phase 9, "Import and export UI", is the next numbered phase and is
not started.

Gates that remain:

- "Dark mode and tokens": the Theme section of docs/checklist.md must pass in
  both schemes. T1 walked it through on 2026-11-12 and I checked the rest
  by eye on 2026-11-13: 13 lines passed, 2 failed (the theme pictures; the
  caption on picture cards in Dark). The gate stays open until the 2
  failures are fixed.
- Phase 7: "a board with a few hundred cards stays responsive while dragging"
  was never checked. docs/plan.md says it waits for a walkthrough with a
  generated guide.
- The review follow-up work (commits 7dea8ff, 8b15673, bd5092f, f5cf5a1,
  ce8a1cc) has no section and no gate in docs/plan.md. Its checks are lines
  in docs/checklist.md. T1 marked the Theme section only.

Gates last run at 0286726 (T2, after the review fixes): 31 unit, 176
behavior, 176 UI tests, clippy clean, format clean, svelte-check clean. The
schema version is 7. No release build was made for T2; the last one is of
ce8a1cc, so `tools/walkthrough/run/t1/` holds a build without the Trash for
boards and guides. For the current head, run `git log --oneline -5`; this
file cannot name the commit that holds it.

Gates last run on 2026-11-23 at the end of T2: 31 unit, 182 behavior, 183
UI tests, clippy clean, format clean, svelte-check clean. The last release
build is of 646332c, in `tools/walkthrough/run/t2/`; no code in `core`,
`app`, or `ui` changed after it.

T2 is done. I validated its two not judged checks on 2026-11-23.

Gates last run on 2026-11-23 at cce45e0 (T3, built): 31 unit, 188 behavior,
184 UI tests, clippy clean, format clean, svelte-check clean. The schema
version is 8. The last release build is of cce45e0's code, made 2026-11-24,
in `tools/walkthrough/run/t3/`; no code in `core`, `app`, or `ui` changed
after it.

T3 is done. I validated its not judged check on 2026-11-24 (Tab reaches
the switch, the focus ring shows, Space and Enter flip it).

## Carry-over from the long session

Things that were only in the conversation.

### The portable test build

- Build: from `app/`, run `cargo tauri build --no-bundle`. It needs the Tauri
  CLI. A plain `cargo build --release` loads the dev URL and shows an error
  page. Output: `target/release/fieldguide.exe`.
- Before copying: run `tasklist` and look for `fieldguide.exe`. The app is
  single-instance, and a running copy locks its exe.
- Run folder: the exe is copied into a folder that holds an empty file named
  `portable`, so the test vault lives beside the exe and never touches the
  real data directory.
- Where it is now: `tools/walkthrough/run/` in the repository, ignored by
  git (moved 2026-11-12). `run/t1/` is the folder T1 used: a build of
  ce8a1cc made 2026-11-12 and the vault the T1 runs left (8 guides).
  `run/` itself still holds the older copy and its vault (19 guides). The
  scripts need neither vault: each makes its own guide.
- Screenshots of the T1 runs: `tools/walkthrough/run/shots-ce8a1cc/`, by
  scheme and script, plus `restart/` and `dark/startup/`.
- Screenshots of the last clean runs, at 19b8dae, are in
  `tools/walkthrough/run/shots-19b8dae/` (`drive`, `board`, `notes`,
  `paste`).
- `tools/walkthrough/README.md` has the build and run steps.
- To start in Dark without clicking: add `theme = "dark"` under `[ui]` in the
  run folder's `config.toml`.

### The walkthrough scripts

- Where they are: `tools/walkthrough/` in the repository (`drive.ps1`,
  `board.ps1`, `notes.ps1`), committed 2026-11-12. Since 2026-11-23 a
  fourth, `trash.ps1`, covers the T2 screens. It needs the vault that
  `tools/walkthrough/seed/` writes, a new one before every run (README).
  Last clean run of `trash.ps1`: 2026-11-23 against the build of 646332c,
  in Dark and in Light, 0 misses each. Last clean run of `notes.ps1`:
  2026-11-24 against the build of cce45e0, in Dark and in Light, 0 misses
  each; it has steps 13 to 19 for T3 since then.
- What they are: PowerShell UI Automation. Each takes `-Exe`, `-OutDir`,
  and `-Theme dark|light`, drives the window, and saves numbered
  screenshots. Rewritten whole in T1; `tools/walkthrough/README.md` lists
  what each covers and how to read a run.
- How to run: `powershell.exe -STA -File <script> -Exe <exe> -OutDir <dir>`.
  The files must be CRLF with a UTF-8 byte order mark, or Windows PowerShell
  5.1 misreads them. `.gitattributes` keeps `*.ps1` as CRLF.
- Last clean run: 2026-11-12 against the build of ce8a1cc, in Dark and in
  Light, 0 misses each.
- Not covered by any script: a file drop (UI Automation cannot send one),
  Tag merge confirm, the Trash keys, the Edit qualifiers suggestions,
  resizing a note, the browser launch after "Open in browser", switching
  Windows between light and dark, and the two restart checks of the Theme
  section (T1 ran those from a one-off script that is not committed).
- Script lessons: never call UI Automation SetFocus on the note editor (a
  blur saves and closes it). Menu rows with `role="menuitemradio"` are
  RadioButton to UI Automation. Confirm dialogs with their buttons, never
  with Enter. Wait in a retry loop for the first screen (the scripts wait up to 15
  seconds).

### Standing rules not in CLAUDE.md or AGENTS.md

- Push: every commit so far was pushed straight to `main`, no branches and
  no pull requests. Decided 2026-11-12: push before a new session starts
  (see Decisions). `.claude/settings.json` allows `git add` and
  `git commit` but not `git push`, so a push asks for permission.
- Commit messages end with
  `Co-Authored-By: Claude Fable 5.2 <noreply@anthropic.com>`.
- American English in UI copy, docs, and comments (color, center).
- Windows toolchain problems (a missing tool, a blocked folder): stop and
  report. Never work around a failing gate.
- Subagent models: set in the agent files on 2026-11-12. `explorer` runs on
  Sonnet; `reviewer` inherits the main model.
- The reply format in `C:\Users\DWats\.claude\CLAUDE.md` (Status, What
  changed, Decisions needed, Guesses, Next, Details) differs from the one in
  AGENTS.md, which adds Concerns.
- Check boxes work in board notes only. Entry bodies keep inert ones. This
  one is in docs/design.md section 6.

### Tooling lessons

- Vitest reports "N failed, no tests" with
  `Cannot read properties of undefined (reading 'config')` when the Bash
  shell starts inside `ui/` with a lowercase drive letter. Run
  `cd / && cd /c/Users/DWats/Projects/field_guide_app/ui && npm test`.
- Heredocs in the Bash tool mangle backslashes. Write a patch script with
  the Write tool and run it with `python -I`.
- The content security policy in `app/tauri.conf.json` must keep
  `ipc: http://ipc.localhost` in `default-src`, or pasted images fail.
- New dependencies added in this stretch: `tauri-plugin-opener` (called from
  Rust only, not registered), `@codemirror/autocomplete`, `@lezer/highlight`.

### Reference files

In the repository (moved 2026-11-12):

- `docs/reviews/ux-2026-11-10.md`: the UX review. "UX review" in this file
  means this one. docs/ux-review.md is the older 2026-11-08 review, which
  docs/design.md cites by that path.
- `docs/reviews/ui-2026-11-10.md`: the UI design review. "UI review" in
  this file means this one.
- `.claude/work/trash-boards-guides/design.md`: the Trash design for T2.
- The screenshots both reviews cite (`shots-drive`, `shots-board`,
  `shots-notes`) are in `tools/walkthrough/run/shots-19b8dae/`, on this
  machine only.

History, outside the repository, in
`C:\Users\DWats\.claude\projects\c--Users-DWats-Projects-field-guide-app\memory\reports\`:

- `plan-dark-mode-2026-11-10.md`: the dark mode plan, built.
- `theme-contract.md`, `ux-fixes-contract.md`, `wave2-contract.md`,
  `board-decorations-contract.md`, `board-images-contract.md`: the briefs the
  subagents built from.

### Known rough edges and deferred fixes

- T1 saw ce8a1cc in a running window on 2026-11-12, in both schemes. What
  it found is in the Inbox; what it could not judge is in T1's Notes.
- The list "the subagents asked for a look at" is answered item by item in
  T1's Notes.
- Backups hold no media. Purge, then media cleanup, then a backup restore
  loses image files. Settings does not say so.
- Wikilinks inside board notes are not rewritten when an import renames a
  colliding entry. Entry bodies are.
- Board notes have no `[[` suggestions.
- `Modal`'s `dirty` prop is set on the Tag editor dialogs, Edit guide, and
  Edit qualifiers only. New entry, New guide, and the board name dialog
  still close on a backdrop click with typed text.
- A new tag made inside the Edit guide dialog is created before Save.
- An unknown `ui.theme` or `ui.density` value in `config.toml` fails the
  config load (docs/design.md section 11).
- Phase 8 review leftovers: 16-bit image sources are documented, not kept;
  no SVG cover round-trip test.
- The Trash design was written at 06b6638. Three things it lists are already
  built: Add all with Undo, trashed entries refused by `add_board_nodes`,
  and confirm dialogs starting on Cancel.

### Guesses nobody has confirmed

Each was reported once under Guesses. None was answered.

- "Dim images in dark" is off by default, 85 percent brightness, never in
  the image viewer.
- A hidden spoiler is a mid-gray block in Light too.
- Every line on a board has the label pill.
- A web link that is not http or https does nothing except a notice.
- A body of only spaces counts as empty for the stub rule. Emptying a body
  later does not make the entry a stub again.
- A dashed link clicked in a note creates the entry, with Undo.
- A single card added from the picker shows no notice.
- Images dropped on the guide page outside the cover start one new entry.
- The zoom cluster sits one notice row above the canvas's bottom edge.
- The board panel is two fixed halves, selection above, picker below.
- Control+R on a board with no card selected does nothing.
- The string pin is a red push pin. The tying hint is a banner at the top.
- `[[` suggestions match the start of a title or alias only. With nothing
  typed, the list shows the first 8 titles.
- The Vocabulary link is in the sidebar footer. Deleting an unused word asks
  first and has no Undo. Symmetric cannot be turned on while an inverse is
  set.
- Boards sits above Trash in the tree. The widen row reads "No more in All
  entries" and is disabled at zero.
- `add_board_nodes` refuses the whole call when one entry is in Trash.

## Tasks

Status values: `todo` | `in progress` | `blocked` | `done` | `dropped`

### T1: Walkthrough of ce8a1cc in Dark and Light
- **Status:** done (walkthrough 2026-11-12; I validated the not judged
  checks on 2026-11-13)
- **Goal:** See everything built since 19b8dae running, in both schemes, and
  list what is wrong. Close the open gate of "Dark mode and tokens".
- **Depends on:** none
- **Inputs:** walkthrough scripts (update first; they predate ce8a1cc);
  docs/checklist.md; Carry-over above (build recipe, stale list, the list of
  things to look at)
- **Done when:**
  - [x] The three scripts are updated for the current controls and run to
        the end against a release build of the commit under test.
  - [x] Screenshots exist in Dark and in Light of: the guides list; a guide
        page; an entry in Read with every markdown feature; the entry in
        Edit; the Tag editor; Trash; Vocabulary; Settings, Appearance; the
        palette; a confirm dialog; a board on dots; a board on cork with
        notes, strings, an index card, and a polaroid; the board panel's
        Entry and Relations tabs; the image viewer.
  - [x] Every line of the Theme section of docs/checklist.md is marked
        passed or failed. (13 passed, 2 failed, after my checks on
        2026-11-13.)
  - [x] Each item under "The subagents asked for a look at" is marked fine
        or reported.
  - [x] Every defect found is one line in the Inbox.
- **Out of scope:** fixing what it finds; the few-hundred-cards check
  (Later); the facts box (T5).
- **Notes:**
  - The gate of "Dark mode and tokens" is not closed. Theme lines 1 (theme
    pictures) and 11 (picture card caption in Dark) failed.
  - The checks the agent could not judge, validated by me on 2026-11-13:
    1. System: switching the Windows 10 app mode switches the Field Guide
       theme. Passed.
    2. Light with Windows in dark mode: the app and the title bar are
       light. The opposite holds too. Passed.
    3. "Open in browser" opens the browser. Passed. It only recognizes
       fully qualified addresses such as `http://google.com` (Inbox).
    4. Escape in a board note: I entered an existing note, typed a word,
       pressed Escape; it looks correct. Passed.
    Items 3 and 4 are parts of larger checklist lines (Entries, Boards)
    that T1 did not run in full, so those lines stay unchecked.
  - No white frame at startup: passed. Captures about 30 ms apart showed
    none, and I looked and saw none (decision 15).
  - "Dark with Windows in light mode" stays passed (decision 14). Windows
    was already in light app mode during the runs.
  - The reviewer found no blocker. Its open points were decisions 14 to 16
    (answered 2026-11-13) and Inbox lines. The last two script edits (the "Theme: System"
    step only with `-Theme`; the focus-loss message) were parsed, not run.
  - The list to look at. Fine: dark title bar and no white frame at
    startup; editor caret, selection, and gutter in Dark; image dimming
    (85 percent in Dark, none in Light, never in the viewer); the notice
    against the zoom cluster; cork grain; label pills on cork; pin
    placement on notes; alert icon alignment; drawn task boxes in Light;
    segmented controls, switches, and tiles in both schemes; the editor's
    height in Edit; the `[[` list's position; thumbnails in media cleanup;
    long titles (they wrap; the breadcrumbs shorten the guide names).
    Reported in the Inbox: the theme pictures; the top bar with the sidebar
    folded; the caption on picture cards in Dark; the header controls with a
    long title move between Read and Edit. Checked by me on 2026-11-13:
    Escape in a note (no double save); the browser launch.
  - What changed: the three scripts were rewritten whole with one helper
    block copied into each; `tools/walkthrough/fixtures/markdown.md` is the
    body with every markdown feature; the README describes coverage and how
    to read a run. No code in `core`, `app`, or `ui` changed.
  - Plan and run log: `.claude/work/t1-walkthrough/plan.md`.
  - Commit: beb7166 (scripts, fixture, checklist marks, this entry). Gates
    ran the same day with no code change since ce8a1cc: 31 unit, 151
    behavior, 159 UI tests, clippy clean, format clean, svelte-check clean.
  - For the next task: `run/t1/` holds the build and a vault with 8 guides.
    A script refuses to start while `fieldguide.exe` runs.

### T2: Trash for boards and guides
- **Status:** done (built, reviewed, and walked through in Dark and Light
  on 2026-11-23; I validated the two not judged checks the same day)
- **Goal:** Deleting a board or a guide sends it to Trash with Undo in the
  notice, the way deleting an entry does. Purge is the one step that cannot
  be reversed; it asks, and it states what is lost with counts.
- **Depends on:** decisions 5 to 11. No task. T1 first is safer, because
  this adds to screens nobody has seen running.
- **Inputs:** docs/design.md section 3 (Trash), section 6 (the line
  "deleting a board asks first, since the entries stay and there is nothing
  else to reverse" is out of date), sections 7 and 8;
  `.claude/work/trash-boards-guides/design.md`, which has the schema, the
  core API, the rules, the tests by name, and four build batches; UX review
  failing item 9
- **Done when:**
  - [x] Migration 0007 adds `deleted_at` to boards and guides; the schema
        version is 7; existing vaults migrate with nothing trashed.
  - [x] A trashed guide hides its entries, relations, qualifier values, and
        board cards from every other guide, and restore returns all of it,
        positions included.
  - [x] A trashed board keeps its notes, images, and strings; restore
        returns them; media cleanup keeps its images while it is in Trash.
  - [x] Nothing can be created in a trashed guide, and writes to a trashed
        board are refused.
  - [x] Export, import, and name rules follow decisions 5 to 7, each with a
        round-trip test.
  - [x] Delete board and Delete guide show a notice with Undo and no dialog.
        (Seen in a window in both schemes, 2026-11-23.)
  - [x] Trashed boards and guides are listed where decisions 8 and 9 say,
        with Restore and Delete for good; purge names the counts core
        computed. (Seen in a window in both schemes, 2026-11-23.)
  - [x] Empty Trash and restore follow decisions 10 and 11.
  - [x] docs/design.md sections 3, 5, 6, 7, and 8 are updated.
- **Out of scope:** decision 12 (second import of a board); backups and
  media.
- **Notes:** The design's named tests are the starting list for the tests
  AGENTS.md asks for first. `empty_trash` changes its return type, so the UI
  and its tests change in the same batch.
  - Plan: `.claude/work/trash-boards-guides/plan.md`. It lists what differs
    from the design, the tests per batch, and a Progress list.
  - Built 2026-11-13 in four batches. Commits: 97f4ee7 and 9f784cd (guides
    in core), a490e6e and 2990cb3 (boards in core), 6205ed9 and d015c94
    (export and import), ca94215 (commands, UI, docs). The approved core
    tests were not edited while each batch was built.
  - Review 2026-11-13 (`reviewer`): no blocker, two major and seven minor
    findings. Fixed at once: a paste or a drop on the entry page of a
    trashed guide wrote to the body; a misplaced doc comment; the Empty
    trash dialog said "the entries stay" while trashed entries go. Open:
    decisions 24 to 27. Three more are Inbox lines.
  - Nothing in the UI was seen in a window. Done criteria 6 and 7 are built
    and stay unchecked until the checklist lines are judged.
  - Done criteria 1, 2, 3, 4, 5, 8, and 9 are met and tested as of 0286726.
    Their boxes get checked at wrap-up, after Batch 5.
  - `docs/design.md` section 3 already states the rule of decision 24. The
    code does not follow it yet: only the guide page and the entry page of
    a trashed guide say so today.
  - Batch 5, part A, built 2026-11-13 (e4dc7c8): one component,
    `TrashedGuide.svelte`, shown by the seven pages inside a guide when the
    guide they loaded is in the Trash; no guide sidebar there. UI only.
    svelte-check clean, 176 UI tests pass. Not seen in a window: one new
    unchecked line in `docs/checklist.md`, Guides section. Its guesses are
    in the plan file under Part A.
  - Batch 5, parts B, C, D, built 2026-11-13 (c7de1e0). The 4 approved
    tests of f2b1f9e pass and were not edited. `guide_contents` and the new
    `guide_trash_contents` share one counting function over a set of guides;
    the purge dialog has two new lines; Empty trash on the guides list
    states core's combined count; a new qualifier label is refused through
    a relation of a trashed guide; a verb's "in Trash" count follows the
    hidden rule. `docs/design.md` section 3 has four new bullets and
    `docs/checklist.md` two new unchecked lines. The guesses are in the plan
    file's Progress list.
  - Review of Batch 5, 2026-11-13 (`reviewer`, diff 0286726..c7de1e0):
    parts B, C, and D are clean, and the approved tests are unchanged. One
    major finding in part A: decision 29. One question about tests:
    decision 30. Three minor findings are Inbox lines. Nothing was changed
    after the review except this file, the plan file, and the walkthrough
    tooling.
  - The T2 walkthrough is prepared, not run. `tools/walkthrough/seed/` is a
    small program (outside the workspace, so outside the gates) that writes
    a vault with three guides tied across guides; the app has no control
    that makes those ties. `tools/walkthrough/trash.ps1` is a fourth script
    for the T2 screens; it checks the dialog lines against the counts the
    seed prints. The seed was built and run against a scratch vault. The
    script was parsed by Windows PowerShell with 0 errors and was never
    run. No release build was made.
  - Decision 29, answer (a), built 2026-11-23 (1d9bb59, then the review
    fixes in the next commit). `guideMoved` in `ui/src/lib/trash.ts` asks
    core for the guide a page loaded. The Tag editor, Vocabulary, and the
    entry page call it on every change and load again when the guide went
    into the Trash or came out. The board page asks for its guide on every
    load. `TrashedGuide` lost its `onrestored` prop: Restore reaches the
    page through the same check. No banner and no disabled controls: see
    Decisions, 2026-11-23.
  - Review of the decision 29 fix, 2026-11-23 (`reviewer`): approved, five
    minor findings, all fixed the same day: a check that a newer change or
    the page's end overtook no longer loads; the board page keeps its guide
    object while nothing about the guide's place changed, so the panel does
    not reload its lists on every board edit; Restore on the Trash message
    loaded twice; one more guard on the board load; a read that fails for
    another reason than "not found" no longer counts as moved. One concern
    is an Inbox line: `EntryView.load` has no newest-request guard.
  - Decision 30, answer (a): two pin tests are in the working tree in
    `core/tests/behavior/trash.rs`, passing, not committed:
    `a_typed_label_that_names_an_existing_one_is_an_edit_in_a_trashed_guide`
    and
    `a_string_to_an_image_counts_only_when_its_other_end_is_a_card_of_the_guide`.
    They add 98 lines and change no existing test. Decision 31 asks for
    approval.
  - The walkthrough is ready and not run. `tools/walkthrough/run/t2/` holds
    a release build made 2026-11-23 from the commit with the review fixes,
    an empty `portable` file, and a vault from the seed. `trash.ps1` gained
    step 7b for decision 29 (Undo of a restore, clicked on the Tag editor,
    Vocabulary, an entry, and a board); parsed with 0 errors, never run.
    Nothing was launched.
  - Decisions 31 and 32 answered (a) on 2026-11-23. The pin tests are
    committed (48639b7).
  - Last review, 2026-11-23 (`reviewer`, the review fixes, the pin tests,
    and the walkthrough tooling): `ui` and `core` approved. One major
    finding in the tooling, fixed the same day: `trash.ps1` now refuses to
    start without a file named `portable` beside `-Exe`, because it
    confirms Purge and Empty trash. The other three scripts still have no
    such check (decision 16, Inbox line).
  - The T2 walkthrough, 2026-11-23: `trash.ps1` against the build of
    646332c, Dark and Light, 0 misses each, 60 screenshots each in
    `tools/walkthrough/run/shots-646332c/`. The script checks every dialog
    line by its text against the counts the seed prints. No app defect was
    found. Three earlier runs stopped on script errors; each was a wrong
    guess of the agent's about the app, and the plan file's Progress list
    names them. What the runs showed that the checklist did not say: Undo
    from the Boards list puts the board back in the list and does not open
    it; Move to Trash on a board's own page takes that page out of Back.
    The Boards line of `docs/checklist.md` now says both.
  - Checked in `docs/checklist.md` after the runs: the guide's Trash with
    boards; the guides' Trash; what a guide purge takes from other guides
    and the combined Empty trash count; Back to a page inside a trashed
    guide; a page that is open when its guide goes to the Trash; the
    Vocabulary count.
  - Not judged by the agent, validated by me on 2026-11-23. Both passed,
    and both lines are checked in `docs/checklist.md`:
    1. New guide's "Copy verbs and qualifiers from" list does not offer a
       guide that is in the Trash. UI Automation cannot read the options of
       a closed list. It is the one open part of the Guides line "More,
       Move guide to Trash"; every other part of that line was seen.
    2. The Boards line with Rename, the picker, and Move to Trash: its T2
       parts were seen (Move to Trash from the page and from the list, Undo,
       the refused name, Back to a trashed board). Its other parts (the
       picker's search line, chips, and Stubs only after reopening; Rename)
       are `board.ps1`'s and were not run in this task, so the line stays
       unchecked.
  - For the next task: `run/t2/` holds the build of 646332c and the vault
    as the last run left it (Valheim only). Seed a new vault before any
    run of `trash.ps1`.

### T3: Pill toggle on lines
- **Status:** done (built and reviewed 2026-11-23, walked through in
  Dark and Light on 2026-11-24; I validated the not judged check the same
  day)
- **Goal:** Let me turn the pill background behind a line's label off for
  one line, so a busy board stays readable. Today every line has the pill.
- **Done when:**
  - [x] The selected line's section in the board panel has a "Label
        background" switch, on by default. (Seen in a window in both
        schemes, 2026-11-24.)
  - [x] The choice survives a reload and an export and import. (Core
        tests 2 to 6.)
  - [x] With the pill off, the label still reads on all four backgrounds in
        both schemes, or the task says where it does not. (Seen
        2026-11-24; the Notes say where it reads less well.)
  - [x] docs/design.md section 6 records the rule. (Section 7 records
        the archive field.)
- **Notes:** Under (a) the choice is a new column on `board_string` and a
  new field on a string in the archive's board file. That is a migration and
  a small format addition; the session's earlier summary of (a) did not say
  so. Under (b) relation and mention lines need new storage per board,
  because those lines are derived and have no row. Asked in my words:
  "(a) optional pill with background, toggle per line".
  - Plan: `.claude/work/t3-pill-toggle/plan.md`, written 2026-11-23 at
    6840717. It has the readings, the six core tests by name, the stub
    layer, the guesses, and a Progress list. Migration 0008, schema
    version 8.
  - Found while planning: Undo of a deleted string ties it again with its
    label and color only (`retie` in `ui/src/views/BoardView.svelte`). The
    plan makes it restore the new value too.
  - Core tests written 2026-11-23, approved and committed the same day: 3
    in `core/tests/behavior/boards.rs`, 1 in `schema.rs`, 2 in
    `export_import.rs`. With the stub layer all 6 fail at their own
    assertions and the 182 older behavior tests pass; clippy and format
    are clean. The stub layer: `BoardString.label_background` (always
    read as `true`) and `BoardStringUpdate.label_background` (ignored) in
    `core/src/board.rs`, one line in `core/src/import.rs`. No migration
    yet; the schema version is still 7.
  - The two edits of decision 35 are made. In `schema.rs` the two version
    lines became one, `schema_version() == SCHEMA_VERSION`, because
    comparing `SCHEMA_VERSION` with itself asserts nothing. The new
    version 7 test does not pin the number 8, so T7's migration will not
    need to edit it.
  - Built 2026-11-23 (cce45e0; the tests are c6742e0). Migration 0008
    adds `board_string.label_background`, default 1; the schema version
    is 8, so T7 takes 0009. The approved tests were not edited during the
    build. The archive field is written only when off; `format_version`
    is still 1.
  - UI: `Switch.svelte` is new (a `role="switch"` button, label left,
    track right). The panel's string section has the switch between the
    swatches and "Delete string". A bare label is `.pill.bare` in
    `BoardCanvas.svelte`: no box, nine text shadows in `--canvas`, a new
    custom property that is `--bg`, or `--cork` on cork. Undo's rule is
    `stringRestore` in `ui/src/lib/board.ts`, with a Vitest test.
  - Done criterion 3, by figures only: the plan file has the contrast
    table. Text against every background is 4.4 to 1 or better. Text
    against the twine itself is 1.0 to 3.4 to 1, so the outline is what
    keeps the letters apart from the string. Not judged until seen in a
    window; `docs/checklist.md`, Boards, has one new unchecked line.
  - Review 2026-11-23 (`reviewer`, diff 6840717..cce45e0): approved, no
    blocker and no major finding. It read every place that reads or
    writes a string row and found no path that loses the value; it
    confirmed the approved tests were not edited by the build, and ran
    every gate itself with the same counts. Four minor findings, none
    fixed: a test gap (decision 39) and three Inbox lines (Undo ties a
    string in two calls; the string section may not fit a 400-pixel-high
    window; High Contrast mode drops the outline). For the window check it
    asks for: the black string in Light at the smallest zoom, and whether
    the outline shows as a halo on cork's grain.
  - Not checked: `tools/walkthrough/seed/` was not compiled against the
    new core (it calls `create_board_string` only, whose signature did
    not change).
  - The T3 walkthrough, 2026-11-24 (decision 38 (a)): `notes.ps1` gained
    steps 13 to 19 and ran against a release build of cce45e0 in
    `tools/walkthrough/run/t3/`, Dark and Light, 0 misses each, 30
    screenshots each in `tools/walkthrough/run/shots-cce45e0/`. The script
    reads the switch through UI Automation and counts a wrong state as a
    miss: on for a new label; off after a click; still off after the
    board is reopened; still off, blue, and labeled after Delete and
    Undo; on again after a second click.
  - Where the bare label reads, judged from the screenshots and from
    crops enlarged four times. At 100 percent zoom all four colors read
    on Dots, Grid, Blank, and Cork in both schemes; the outline cuts the
    string beside the letters, the black string in Light included, and on
    cork it does not show as a halo. At 63 percent (Fit for the test
    board) the labels still read, but the outline is about 1 pixel, the
    string shows between the letters, and a pill reads better; the
    weakest case is Cork in Light. At 37 percent no label reads, with or
    without a pill.
  - Found in the runs, both Inbox lines: two labels without pills at one
    place are drawn over each other and neither reads (a pill covers the
    one under it); the outline gets thinner as the board zooms out.
  - Runs that did not count: the first launch in the new folder took
    longer than the script's 15 seconds for the guides list; then one
    Dark run lost the Enter that ties the first string in step 7, an
    older step, and every later step that needed that string missed. The
    next run was clean and the cause was not found (Inbox line). A third
    Dark run followed a script change: step 13 now moves the image so no
    two labels share a spot.
  - Not judged by the agent, validated by me on 2026-11-24: the switch
    from the keyboard. I selected a string, clicked a color in the panel,
    pressed Tab, and focus went to the switch; Space and Enter flip it.
    Passed, and the line is checked in `docs/checklist.md`. The focus
    ring was present; the agent had split it into its own line, and that
    line is checked too. With a string selected and no click in the panel, Tab moves to
    other board items and not into the panel (Inbox line).
  - For the next task: `run/t3/` holds the build of cce45e0 and a vault
    with four guides. The next migration is 0009.

### T4: Narrow windows
- **Status:** todo (decision 4 answered (a) on 2026-11-12)
- **Goal:** Decide what the app does when the window is narrow. Today
  nothing adapts: at the 640-pixel minimum the content column gets about 60
  pixels between the sidebar and the panel.
- **Done when:**
  - [ ] Under (a): mock-ups at 1100, 860, and 400 pixels wide of the entry
        page, the guide page, and a board, and I have chosen what to build.
        Nothing is built in this task.
  - [ ] Under (b): a scope note for a phone build (touch, no hover, files,
        packaging), and I have chosen.
  - [ ] Under (c): the task is dropped and docs/design.md records it.
- **Notes:** UI review failing item 27 proposed folding the panel to its
  rail below 1100 pixels and the sidebar below 860, with the saved choice
  untouched. My words: "We should think about what this would look like on a
  phone if we're going to think about narrow displays." `minWidth` is 640 in
  `app/tauri.conf.json`.

### T5: Facts box mock-up
- **Status:** todo
- **Depends on:** T1
- **Goal:** Show me a facts box (infobox) at the top of an entry in Read, so
  I can decide whether to build it. It floats right at the top of the body
  and holds the primary image with its caption, the tags, and the relation
  groups as label and value rows.
- **Done when:**
  - [ ] A mock-up of the Stone Block entry with the box, in Light and in
        Dark, using the app's real tokens.
  - [ ] It also shows an entry with no image and an entry with many
        relations.
  - [ ] It says what the box does to the right panel's Relations tab.
  - [ ] I have chosen build or not, and docs/design.md records it.
- **Notes:** UI review, Missing item 1; decision 9 there was answered "mock
  it first". Effort if built: medium in the UI, large if core renders it for
  export. Nothing is built in this task.

### T6: Recipes: design and mock-ups
- **Status:** todo
- **Goal:** Settle how a recipe is stored, exported, placed in a body, and
  drawn, before any code. Show me mock-ups so I can choose. Nothing is built
  in this task.
- **Depends on:** none. It starts from my answers under Decisions,
  2026-11-23, the three lines that begin "Recipes".
- **Inputs:** `.claude/work/recipes/research.md`; the three Decisions lines
  of 2026-11-23 that begin "Recipes"; docs/design.md section 3 (Trash,
  Media), section 4, section 5 ("Markdown dialect"), sections 7 and 8; T5,
  because a facts box may hold a recipe row, as the Factorio wiki's does.
- **Done when:**
  - [ ] `.claude/work/recipes/design.md` describes the recipe record: its
        recipe type, an optional name, and slots. A slot has a role (input,
        output, station, a tool that is kept), an amount (a number or a
        range, with an optional unit), an optional grid position, and one or
        more choices, each an entry or a tag.
  - [ ] It describes the recipe type: defined per guide as data, none in a
        new guide, with a name, a layout (grid with a width and a height,
        list, row), and the extra fields its recipes carry (time, skill
        level, and the like).
  - [ ] It describes the marker that places recipes in a body, the rule for
        an entry with recipes and no marker (they show at the end of the
        body), and how "Used in" on an ingredient's page is generated.
  - [ ] It states the rules for Trash, purge, rename, entries of other
        guides, the export file, and the import mapping of recipe types,
        each with the reason.
  - [ ] It names the tests that would gate the core build, by behavior.
  - [ ] It is checked against these cases, and says for each how it is
        recorded or that it is not: a shaped 3 by 3 grid with "any Planks";
        a shapeless recipe; a 9 by 9 grid; smelting with two stations; a
        recipe with a profession, a skill level, and a kept tool; a recipe
        with a time and a list of machines; an alternate recipe with a
        byproduct; a recipe made by hand with no station; an alloy with
        ratio ranges; a barrel recipe with liters and a sealing time; a
        knapping pattern.
  - [ ] Mock-ups in Light and in Dark, with the app's real tokens: a grid
        recipe at 3 by 3 and at 9 by 9; a list recipe; a row recipe; one
        table holding several recipes of one entry, one marked as the
        alternate; "Used in" on an ingredient's page; a slot with a picture,
        a slot without one, and a slot with several choices; the hover
        popup; the editor for a recipe; the editor for a recipe type; the
        pattern editor.
  - [ ] Every layout and control has a generic name. None names a game. I
        have chosen the names, and `CLAUDE.md`, Names, has them.
  - [ ] Each open question under Notes is answered by me.
  - [ ] I have approved the design. It is a new section of docs/design.md
        with its reasons, and docs/plan.md has a section for recipes, placed
        before Phase 9, with the tests that gate it.
  - [ ] T7 and T8 are checked against the approved design, and changes to
        them are proposed to me.
- **Out of scope:** any change in `core`, `app`, or `ui`. The tree, the
  totals, the bill of materials, and recipes on boards (Later; each has its
  own interview). The design must not block them: it says how amounts
  multiply down a chain and where an item's default recipe is kept.
- **Notes:**
  - Decided so far: native, no plugin; a recipe is its own record in core;
    recipes show where a marker sits in the body, or at the end of the body
    without one; layouts are grid, list, and row, and several recipes of one
    entry form one table; a slot shows the entry's picture as an icon, a
    hover popup with details, and a click opens the entry; the first guide
    to serve is Minecraft with modded grid sizes, and Vintage Story is the
    wildcard; a pattern editor is designed here and built after grid and
    list.
  - Open questions the interview did not reach: the marker's syntax; whether
    "Used in" on an entry lists recipes that accept it through a tag; what
    the hover popup shows; where recipe types are managed (a page beside
    Vocabulary is one option); whether a slot may hold an entry of another
    guide; what a recipe shows while one of its entries is in the Trash;
    whether core renders the recipe for the preview and the export, as it
    renders bodies; whether recipes appear in the Relations tab or on
    boards.
  - My pictures of the Satisfactory, Wowhead, and Factorio pages are
    described under Decisions; the files are not in the repository.

### T7: Recipes in core
- **Status:** todo
- **Depends on:** T6, approved.
- **Goal:** Core stores recipe types and recipes, answers "which recipes
  make this entry" and "which recipes use this entry", and carries both
  through export and import without loss. No screen changes.
- **Inputs:** the recipes section of docs/design.md that T6 adds, and its
  named tests; docs/plan.md, the recipes section; `AGENTS.md`, "For changes
  in `core/`" (tests first with stubs, my approval, then the build).
- **Done when:**
  - [ ] The next free migration adds the recipe tables, and an existing
        vault migrates with no recipes and nothing else changed.
  - [ ] A new guide has no recipe types. A recipe type is created, renamed,
        and deleted; a name is unique in its guide without regard to case;
        deleting one in use is refused.
  - [ ] A recipe keeps its type, its name, and its slots: role, amount or
        range, unit, grid position, and the choices in order. Reading it
        back returns the same record.
  - [ ] Several recipes may make one entry, and each keeps its own identity.
  - [ ] "Used in" for an entry comes from the stored recipes, by the rule
        the design sets for choices that are tags.
  - [ ] Renaming an entry or a tag changes no recipe; recipes hold ids.
  - [ ] Trash, restore, and purge of an entry, a guide, and a recipe follow
        the design, with a test for each rule. A new cross-guide
        query is added to the test that reads every list of a live guide.
  - [ ] Export and import round-trip at every scope: recipe types, recipes,
        slot order, amounts, ranges, units, positions, and choices compare
        equal. Recipe types map on import the way verbs do.
  - [ ] Every gate passes, and the `reviewer` has read the diff.
- **Out of scope:** commands in `app`, anything in `ui`, the marker's
  rendering if the design puts it in T8, the tree, the totals, the bill of
  materials.
- **Notes:** T3 also needs a migration. Whichever task is built first takes
  the next number. The archive gains files, so `format_version` follows
  whatever the design decides.

### T8: Recipes on the entry page
- **Status:** todo
- **Depends on:** T7; the mock-ups I chose in T6.
- **Goal:** I can define a recipe type, record a recipe on an entry, and
  read it on the output's page and on each ingredient's page, as a grid, a
  list, or a row.
- **Inputs:** the mock-ups and names chosen in T6; the recipes section of
  docs/design.md; docs/design.md section 5, "Interface principles"
  (recovery, focus, controls, empty states); `RelationBuilder.svelte`, for
  the suggestion and stub pattern.
- **Done when:**
  - [ ] A recipe type can be created, renamed, and deleted where the design
        says, with its layout and, for a grid, its width and height.
  - [ ] A recipe can be added to an entry, edited, and deleted. Delete shows
        a notice with Undo and no dialog.
  - [ ] A slot's field suggests entries and tags. A name that matches
        nothing follows the design's rule (a stub, as the relation line
        makes, or text).
  - [ ] The grid, list, and row layouts match the chosen mock-ups in Light
        and in Dark. Several recipes of one entry form one table.
  - [ ] A slot shows the entry's primary image with the amount, or the title
        when the entry has no image. Hover or keyboard focus shows the
        popup. A click or Enter opens the entry.
  - [ ] Recipes show at the marker in the body, or at the end of the body
        without one. "Used in" shows on each ingredient's page.
  - [ ] These can be entered and read on every page involved: a pickaxe on a
        3 by 3 grid with "any Planks" and sticks; a recipe on a 9 by 9 grid;
        a smelting recipe with two stations; a list recipe with a
        profession, a skill level, and a kept tool.
  - [ ] docs/checklist.md has lines for all of it. Checks that cannot be
        proven are listed as not judged for me.
  - [ ] Every gate passes, and the `reviewer` has read the diff.
- **Out of scope:** the pattern editor, the tree, the totals, the bill of
  materials, recipes on boards, the import mapping screen (Phase 9).
- **Notes:** This is large. T6 may propose splitting it, for example showing
  recipes first and the editors second. No layout or control is named after
  a game.

## Later

- Phase 9: import and export screens (after T1).
- Phase 10: packaging and update check.
- The Phase 7 check: a board with a few hundred cards, from a generated
  guide, stays responsive while dragging.
- Whatever decisions 2, 12, and 13 lead to: "In place" editing; a second
  import of a board keeping local notes; combining pairs when a verb
  becomes symmetric.
- A section in docs/plan.md for the review follow-up work, so the plan and
  the commits agree.
- `[[` suggestions in board notes; wikilinks in notes rewritten on import.
- `dirty` on the remaining dialogs with typed input.
- Settings says that backups hold no media.
- UX review "Missing" items that need a decision each: related cards added
  around a selected card, a note turned into an entry, a string turned into
  a relation, a guide home page, templates per tag.
- UI review "Missing" items not built: the section in view in Contents (2),
  typographic poster placeholders (4), an avatar-style cover crop (5),
  shortcuts in menu rows (6), a reading width setting (7), board thumbnails
  (8), a selected card's connections standing out (9), a bundled serif face
  (10), a print stylesheet (11), a small-size app icon (12).
- The Later list in docs/plan.md: markdown extensions, video, lossy WebP,
  push pins, a drawing tool.
- Recipes are built after T2 and before Phase 9, so the import mapping
  screen is built once with recipe types in it (my answer, 2026-11-23).
  Phase 9 follows T8.
- Recipes, after T8, each with its own interview first:
  - The pattern editor (shapes in place of slots), designed in T6.
  - The recipe tree: indented and expandable, each ingredient opening to its
    own recipe; one default recipe per item, switched on the node; where the
    tree stops; loops.
  - Raw material totals for one output, as a row and under the tree.
  - The bill of materials: several crafts planned together; one list of raw
    materials and crafts to perform, with check boxes, rounded up to whole
    crafts; several saved; a "have" count per line.
  - A bill of materials placed on a board.
  - Calculator features: a target quantity, rates per minute, machine
    counts.
  - A list of recipes by recipe type or by station on any entry.
  - Whether recipes draw as lines on boards or show in the Relations tab.
  - Recipe types in the Phase 9 import mapping screen.

## Decisions waiting on me

Option (a) was the session's recommendation in each. Once answered, move the
decision to docs/design.md with its reason, or to Decisions below.

1. Walkthrough in Dark and Light: (a) agent takes screenshots; (b) I look first. **Answer:** (a)
2. "In place" editing: (a) Later; (b) remove; (c) build now. **Answer:**(a)
3. Pill toggle on lines: (a) strings only; (b) every line. **Answer:**(a)
4. Narrow windows: (a) mock three widths; (b) phone phase; (c) leave. **Answer:**(a)
5. Trash names: (a) kept until purge; (b) freed. **Answer:**(a)
6. Trash export: (a) left out; (b) carried. **Answer:**(a)
7. Import meets a trashed name: (a) "Name (imported)"; (b) refuse; (c) restore and import into it. **Answer:**(a)
8. Trashed boards shown: (a) in the guide's Trash; (b) on the Boards page. **Answer:**(a)
9. Trashed guides shown: (a) a Trash row in the guides sidebar; (b) a toggle. **Answer:**(a)
10. Empty Trash: (a) covers boards and guides; (b) one item at a time. **Answer:**(a)
11. Restore a board whose guide is trashed: (a) refuse; (b) allow. **Answer:**(a)
12. Second import of a board: (a) keep local notes and add imported ones; (b) keep replacing. **Answer:**(a)
13. Symmetric turned on for a verb with both A to B and B to A: (a) combine each pair; (b) leave both. **Answer:**(a)
14. T1, Theme line "Dark with Windows in light mode", marked passed although I listed it as not judged: (a) keep it passed, since Windows was already in light mode; (b) mark it not judged. **Answer:** (a)
15. T1, Theme line "no white frame at startup", marked passed from captures 30 ms apart: (a) mark it not judged and I check it by eye; (b) accept the pass with the limit as written. **Answer:** a pass; I looked and saw no white frame
16. Walkthrough scripts refuse to start when no `portable` file sits beside `-Exe`: (a) add it as a small follow-up to T1; (b) Inbox line only. **Answer:** (b)
17. T2: the plan in `.claude/work/trash-boards-guides/plan.md`: (a) approved; (b) change it. **Answer:** (a)
18. T2: `add_board_nodes` returning ids and a new `remove_board_nodes`, both in the design: (a) leave them out, since Add all with Undo is already built in the UI; (b) build them. **Answer:** (a)
19. T2: approval of the core tests: (a) once per core batch, three stops; (b) once for all core tests, one stop. **Answer:** (a)
20. T2: how a new core test fails first: (a) tests committed with stubs (signatures, fields, error variants; each new function returns an error), so old tests pass and new ones fail when run; (b) tests alone, nothing compiles until the code exists. **Answer:** (a), in the Rust book's way: see Decisions, 2026-11-13
21. T2: the batch 1 tests (10 new tests in `core/tests/behavior/trash.rs`, `guides.rs`, and `schema.rs`, with the stub layer; not committed): (a) approved, commit them and build batch 1; (b) change them. **Answer:** (a)
22. T2: the batch 2 tests (8 new tests in `core/tests/behavior/boards.rs`, `trash.rs`, and `media_cleanup.rs`, with the stub layer; not committed): (a) approved, commit them and build batch 2; (b) change them. **Answer:** (a)
23. T2: the batch 3 tests (8 new tests in `core/tests/behavior/export_import.rs`, with one stub, the report field `boards_renamed`; not committed). 7 fail for the expected reason. `a_guide_with_a_trashed_board_round_trips_without_it` passes already, because batch 2 made `list_boards` leave out trashed boards: (a) approved, keep the passing test as a pin, commit, and build batch 3; (b) approved without the passing test; (c) change them. **Answer:** (a)
24. T2 review: pages inside a trashed guide stay reachable by Back and through the guide sidebar (Tag editor, Vocabulary, Boards, a board, the guide's Trash). Tag delete, tag merge, and vocabulary merge work there and cannot be undone. The plan said no page offers them; that was wrong: (a) every page inside a trashed guide shows "This guide is in the Trash" with Restore, and the guide sidebar is not drawn; (b) the same, and core also refuses every edit inside a trashed guide; (c) keep it, and correct the plan and `docs/design.md`. **Answer:** (a), and note it in the design
25. T2 review: purging a guide removes two things from other, live guides that the dialog does not name: qualifier values that point at its entries, and strings tied to its cards on other guides' boards: (a) `guide_contents` gains a count for each and the dialog a line for each; the approved test `guide_contents_counts_what_purge_removes` changes to name the two new fields; (b) leave the dialog, and record the gap in `docs/design.md`. **Answer:** (a)
26. T2 review: Empty trash on the guides list adds up the per-guide counts, so one relation between two trashed guides is counted twice: (a) core counts all trashed guides together in one new function, test first; (b) the dialog lists each guide with its entry count and says everything in them is removed, with no summed counts; (c) keep the sums. **Answer:** (a)
27. T2 review, two small gaps in core: a new qualifier label can still be created through a relation of a trashed guide (no page reaches it); the Vocabulary page's "in Trash" count ignores relations into a trashed guide, so Delete is refused with no reason shown: (a) a failing test for each, shown to me, then the fix; (b) Inbox lines only. **Answer:** (a)
28. T2: the Batch 5 tests (4 new tests: 3 in `core/tests/behavior/trash.rs`, 1 in `vocabulary.rs`; the stub layer is two new `GuideContents` fields at 0 and `guide_trash_contents` returning zeros; the approved test `guide_contents_counts_what_purge_removes` gains the two fields at 0 and nothing else; not committed). All 4 fail at their own assertions: (a) approved, commit them and build parts B, C, and D; (b) change them. **Answer:** (a)
29. T2 review of Batch 5: a page inside a guide keeps its controls when the guide goes to the Trash while the page is open. Found path: restore a guide from the guides' Trash, open its Tag editor (or Vocabulary, an entry, a board), then click Undo in the "Restored the guide" notice. The guide is in the Trash again; the page and the guide's sidebar stay, so tag delete, tag merge, and vocabulary merge are reachable, which decision 24 was meant to stop. The Tag editor, Vocabulary, and the entry page do not reload on a change; the board page reloads but keeps its cached guide: (a) every page inside a guide checks its guide again whenever anything changes (`session.revision`), so any such path shows "This guide is in the Trash"; (b) Undo in the "Restored the guide" notice also goes to the guides' Trash page, which closes this one path only; (c) both. **Answer:** (a)
30. T2 review of Batch 5: two rules have no core test. First, a qualifier whose "new" label text names a label the trashed guide already has is not refused (a guess I confirmed on 2026-11-13). Second, a string with an image end on another guide's board is not counted in `strings_on_other_boards` unless its other end is a card of the purged guide. The code does both today, so the tests would pass at once and act as pins: (a) I write both tests and show them to you before committing; (b) Inbox line only. **Answer:** (a)
31. T2: the two pin tests of decision 30 (in `core/tests/behavior/trash.rs`, 98 new lines, no existing test changed, both pass; not committed): (a) approved, commit them; (b) change them. **Answer:** (a)
32. T2 walkthrough: `trash.ps1` takes the keyboard, the mouse focus, and the clipboard for several minutes per run, twice (Dark, then Light), and its first run will likely stop for script fixes: (a) take the wheel now; (b) wait until I say so. **Answer:** (a)
33. T3: how a string's label is drawn when its label background is off: (a) text only, with a thin outline in the canvas color around the letters, so the string does not strike through them, the way map labels are drawn; (b) text only, no outline, and the string runs through the letters; (c) text only, moved above the string, no outline. **Answer:** (a)
34. T3: the archive. A string in a board file gains the field `label_background`: (a) written only when it is off, as `"label_background": false`; absent means on; `format_version` stays 1, as it did when a note's height was added; (b) always written; `format_version` stays 1; (c) `format_version` becomes 2, and a build that knows only 1 refuses the archive. **Answer:** (a)
35. T3: two approved tests must change. `core/tests/behavior/boards.rs` has a `BoardStringUpdate` literal that names every field, so a new field stops it compiling. `a_version_6_vault_migrates_to_7_with_nothing_trashed` in `schema.rs` asserts that the schema version is 7, and T3 makes it 8: (a) the literal gains `..BoardStringUpdate::default()`, and the two version lines compare with `SCHEMA_VERSION`; both edits are shown with the new tests before the commit; (b) change them another way, and I say how. **Answer:** (a)
36. T3: the plan in `.claude/work/t3-pill-toggle/plan.md`: (a) approved; (b) change it. **Answer:** (a)
37. T3: the six core tests and the two edits of decision 35 (in `core/tests/behavior/boards.rs`, `schema.rs`, and `export_import.rs`, with the stub layer; not committed). All 6 fail at their own assertions: (a) approved, commit them and build; (b) change them. **Answer:** (a)
38. T3: the window check for done criteria 1 and 3 (the switch, and the bare label on four backgrounds in both schemes). No release build holds T3 yet: (a) the agent makes a release build, adds steps to `tools/walkthrough/notes.ps1`, and takes the keyboard, the mouse, and the clipboard for a run in Dark and a run in Light; (b) the agent makes the release build without opening a window, and I check the new line of `docs/checklist.md` by eye; (c) leave it not judged for now and close the task with the line unchecked. **Answer:** (a)
39. T3 review: no test turns a string's label background off and then imports the same archive a second time into the same guide (the "second import replaces the board" path). It shares its one insert with the tested path, so the test would pass at once and act as a pin: (a) Inbox line only; (b) I write the pin test and show it to you before committing. **Answer:** (a)

## Inbox

Anything discovered or thought of mid-task, by either of us. The agent appends
and keeps working; I triage between tasks.

Format: `- [date] [agent|me] idea or problem — where it came up`

**CI/CD**
- [2026-11-12] [Eric] Enhancement: Setup CI/CD pipeline through github actions. Should test code, run security audits and if both succeed build releases in parallel if possible for windows, mac and linux. All releases should be bundled into a single github release with notes derived from commits.


**Defects** (for a defect pass, run as its own session task)

- [2026-11-12] [agent] Defect: the Images album does not open on an entry whose body shows the same picture twice; after that, the album on the next entry shows its backdrop and no dialog. Reproduced with a real mouse click. Suspected, not verified: the album list is keyed by the picture's address in `ui/src/components/ImageAlbum.svelte` — T1 walkthrough
- [2026-11-12] [agent] Defect: all three theme pictures in Settings follow the current scheme (three dark windows in Dark, three light in Light). Fails Theme line 1 — T1, `drive/40-settings-appearance` in both schemes
- [2026-11-12] [agent] Defect: on a board in Dark, a picture card's caption is pale text (about #E5E1D6) on the off-white frame (#D6D2C8). Fails Theme line 11 — T1, `dark/board/b13`. Seen again by me on 2026-11-24: "entries with images have a label that is white and very difficult to read" — my T3 check
- [2026-11-12] [agent] Defect: the sidebar's entry list keeps the "stub" chip after an entry's body is first written, until something else reloads the sidebar — T1, `dark/board/b00`, `dark/drive/05`
- [2026-11-12] [agent] Defect: the entry header's Read or Edit toggle, Images, and More move right when Edit folds the right panel (toggle at x 710 in Read, x 1001 in Edit). The Entries checklist says they do not move — T1, `dark/drive/20` and `21`
- [2026-11-12] [agent] Defect: with the sidebar folded, the breadcrumbs start 56 px right of the content column — T1, `dark/drive/22`
- [2026-11-12] [agent] Defect: the board panel's title for a selected string reads "Stringfrom Stone Block to note ..."; a space is missing — T1, `dark/notes/n06`. Seen again by me on 2026-11-24 — my T3 check
- [2026-11-12] [agent] Defect: Settings, Data: a long path breaks in the middle of a word ("va" / "ult") — T1, `dark/drive/41`
- [2026-11-13] [me] Web links: only a fully qualified address is recognized, such as `http://google.com`; a bare `google.com` is not a link. "Open in browser" itself works. Defect or enhancement: for triage — my check of T1's not judged list
- [2026-11-13] [me] Defect: a double click on a board note does not open it for editing, whether I click as fast as possible or slowly. docs/design.md section 6 says it should, and `BoardCanvas.svelte` has code for it (two presses on the same note within 400 ms, or Enter). Cause not investigated. The walkthrough scripts open notes by keyboard only, so they did not catch it — after T1

- [2026-11-13] [agent] Problem: purging one entry removes the qualifier values that point at it and the strings tied to its cards, and the purge dialog names neither. It predates T2 — T2 review
- [2026-11-13] [agent] Suspected, not verified: deleting or merging a global tag takes it off a guide that is in the Trash, so Restore does not return it. `docs/design.md` section 3 says a trashed guide keeps everything — T2 review
- [2026-11-13] [agent] Problem: a new relation to an entry whose guide is in the Trash is refused with "is in the trash; restore it first". The entry is not in the Trash; its guide is — T2
- [2026-11-13] [agent] Problem: the Vocabulary page says "1 of them has an entry in Trash" for a relation into a guide that is in the Trash. The entry is not in the Trash; its guide is. The count is right since Batch 5; the words are not — T2 batch 5, part D
- [2026-11-13] [agent] Problem: on an entry page or a board page of a guide in the Trash, the palette still lists "Rename entry", "Toggle Read and Edit", "Add a tag", "Add a relation", "New note", and "Add entries". They do nothing visible and show no notice. No entry data is written; "Add entries" and "Show or hide the right panel" write the panel preference — T2 batch 5 review
- [2026-11-13] [agent] Suspected, not verified: after Restore on a "This guide is in the Trash" page the button is gone and nothing moves keyboard focus, so focus falls to the page body — T2 batch 5 review
- [2026-11-23] [agent] Problem: the page for a board in the Trash, and a board's page inside a guide in the Trash, show the breadcrumb "Guides / <guide>" without "Boards" and the board's name. An entry's page inside a trashed guide does name the entry — T2 walkthrough, `dark/trash/t01b`, `t34d`
- [2026-11-23] [agent] Question: Undo in the "Moved the board to Trash" notice opens the board when the board was trashed from its own page, and only puts it back in the list when it was trashed from the Boards list. Keep the difference or open the board in both? — T2 walkthrough, `dark/trash/t52`, `t56b`
- [2026-11-23] [agent] Problem: `EntryView.load` has no newest-request guard, so two loads that overlap can finish in either order. It predates T2; the decision 29 fix added one more caller, which now skips a check that was overtaken — T2 decision 29 review
- [2026-11-13] [agent] Wording: "2 qualifiers on relations in other guides" in the purge dialog also covers two qualifiers on one relation — T2 batch 5 review
- [2026-11-23] [agent] Problem: Undo ties a deleted string again in two calls, create and then update (color, and since T3 the label background). If the second call fails, the string comes back red with its pill and no notice. It predates T3. One core call that ties a string with all its attributes would make Undo one step — T3 review, `retie` in `ui/src/views/BoardView.svelte`
- [2026-11-23] [agent] Suspected, not verified: at the minimum window height of 400 pixels the board panel's upper half may be shorter than the selected string's section, which is one row taller since T3, so "Delete string" may overlap the picker's heading. Estimated from the CSS. It belongs with T4 — T3 review, `ui/src/components/BoardPanel.svelte`
- [2026-11-23] [agent] Test gap: no core test turns a string's label background off and then imports the same archive a second time into the same guide. That path shares its one insert with the tested import — T3 review; I answered "Inbox line only" (decision 39)
- [2026-11-24] [me] Observation: with a string selected on the board and no click in the panel, Tab does not move through the selected string's options (label, colors, Label background, Delete string); it jumps between other board items. From a control inside the panel, Tab does reach the next option. Defect or enhancement: for triage — my T3 check
- [2026-11-24] [agent] Question: two string labels with their background off at one place are drawn over each other, and neither reads. With pills the upper one covers the lower. Seen where two strings cross at their middles. Leave it, or move one label along its string? — T3 walkthrough, first Dark run, `n31`
- [2026-11-24] [agent] Idea: the outline of a string's bare label scales with the zoom, so at 63 percent it is about 1 pixel and the string shows between the letters. An outline that keeps its width on screen would read better zoomed out. It belongs with the board level of detail line — T3 walkthrough, `light/notes/n32-bare-cork`
- [2026-11-23] [agent] Problem: Windows High Contrast mode (forced colors) removes text shadows, so a string's label with its background off loses its outline there. Nothing in the app is checked in that mode — T3 review

**Questions and unconfirmed guesses** (each needs my answer)

- [2026-11-12] [agent] Question: board panel, Relations tab: after Save the verb suggestions stay open over the relation list, and the list has about one row of height above the picker. Change or keep? — T1, `dark/board/b13`
- [2026-11-12] [agent] Question: after navigation by keyboard, the page heading shows a focus ring as a wide box. Keep or remove? — T1, `dark/drive/40`, `51`
- [2026-11-12] [agent] Question: an info notice stays up across pages until dismissed or replaced, and covers the bottom rows of Settings and of the media cleanup list. Keep or time it out? — T1, `dark/drive/43`
- [2026-11-12] [agent] Question: the editor's text looks bold in both schemes. Intended? — T1, `drive/05`
- [2026-11-12] [agent] Guess: editor shortens long media file names except on the line being edited; saved text unchanged. Nobody asked for it. Keep or remove?
- [2026-11-12] [agent] Guess: yellow note is slightly darker in Dark so links pass contrast. Keep or remove?

- [2026-11-13] [agent] Question: while a board in the Trash holds a name, every import of the same archive makes one more board, "Name (imported)", then "Name (imported 2)". Match the renamed board on a later import? It belongs with decision 12 — T2
- [2026-11-13] [agent] Guess: one purge line differs from the Trash design: "7 of those relations are with entries in other guides." The design's wording read as if those relations were counted twice. Keep or change?

**Board enhancements**

- [2026-11-13] [me] Enhancement: board "icon pins" or "stickers" in a variety of shapes. They stay prominent at every zoom level. Mine: stars, arrows, skull, treasure chest, lock, key. The app is not only for games. Agent's additions: question mark, exclamation mark, check mark, cross, numbered and lettered badges, flag, map marker, heart, lightbulb, warning triangle, clock, calendar, magnifier, eye, person, group, house, book, bookmark, speech bubble, coin, wrench, flask, leaf, lightning bolt, target, thumbs up and down, a plain dot in each tag color; and a sticker made from any typed character or emoji, so the set stays open — after T1
- [2026-11-13] [me] Enhancement: board level of detail. Zoomed out, things become unreadable; different levels of detail by zoom level may help. I am unsure of the best way; may be worth a dedicated research task — after T1
- [2026-11-13] [me] Enhancement: board multi-select with the common keys (think Lucidchart). Hold click and drag to box select. Shift with hold click and drag adds to the selection — after T1
- [2026-11-13] [me] Enhancement: board right-click menu by context. On empty space: a palette of things to add (note, pins, stickers, searchable entries). On an item: lock position, delete, arrange (bring to front, send to back, and the like). With several items selected: group, ungroup, lock, arrange, delete — after T1

**Walkthrough scripts**

- [2026-11-12] [agent] Idea: the three walkthrough scripts repeat one helper block of 356 lines. Move it to one file they all load? — T1
- [2026-11-12] [agent] Problem: the walkthrough scripts flip "Line numbers" and "Dim images in dark" by a click without reading their state. A run that stops between the two clicks leaves the setting on, and the next run's screenshots show the opposite state with 0 misses — T1 review
- [2026-11-12] [agent] Problem: the two restart checks (Dark saved at startup; old `config.toml` without `theme`) have no committed script, so two checked Theme lines cannot be repeated from the repository — T1 review
- [2026-11-12] [agent] Problem: a board note's text goes out in one SendKeys call after one foreground check. If focus changes during that call, the rest of the text goes to the other window. Suspected, not seen — T1 review
- [2026-11-24] [agent] Problem: in one run of `notes.ps1`, step 7, the Enter that ties the first string (card to note, focus set by UI Automation) did nothing; the tying hint stayed, and 16 misses followed. T1's runs of the same step and the next three runs were clean. Cause not found; no app defect was shown — T3 walkthrough
- [2026-11-24] [agent] Problem: the first launch in a new run folder makes the webview profile and took longer than the 15 seconds the scripts wait for the guides list. The README now says to run again; a longer first wait would remove the step — T3 walkthrough
- [2026-11-13] [agent] Idea: the walkthrough scripts refuse to start when no file named `portable` sits beside `-Exe`. Without it they would drive the real data directory — T1 review; I answered "Inbox line only" (decision 16)

## Decisions

Small decisions that do not belong in docs/design.md.

Format: `- [date] decision — reason`

- [2026-11-12] `explorer` runs on Sonnet and `reviewer` inherits the main model — set by me in the agent files.
- [2026-11-12] Push to `main` before a new session starts — my instruction; no reason given.
- [2026-11-12] The walkthrough scripts live in `tools/walkthrough/`; the app copy, the test vault, and screenshots live in `tools/walkthrough/run/`, ignored by git — the temp folder they were in would not survive a cleanup. I chose (a); the session picked the folder.
- [2026-11-12] The leftover walkthrough vault is not committed — no script or test reads it; each script makes its own guide.
- [2026-11-12] This file names the commit the gates last ran at, not the head; `git log` gives the head — a committed file cannot name its own commit.
- [2026-11-12] T1 plan approved. The agent updates the scripts and builds with no window, then stops and asks for the keyboard and mouse; once given, it has as long as needed — my answer (a) to T1 question 1.
- [2026-11-12] T1: the keyboard and mouse are handed over for the walkthrough runs of this task — my answer (a). It covers T1 only.
- [2026-11-12] T1: the agent does not switch Windows between light and dark app mode. The three Theme lines that need it are marked "not judged" and I check them by eye — my answer (b).
- [2026-11-12] T1: the web link check stops at the confirm dialog. The browser launch is marked "not judged" — my answer (b).
- [2026-11-12] A check that screenshots cannot prove is marked "not judged", never passed. The agent lists every "not judged" line at the end of the task and I validate them before the next task starts. A validated line is marked off and stays marked off until code that affects that feature changes — my instruction. "Marked off" read by the agent as: checked in docs/checklist.md.
- [2026-11-12] T1 guesses confirmed: a fresh build from the head; a new run folder `tools/walkthrough/run/t1/`; the three scripts extended, no fourth; a `-Theme` parameter; the markdown fixture file.
- [2026-11-13] T1 validated by me: the four not judged checks passed, "Dark with Windows in light mode" stays passed, "no white frame" is a pass by my own look, and the `portable` refusal for the scripts is an Inbox line only — my answers to T1's report.
- [2026-11-13] Defects found in a task go to the Inbox. A defect pass is run as its own session task — my instruction.
- [2026-11-13] The Inbox is grouped (Defects; Questions and unconfirmed guesses; Board enhancements; Walkthrough scripts) — I said to group as the agent sees fit; the agent chose the groups.
- [2026-11-13] Double click on a board note is a defect, not an enhancement: I tried it fast and slow and it did not open the note. The line moved to Defects — my answer (a).
- [2026-11-13] The agent's guesses in T1's closing report stand for now: the web link line sits under Defects for triage; the agent's sticker suggestions stay on the icon pins line; the Entries and Boards checklist lines that hold the browser launch and Escape in a note stay unchecked; the four Inbox group names — my answer.
- [2026-11-13] T2 plan approved (decision 17). `add_board_nodes` and `remove_board_nodes` are left out of T2, because Add all with Undo is already built in the UI (decision 18). Core tests are approved once per core batch (decision 19) — my answers (a).
- [2026-11-13] A new core test is committed with stubs, the way the Rust book does it (chapter 12.4, listings 12-15 and 12-16, https://doc.rust-lang.org/book/ch12-04-testing-the-librarys-functionality.html): just enough code that the test compiles and does not panic inside the new function, so it fails at its own assertion. A stub returns an empty or unchanged value; it never calls `unimplemented!` or `todo!` — my answer (a) to decision 20, with the page I pointed at. "Fails at its own assertion" is the agent's reading of the page.
- [2026-11-13] T2: the batch 1 tests are approved as written (decision 21). They are not edited while batch 1 is built; if one looks wrong, the agent stops and asks — my answer (a).
- [2026-11-13] T2: the batch 2 tests are approved as written (decision 22) — my answer (a).
- [2026-11-13] T2: the batch 3 tests are approved as written, and the round-trip test that passes already stays as a pin (decision 23) — my answer (a).
- [2026-11-13] T2 review follow-up, my answers (a) to decisions 24 to 27. Every page inside a trashed guide shows "This guide is in the Trash" with Restore, and the guide sidebar is not drawn; `docs/design.md` section 3 has the rule and its reason. `guide_contents` gains counts for the qualifier values and the strings a purge removes from other guides, and the approved test `guide_contents_counts_what_purge_removes` may change to name the two new fields. Core counts all trashed guides together for Empty trash on the guides list. The two small core gaps each get a failing test, shown to me, then the fix. None of this is built yet; the plan's Batch 5 lists it.
- [2026-11-13] The guesses in the T2 review report stand: `main` was pushed at 0286726; no fix that touches an approved test was started without an answer — my answer "check".
- [2026-11-13] T2: the Batch 5 tests are approved as written (decision 28). They are not edited while parts B, C, and D are built; if one looks wrong, the agent stops and asks — my answer (a).
- [2026-11-13] T2: the agent may run a walkthrough when T2 is wrapped up — my words: "When we wrap up this task you can do a walkthrough". It covers T2 only. The agent reads it as: after parts B, C, and D are built and the gates pass, the keyboard and mouse are handed over for the T2 walkthrough runs.
- [2026-11-13] The guesses in the Batch 5 part A report stand — my answer "check" to each: on a trashed guide's pages the sidebar column shows the guides list's sidebar; the entry page shows the same message as the other pages; a board in the Trash inside a guide in the Trash shows the guide's message first; the palette lists no entries of a trashed guide and offers no "create entry" row; New opens New guide there; breadcrumbs are unchanged. The two planned guesses stand too: a "new" qualifier label whose text names an existing label counts as an edit and is not refused; the dialog copy is "N qualifiers on relations in other guides name these entries and are removed." and "N strings tied to those cards go with them."
- [2026-11-23] T2, decision 29, my answer (a): every page inside a guide checks its guide again whenever anything changes, so a page that is open when its guide goes to the Trash shows "This guide is in the Trash". I also asked whether delete events should update the page with a banner or a popup and a control toggle. The agent's answer: (a) already is the event; every change bumps `session.revision` at once, the app has one window, and decision 24 chose the message in place of the page over disabled controls. No banner is built.
- [2026-11-23] T2, decision 30, my answer (a): the agent writes the two pin tests and shows them to me before committing.
- [2026-11-23] The guesses in the Batch 5 parts B, C, and D report stand. The singular dialog lines, the order of the two new lines, and the Vocabulary wording are "good enough for now"; a UX design pass and a UI clean-up pass may revisit them. "this entry" for a guide with one entry, the fourth walkthrough script, and the seed program outside the workspace: "check".
- [2026-11-23] T2: the two pin tests of decision 30 are approved as written (decision 31) - my answer (a).
- [2026-11-23] T2: the keyboard and mouse are handed over for the T2 walkthrough runs (decision 32) - my answer (a). It covers the T2 runs of `trash.ps1` in this session only.
- [2026-11-23] T2 validated by me: New guide's vocabulary list does not offer a trashed guide (pass); the board picker remembers its search line, tag chip, and Stubs only after the board is reopened (pass); Rename works from the board page and from the Boards list (pass). The four guesses in the T2 closing report have no answer from me yet.
- [2026-11-12] The two 2026-11-10 reviews live in `docs/reviews/` and the Trash design in `.claude/work/trash-boards-guides/design.md`; the dark mode plan and the subagent briefs stay outside the repository — they are history.
- [2026-11-23] Recipes, planning interview round 1 (a separate planning session; no task yet; "recipe" is the working word, the name is not decided). My answers:
  - Native, not a plugin: each guide defines its recipe types as data (name, layout, grid size), the way it defines verbs; no code is loaded — "(a) sounds good". The agent's reason: an archive must round-trip without carrying code.
  - "A tiered list" means an indented tree in which each ingredient opens to its own recipe; I meant World of Warcraft, not Minecraft.
  - Amounts: display with raw material totals, and a planner (a + c). My words: "As a user I would like to be able to plan out multiple, complex crafts and receive a complete bill of material as a check list. As a user I would also like to be able to save multiple BoMs. As a user I would like to be able to add a bill of material to a board."
  - The first build serves Minecraft, with modded grid sizes. Vintage Story is the wildcard to test the model against.
  - A recipe may show an item's icon in place of its text; hovering shows a popup with more details; a click on an input's image opens that item's entry.
  - The three layout sketches (grid with station and output, list with profession and skill, expandable tree) were well received.
  - Where a recipe is stored is open: I asked for research first on how wikis handle a recipe (Minecraft, Vintage Story, Factorio, Satisfactory, Grounded; Wowhead, Icy Veins, and other World of Warcraft wikis).
- [2026-11-23] Recipes, planning interview round 2. The research is in `.claude/work/recipes/research.md` (my "check" on the agent writing it). The agent numbered the questions 1, 6, 7, 8, 9, 10 and I answered 1 to 6; the agent reads my answers by position. My answers:
  - A recipe is its own record in core. It appears on the output's page and, generated, on each ingredient's page — (a).
  - Recipes show where I place a marker in the entry body — (c): "this seems to make the most sense looking at minecraft.wiki".
  - One slot holds one or more choices, each an entry or a tag ("any Planks") — (a).
  - In the tree, each item has one default recipe that I can switch on the node — (a).
  - A bill of materials lists raw materials and the crafts to perform, both with check boxes, rounded up to whole crafts — (a).
  - Vintage Story: my "6: (b)" is read as "also a pattern editor". Not confirmed; the agent asked.
  - I attached three pictures: the Satisfactory wiki's recipe table (a row per recipe; ingredients, produced in, products, unlocked by; rates per minute; an "Alternate" badge), Wowhead's tooltip and reagent list, and the Factorio wiki's infobox (a recipe row and a "Total raw" row). My words: "These are more a table. Factorio gives you a total raw BOM but isn't a tree either."
- [2026-11-23] Recipes, planning interview round 3, my answers (a) to all six. T6, T7, and T8 were written from the three rounds.
  - Vintage Story: a pattern editor is designed now and built after grid and list. My "6: (b)" in round 2 meant this.
  - An entry with recipes and no marker shows them at the end of its body; a marker moves them — so nothing recorded is hidden by a forgotten marker (the agent's reason).
  - Layouts in the first build: grid, list, and row (icons in one line, as on the Factorio wiki). Several recipes of one entry form one table, a row per recipe.
  - Names for now: "recipe", "recipe type", "bill of materials".
  - Recipes are built after T2 and before Phase 9, so the import mapping screen is built once with recipe types in it.
  - The first task is design and mock-ups with nothing built, as T4 and T5 are.
  - Generic names only. My words: "I don't want them to be presented as "minecraft recipe", "vintage story recipe", etc. I'd like to use generic names for what we're offering. Grid recipe, tree recipe, etc. We can workshop names as we move into UX."
  - These answers are not in docs/design.md yet. T6 moves them there with the approved design.
- [2026-11-23] T3, my answers (a) to decisions 33 to 36, and to the report's two concerns. With its label background off, a string's label is text with a thin outline in the canvas color. The archive field `label_background` is written only when it is off, absent means on, and `format_version` stays 1. The two approved tests change as decision 35 (a) says, shown with the new tests before the commit. The plan is approved. Undo of a deleted string restoring the new value too: "accepted". Cork's grain judged only in a window: "understood". The rule and the field are in docs/design.md sections 6 and 7, written before the code; the code does not follow them yet.
- [2026-11-23] T3: the six core tests and the two edits of decision 35 are approved as written (decision 37). They are not edited while T3 is built; if one looks wrong, the agent stops and asks - my answer (a).
- [2026-11-24] T3, my answers (a) to decisions 38 and 39. The agent makes a release build, adds steps to `tools/walkthrough/notes.ps1`, and takes the keyboard, the mouse, and the clipboard for a run in Dark and a run in Light. It covers the T3 runs of `notes.ps1` in this session only. The second-import test gap is an Inbox line only.
- [2026-11-24] T3 validated by me: with focus on a color swatch in the panel, Tab reaches the "Label background" switch, and Space and Enter flip it (pass). My three observations are in the Inbox: two were there already and are marked as seen again; the Tab path from a selected string into the panel is a new line.
- [2026-11-24] T3: the focus ring on the "Label background" switch was present - my answer (a). Both keyboard lines for the switch are checked in `docs/checklist.md`.

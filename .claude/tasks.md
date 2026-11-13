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
  both schemes. T1 walked it through on 2026-11-12: 11 lines passed, 2
  failed, 2 are not judged. The gate stays open until the 2 failures are
  fixed and I have checked the 2 not judged lines.
- Phase 7: "a board with a few hundred cards stays responsive while dragging"
  was never checked. docs/plan.md says it waits for a walkthrough with a
  generated guide.
- The review follow-up work (commits 7dea8ff, 8b15673, bd5092f, f5cf5a1,
  ce8a1cc) has no section and no gate in docs/plan.md. Its checks are lines
  in docs/checklist.md. T1 marked the Theme section only.

Gates last run at ce8a1cc, the last commit that changed code: 31 unit, 151
behavior, 159 UI tests, clippy clean, svelte-check clean, release build
succeeded. For the current head, run `git log --oneline -5`; this file
cannot name the commit that holds it.

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

### The three walkthrough scripts

- Where they are: `tools/walkthrough/` in the repository (`drive.ps1`,
  `board.ps1`, `notes.ps1`), committed 2026-11-12.
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
- Until T2 is built, Delete board still removes its notes, images, and
  strings after one confirm that does not list them.
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
- **Status:** done 2026-11-12, except that 4 "not judged" checks wait for me
  to validate them (listed in Notes)
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
        passed or failed. (11 passed, 2 failed, 2 not judged.)
  - [x] Each item under "The subagents asked for a look at" is marked fine
        or reported.
  - [x] Every defect found is one line in the Inbox.
- **Out of scope:** fixing what it finds; the few-hundred-cards check
  (Later); the facts box (T5).
- **Notes:**
  - The gate of "Dark mode and tokens" is not closed. Theme lines 1 (theme
    pictures) and 11 (picture card caption in Dark) failed.
  - Not judged, for me to validate by eye:
    1. System: switching Windows between light and dark changes the app
       while it runs.
    2. Light with Windows in dark mode: the app and the title bar are light.
    3. "Open in browser" in the web link dialog opens the default browser.
       The dialog itself is fine; the script answered Cancel.
    4. Escape in a board note saves once, not twice. Focus does return to
       the note (seen in the script's output for all five notes).
  - Passed with a limit: no white frame at startup. Captures ran from 85
    ms to 1.5 s, about 30 ms apart; a shorter flash would not show. Whether
    this counts as passed is decision 15.
  - "Dark with Windows in light mode" is marked passed, although my answer
    on 2026-11-12 listed it as not judged. Reason: Windows was already in
    light app mode (read, not changed), and the title bar is dark in
    `dark/drive/50-screen-title-bar`. Whether the mark stays is decision 14.
  - The reviewer found no blocker. Its open points are decisions 14 to 16
    and two Inbox lines. The last two script edits (the "Theme: System"
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
    long title move between Read and Edit. Not judged: Escape in a note
    (double save); the browser launch.
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
- **Status:** todo (decisions 5 to 11 answered (a) on 2026-11-12)
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
  - [ ] Migration 0007 adds `deleted_at` to boards and guides; the schema
        version is 7; existing vaults migrate with nothing trashed.
  - [ ] A trashed guide hides its entries, relations, qualifier values, and
        board cards from every other guide, and restore returns all of it,
        positions included.
  - [ ] A trashed board keeps its notes, images, and strings; restore
        returns them; media cleanup keeps its images while it is in Trash.
  - [ ] Nothing can be created in a trashed guide, and writes to a trashed
        board are refused.
  - [ ] Export, import, and name rules follow decisions 5 to 7, each with a
        round-trip test.
  - [ ] Delete board and Delete guide show a notice with Undo and no dialog.
  - [ ] Trashed boards and guides are listed where decisions 8 and 9 say,
        with Restore and Delete for good; purge names the counts core
        computed.
  - [ ] Empty Trash and restore follow decisions 10 and 11.
  - [ ] docs/design.md sections 3, 5, 6, 7, and 8 are updated.
- **Out of scope:** decision 12 (second import of a board); backups and
  media.
- **Notes:** The design's named tests are the starting list for the tests
  AGENTS.md asks for first. `empty_trash` changes its return type, so the UI
  and its tests change in the same batch.

### T3: Pill toggle on lines
- **Status:** todo (decision 3 answered (a) on 2026-11-12)
- **Goal:** Let me turn the pill background behind a line's label off for
  one line, so a busy board stays readable. Today every line has the pill.
- **Done when:**
  - [ ] The selected line's section in the board panel has a "Label
        background" switch, on by default.
  - [ ] The choice survives a reload and an export and import.
  - [ ] With the pill off, the label still reads on all four backgrounds in
        both schemes, or the task says where it does not.
  - [ ] docs/design.md section 6 records the rule.
- **Notes:** Under (a) the choice is a new column on `board_string` and a
  new field on a string in the archive's board file. That is a migration and
  a small format addition; the session's earlier summary of (a) did not say
  so. Under (b) relation and mention lines need new storage per board,
  because those lines are derived and have no row. Asked in my words:
  "(a) optional pill with background, toggle per line".

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
14. T1, Theme line "Dark with Windows in light mode", marked passed although I listed it as not judged: (a) keep it passed, since Windows was already in light mode; (b) mark it not judged. **Answer:**
15. T1, Theme line "no white frame at startup", marked passed from captures 30 ms apart: (a) mark it not judged and I check it by eye; (b) accept the pass with the limit as written. **Answer:**
16. Walkthrough scripts refuse to start when no `portable` file sits beside `-Exe`: (a) add it as a small follow-up to T1; (b) Inbox line only. **Answer:**

## Inbox

Anything discovered or thought of mid-task, by either of us. The agent appends
and keeps working; I triage between tasks.

Format: `- [date] [agent|me] idea or problem — where it came up`

- [2026-11-12] [agent] Guess: editor shortens long media file names except on the line being edited; saved text unchanged. Nobody asked for it. Keep or remove?
- [2026-11-12] [agent] Guess: yellow note is slightly darker in Dark so links pass contrast. Keep or remove?
- [2026-11-12] [agent] Defect: the Images album does not open on an entry whose body shows the same picture twice; after that, the album on the next entry shows its backdrop and no dialog. Reproduced with a real mouse click. Suspected, not verified: the album list is keyed by the picture's address in `ui/src/components/ImageAlbum.svelte` — T1 walkthrough
- [2026-11-12] [agent] Defect: all three theme pictures in Settings follow the current scheme (three dark windows in Dark, three light in Light). Fails Theme line 1 — T1, `drive/40-settings-appearance` in both schemes
- [2026-11-12] [agent] Defect: on a board in Dark, a picture card's caption is pale text (about #E5E1D6) on the off-white frame (#D6D2C8). Fails Theme line 11 — T1, `dark/board/b13`
- [2026-11-12] [agent] Defect: the sidebar's entry list keeps the "stub" chip after an entry's body is first written, until something else reloads the sidebar — T1, `dark/board/b00`, `dark/drive/05`
- [2026-11-12] [agent] Defect: the entry header's Read or Edit toggle, Images, and More move right when Edit folds the right panel (toggle at x 710 in Read, x 1001 in Edit). The Entries checklist says they do not move — T1, `dark/drive/20` and `21`
- [2026-11-12] [agent] Defect: with the sidebar folded, the breadcrumbs start 56 px right of the content column — T1, `dark/drive/22`
- [2026-11-12] [agent] Defect: the board panel's title for a selected string reads "Stringfrom Stone Block to note ..."; a space is missing — T1, `dark/notes/n06`
- [2026-11-12] [agent] Defect: Settings, Data: a long path breaks in the middle of a word ("va" / "ult") — T1, `dark/drive/41`
- [2026-11-12] [agent] Question: board panel, Relations tab: after Save the verb suggestions stay open over the relation list, and the list has about one row of height above the picker. Change or keep? — T1, `dark/board/b13`
- [2026-11-12] [agent] Question: after navigation by keyboard, the page heading shows a focus ring as a wide box. Keep or remove? — T1, `dark/drive/40`, `51`
- [2026-11-12] [agent] Question: an info notice stays up across pages until dismissed or replaced, and covers the bottom rows of Settings and of the media cleanup list. Keep or time it out? — T1, `dark/drive/43`
- [2026-11-12] [agent] Question: the editor's text looks bold in both schemes. Intended? — T1, `drive/05`
- [2026-11-12] [agent] Idea: the three walkthrough scripts repeat one helper block of 356 lines. Move it to one file they all load? — T1
- [2026-11-12] [agent] Problem: the walkthrough scripts flip "Line numbers" and "Dim images in dark" by a click without reading their state. A run that stops between the two clicks leaves the setting on, and the next run's screenshots show the opposite state with 0 misses — T1 review
- [2026-11-12] [agent] Problem: the two restart checks (Dark saved at startup; old `config.toml` without `theme`) have no committed script, so two checked Theme lines cannot be repeated from the repository — T1 review
- [2026-11-12] [agent] Problem: a board note's text goes out in one SendKeys call after one foreground check. If focus changes during that call, the rest of the text goes to the other window. Suspected, not seen — T1 review

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
- [2026-11-12] The two 2026-11-10 reviews live in `docs/reviews/` and the Trash design in `.claude/work/trash-boards-guides/design.md`; the dark mode plan and the subagent briefs stay outside the repository — they are history.

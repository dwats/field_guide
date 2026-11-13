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
  both schemes. It needs a running window. That is T1.
- Phase 7: "a board with a few hundred cards stays responsive while dragging"
  was never checked. docs/plan.md says it waits for a walkthrough with a
  generated guide.
- The review follow-up work (commits 7dea8ff, 8b15673, bd5092f, f5cf5a1,
  ce8a1cc) has no section and no gate in docs/plan.md. Its checks are lines
  in docs/checklist.md, none run yet.

Last commit: ce8a1cc. Gates at that commit: 31 unit, 151 behavior, 159 UI
tests, clippy clean, svelte-check clean, release build succeeded.

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
  git (moved 2026-11-12). It holds a build of ce8a1cc and the test vault
  the scripts expect (a guide with Stone Block, Oak Log, and related
  entries, boards, and media). It exists on this machine only.
- Screenshots of the last clean runs, at 19b8dae, are in
  `tools/walkthrough/run/shots-19b8dae/` (`drive`, `board`, `notes`,
  `paste`).
- `tools/walkthrough/README.md` has the build and run steps.
- To start in Dark without clicking: add `theme = "dark"` under `[ui]` in the
  run folder's `config.toml`.

### The three walkthrough scripts

- Where they are: `tools/walkthrough/` in the repository (`drive.ps1`,
  `board.ps1`, `notes.ps1`), committed 2026-11-12.
- What they are: PowerShell UI Automation. Each takes `-Exe` and `-OutDir`,
  drives the window, and saves numbered screenshots.
- How to run: `powershell.exe -STA -File <script> -Exe <exe> -OutDir <dir>`.
  The files must be CRLF with a UTF-8 byte order mark, or Windows PowerShell
  5.1 misreads them. `.gitattributes` keeps `*.ps1` as CRLF.
- Coverage: `drive.ps1` covers media (guide page, new entry, edit, paste and
  drop an image, the viewer, the tag editor, Trash, Settings, the palette).
  `board.ps1` covers boards (list, new board, picker, Add and Add all, drag,
  rename, delete). `notes.ps1` covers notes, check boxes, colors, keyboard
  tying, string label and color, backgrounds, delete with Undo, a pasted
  image, and reopening.
- Last clean run: against 19b8dae. They have not run against any later
  commit.
- Stale since 19b8dae (the scripts find controls by name and type):
  - The sidebar footer's Boards link is gone. Boards is a row in the tree.
  - The board header lost its layer toggles, Fit, and zoom. Mentions, Other
    guides, and Background are rows in a View menu. Zoom and Fit are a
    cluster at the canvas's bottom right.
  - The board panel is two halves, with Entry and Relations tabs.
  - Notices are at the bottom of the content, not under the top bar.
  - Deleting a relation shows no confirm dialog.
  - Confirm dialogs start with focus on Cancel.
  - A paste on the guide page opens New entry. It no longer sets the cover.
    The cover is set from More, "Set cover...".
  - Read or Edit and Posters or Table are pressed toggles now. Their
    UI Automation control type may have changed.
  - Settings choices are tiles and switches. Tag colors are swatches.
  - New entries show the "stub" chip. Editor line numbers are off.
  - Escape in a note saves.
- Not covered by any script: the Theme choice, Dark screenshots, the
  Vocabulary page, `[[` suggestions, web links, the Edit qualifiers
  suggestions, Tag merge confirm, the Trash keys.
- Script lessons: never call UI Automation SetFocus on the note editor (a
  blur saves and closes it). Menu rows with `role="menuitemradio"` are
  RadioButton to UI Automation. Confirm dialogs with their buttons, never
  with Enter. Wait in a retry loop for the first screen (up to 10 seconds).

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

### Reference files outside the repository

All in `C:\Users\DWats\.claude\projects\c--Users-DWats-Projects-field-guide-app\memory\reports\`:

- `review-ux-2026-11-10.md` and `review-ui-2026-11-10.md`: the two reviews.
  docs/ux-review.md in the repository is the older 2026-11-08 review.
- `plan-dark-mode-2026-11-10.md`: the dark mode plan.
- `design-trash-boards-guides-2026-11-11.md`: the Trash design for T2.
- `theme-contract.md`, `ux-fixes-contract.md`, `wave2-contract.md`,
  `board-decorations-contract.md`, `board-images-contract.md`: the briefs the
  subagents built from.

### Known rough edges and deferred fixes

- Nothing since 19b8dae has been seen in a running window. That is eight
  commits: 06b6638, d557267, 7dea8ff, 8b15673, bd5092f, f5cf5a1, ce8a1cc,
  and the docs inside them.
- The subagents asked for a look at: the dark title bar and no white frame
  at startup; the editor caret, selection, and gutter in Dark; the theme
  pictures in Settings; image dimming; the notice against the board's zoom
  cluster; the top bar with the sidebar folded (breadcrumbs may not line
  up); cork grain; dark caption text on picture cards; label pills on cork;
  pin placement on notes; alert icon alignment; drawn task boxes in Light;
  long titles in headers; segmented controls, switches, and tiles in both
  schemes; the editor's height in Edit; the `[[` list's position; Escape in
  a note (focus return, no double save); the browser launch from a web link;
  thumbnails in media cleanup.
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
- **Status:** todo (decision 1 answered (a) on 2026-11-12)
- **Goal:** See everything built since 19b8dae running, in both schemes, and
  list what is wrong. Close the open gate of "Dark mode and tokens".
- **Depends on:** none
- **Inputs:** walkthrough scripts (update first; they predate ce8a1cc);
  docs/checklist.md; Carry-over above (build recipe, stale list, the list of
  things to look at)
- **Done when:**
  - [ ] The three scripts are updated for the current controls and run to
        the end against a release build of the commit under test.
  - [ ] Screenshots exist in Dark and in Light of: the guides list; a guide
        page; an entry in Read with every markdown feature; the entry in
        Edit; the Tag editor; Trash; Vocabulary; Settings, Appearance; the
        palette; a confirm dialog; a board on dots; a board on cork with
        notes, strings, an index card, and a polaroid; the board panel's
        Entry and Relations tabs; the image viewer.
  - [ ] Every line of the Theme section of docs/checklist.md is marked
        passed or failed.
  - [ ] Each item under "The subagents asked for a look at" is marked fine
        or reported.
  - [ ] Every defect found is one line in the Inbox.
- **Out of scope:** fixing what it finds; the few-hundred-cards check
  (Later); the facts box (T5).
- **Notes:** If decision 1 is (b), I look first and this task starts from my
  list. The scripts, the run folder, and the earlier screenshots are in
  `tools/walkthrough/`.

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
  `design-trash-boards-guides-2026-11-11.md` (path under Carry-over), which
  has the schema, the core API, the rules, the tests by name, and four build
  batches; UX review failing item 9
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

## Inbox

Anything discovered or thought of mid-task, by either of us. The agent appends
and keeps working; I triage between tasks.

Format: `- [date] [agent|me] idea or problem — where it came up`

- [2026-11-12] [agent] Guess: editor shortens long media file names except on the line being edited; saved text unchanged. Nobody asked for it. Keep or remove?
- [2026-11-12] [agent] Guess: yellow note is slightly darker in Dark so links pass contrast. Keep or remove?

## Decisions

Small decisions that do not belong in docs/design.md.

Format: `- [date] decision — reason`

- [2026-11-12] `explorer` runs on Sonnet and `reviewer` inherits the main model — set by me in the agent files.
- [2026-11-12] Push to `main` before a new session starts — my instruction; no reason given.
- [2026-11-12] The walkthrough scripts live in `tools/walkthrough/`; the app copy, the test vault, and screenshots live in `tools/walkthrough/run/`, ignored by git — the temp folder they were in would not survive a cleanup. I chose (a); the session picked the folder.


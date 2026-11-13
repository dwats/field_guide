# T1 plan: walkthrough of ce8a1cc in Dark and Light

Status: done. Committed as beb7166 on 2026-11-12. The owner validated the not judged checks on 2026-11-13; the Theme section now stands at 13 passed, 2 failed. The results are in `docs/checklist.md` (Theme section) and in T1's Notes and the Inbox in `.claude/tasks.md`.

## Goal

See everything built since 19b8dae in a running window, in both schemes. List what is wrong. Close the open gate of "Dark mode and tokens" in `docs/plan.md`. Fix nothing.

## Answers (2026-11-12)

- The plan is approved. The agent updates the scripts and builds with no window, then stops and asks for the keyboard and mouse. Once given, it has as long as needed.
- The agent does not switch Windows between light and dark app mode. Three Theme lines are "not judged": "Dark with Windows in light mode", "System: switching Windows...", and "Light with Windows in dark mode".
- The web link check stops at the confirm dialog. The script answers Cancel. The browser launch is "not judged".
- Every "not judged" line is listed at the end of the task for the owner to validate. A validated line is marked off until code that affects it changes.

## Deviations from the first draft

- Vocabulary is covered by `board.ps1`, not `drive.ps1`. `board.ps1` makes a verb and a qualifier label, so the page has rows to show.
- "Set cover..." opens a dialog that takes a paste. There is no native file picker. The crop box and its slider are covered by `drive.ps1`.
- `-Theme` switches through the palette row "Theme: Dark" or "Theme: Light" at the start of each script.
- Screenshot folders: `tools/walkthrough/run/shots-ce8a1cc/<dark|light>/<drive|board|notes>/`.
- The restart checks in step 3.5 use a short script in the session's scratch folder. It is not committed.
- The scripts were rewritten whole. The helper block is the same text in all three files. New helpers: a miss counter and exit code, `Region`, `FindControl`, `OpenMenu`, `Palette`, `PasteText`, `ClipImage`, `Reveal`, `Dump`, a fixed window size, a refusal to start while the app runs, and a normal window close at the end.

## Commit under test

- ce8a1cc is the last commit that changed code. `git diff --stat ce8a1cc HEAD -- core app ui Cargo.toml Cargo.lock` is empty at 139bb1e.
- The build is made fresh from the head, so the binary is known to match. Screenshots go in `tools/walkthrough/run/shots-ce8a1cc/`.

## Approach

### Step 1: update the scripts from the UI source (no window)

Read the accessible names and roles in `ui/src` and change each stale lookup. The stale list in `.claude/tasks.md` is the starting point:

- Boards is a row in the sidebar tree, not a footer link.
- Mentions, Other guides, and Background are rows in the board's View menu.
- Fit and zoom are a cluster at the canvas's bottom right.
- The board panel has Entry and Relations tabs.
- Read or Edit and Posters or Table are pressed toggles.
- Settings choices are tiles (`role="radio"`) and switches. Tag colors are swatches.
- A note is saved with Escape or a click elsewhere.
- `notes.ps1` defines `Line` twice; one goes.

Control types that only a running window can confirm are fixed in step 3.

### Step 2: extend the scripts to cover the screenshot list

No fourth script. Each script keeps its subject.

| Script | Added |
|---|---|
| `drive.ps1` | The guides list. An entry with every markdown feature, pasted from a fixture file through the clipboard. A long title. `[[` suggestions. The image viewer. A web link dialog. Settings, Appearance, with the theme pictures, switches, and tiles. Settings, Media cleanup with thumbnails. A confirm dialog (Purge in Trash, answered Cancel). The palette's three Theme rows. The sidebar folded. The cover crop box, if the native file picker can be driven. |
| `board.ps1` | Vocabulary. The View menu. The zoom cluster with a notice showing. The panel's Entry and Relations tabs. A relation typed in the panel. Label pills. |
| `notes.ps1` | All four backgrounds. Five note colors. Four string colors. An index card (entry without a picture) and a polaroid on cork. Pins. Escape in a note. |

New file: `tools/walkthrough/fixtures/markdown.md`, the body with every markdown feature (headings, lists, task list, table, code, quote, nested quote, each alert kind, footnote, spoiler, anchor, wikilinks resolved and unresolved, a web link, images left, center, full, and inline).

New script parameter: `-Theme dark|light`. The script switches the theme through the palette row before the first screenshot. That also exercises the palette rows.

### Step 3: build and run (needs the keyboard and mouse)

1. `tasklist`: no `fieldguide.exe` running.
2. From `app/`: `cargo tauri build --no-bundle`.
3. New run folder `tools/walkthrough/run/t1/` with the exe and an empty `portable` file. The old vault in `run/` is not touched.
4. Run each script with `-Theme dark`, fix what it cannot find, repeat until it runs to the end. Then run each with `-Theme light`.
5. Restart checks, by a short separate run: `theme = "dark"` saved, a burst of screen captures during the first 3 seconds (white frame, title bar); an old `config.toml` without `theme`; "Dim images in dark" on.
6. Windows light and dark switch for three Theme lines (decision 2).

### Step 4: judge and record

- Read every screenshot. Mark each line of the Theme section in `docs/checklist.md` passed or failed. A failed line stays unchecked with a note beside it.
- Mark each item of "The subagents asked for a look at" fine or reported, in the Notes of T1 in `.claude/tasks.md`.
- One Inbox line per defect.
- A line that screenshots cannot prove (timing, motion) is marked "not judged" and listed for the owner to check by eye. It is not marked passed.

### Step 5: gates, review, wrap up

- Gates: `cargo test --workspace`, clippy, `cargo fmt --all`, `npm test`, svelte-check. No code changes are expected in `core`, `app`, or `ui`.
- `reviewer` subagent on the diff.
- Update T1 in `.claude/tasks.md`, `tools/walkthrough/README.md`, and the gate line in `docs/plan.md` if every Theme line passes. Commit.

## Files to touch

- `tools/walkthrough/drive.ps1`, `board.ps1`, `notes.ps1` (CRLF, UTF-8 with a byte order mark)
- `tools/walkthrough/fixtures/markdown.md` (new)
- `tools/walkthrough/README.md`
- `docs/checklist.md` (Theme section marks)
- `docs/plan.md` (only if the gate closes)
- `.claude/tasks.md` (T1 status and Notes, Inbox)
- Not touched: `core/`, `app/`, `ui/`

## Done criteria

Copied from T1.

- [x] The three scripts are updated for the current controls and run to the end against a release build of the commit under test.
- [x] Screenshots exist in Dark and in Light of: the guides list; a guide page; an entry in Read with every markdown feature; the entry in Edit; the Tag editor; Trash; Vocabulary; Settings, Appearance; the palette; a confirm dialog; a board on dots; a board on cork with notes, strings, an index card, and a polaroid; the board panel's Entry and Relations tabs; the image viewer.
- [x] Every line of the Theme section of `docs/checklist.md` is marked passed or failed. (11 passed, 2 failed, 2 not judged.)
- [x] Each item under "The subagents asked for a look at" is marked fine or reported.
- [x] Every defect found is one line in the Inbox.

## Risks

- The scripts take the keyboard and mouse. A click in another window makes `AssertForeground` stop the run. Estimate: about an hour in bursts, most of it fixing lookups.
- `PrintWindow` may not show the real title bar color. The title bar checks use a screen capture of the window's rectangle instead, so the window must not be covered.
- The white frame at startup lasts a few frames. A capture burst can miss it. That line may end as "not judged".
- (Resolved: "Set cover..." takes a paste; there is no native file picker, and the crop slider was captured.)
- A file drop cannot be sent by UI Automation. Dropped images are covered by paste only.
- CodeMirror closes brackets and indents on its own. Long bodies go in by clipboard paste, not typed keys.
- The scripts repeat one helper block each (356 lines after the rewrite). This task does not merge them (out of scope; Inbox).

## Open questions (all three answered; see Answers above)

1. When the keyboard and mouse are handed over.
2. Whether the agent may switch Windows between light and dark app mode for three Theme lines, and restore it.
3. Whether the web link check opens the default browser once.

## Run log

- 2026-11-12: `drive.ps1 -Theme dark` runs to the end with 0 misses on a fresh vault in `run/t1/`.
- 2026-11-12: `board.ps1 -Theme dark` ran to the end with 2 misses (the Mentions row in the View menu). Lookup fixed; the rerun was clean.
- The script parts are assembled from the session's scratch folder until all three run clean. The files in `tools/walkthrough/` are the result.

## Findings so far (Dark), to become Inbox lines at wrap-up

Defects, each seen in a screenshot or reproduced:

1. The Images album does not open on an entry whose body shows the same picture twice. Reproduced with a real mouse click. After that, the album on the next entry shows its backdrop and no dialog. Suspected cause, not verified: the album's list is keyed by the picture's address (`ui/src/components/ImageAlbum.svelte`, the `each` block).
2. The sidebar's entry list keeps the "stub" chip after an entry's body is first written. It clears when something else reloads the sidebar (a tag added, a navigation). Seen on Stone Block in `dark/board/b09` and on a probe entry.
3. A picture card's caption on a board is pale text on the off-white frame in Dark; it is hard to read. Seen in `dark/board/b09` and `b13` ("Cobblestone Block").
4. Settings, Data: a long path breaks in the middle of a word ("va" / "ult"). Seen in the first run's `43-settings-media-cleanup`.

Observations, not clearly defects:

- Board panel, Relations tab: after Save, the verb suggestions stay open over the relation list, and the list gets about one row of height above the picker (`dark/board/b13`).
- 2026-11-12: all three scripts run to the end with 0 misses in Dark and in Light, against the build in `run/t1/`. Screenshots are in `run/shots-ce8a1cc/<dark|light>/<drive|board|notes>/`.

More findings (Dark drive set judged up to `40-settings-appearance`):

5. Theme pictures: in Dark all three tiles (System, Light, Dark) picture a dark window; in Light all three picture a light window. Pixel samples of `40-settings-appearance` in both schemes confirm it. Theme checklist line 1 fails on its second sentence.
6. Board panel, a string selected: the title reads "Stringfrom Stone Block to note ..."; the space after "String" is missing (`dark/notes/n06`).
7. Entry header: the Read or Edit toggle, Images, and More move right when Edit folds the right panel (toggle at x 710 in Read, x 1001 in Edit; `dark/drive/20` and `21`). The checklist says they do not move.
8. Sidebar folded: the breadcrumbs start at x 116 and the content column at x 60; they do not line up (`dark/drive/22`).

More observations:

- After navigation by keyboard, the page heading shows a focus ring as a wide box (`dark/drive/20`, `40`).
- An info notice stays up across pages until dismissed or replaced; it covers the bottom rows of Settings and of the media cleanup list.
- Breadcrumbs shorten the guide names to "Gu..." and "Min..." to make room for a long entry title.

Passed so far in Dark: the editor's caret, selection, links, and muted markup; every markdown feature in Read; the spoiler is a gray block; alert icons line up; menus, the palette, and dialogs stand out; confirm dialogs start on Cancel; the image viewer's stage is black and its bar follows the theme; the crop slider and selects look native and dark; tag swatches.
- 2026-11-12: `drive.ps1` gained an album screenshot under Dim images and the palette row "Theme: System"; its final form ran clean in Dark and in Light.
- 2026-11-12: restart checks done. Dark saved: 40 screen captures from 85 ms to 1.5 s, none bright, title bar dark from the first full frame. Old config without `theme`: opens as System, dimming off. Captures that showed other windows were deleted; `dark/startup/` keeps two frames.
- Windows was in light app mode for every run (read from the registry, not changed). So "Dark with Windows in light mode" was judged; the two lines that need Windows in dark mode were not.
- Dimming measured from pixels: white becomes #D9D9D9 (85 percent) in entries, the album, covers, and boards in Dark; unchanged in the viewer and in Light.

Final list of findings: the Inbox in `.claude/tasks.md`. Final marks: `docs/checklist.md`, Theme section.

## Review (2026-11-12)

The `reviewer` subagent found no blocker. Fixed after it: README statements that did not match the scripts; the "Theme: System" step now runs only with `-Theme`; the focus-loss message; stale lines in this file; "Grey" in test text. Left for the owner as decisions 14 to 16 in `.claude/tasks.md`: the mark on "Dark with Windows in light mode", the mark on "no white frame", and a refusal to start without `portable`. Inbox lines cover the switches that are flipped without reading their state and the restart checks that have no committed script. The last two script edits (the `if ($Theme)` guard and the message text) were parsed, not run.

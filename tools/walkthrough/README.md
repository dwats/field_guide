# Walkthrough scripts

Four PowerShell UI Automation scripts that drive a release build of Field Guide and save numbered screenshots. They are a manual check, not a gate. Do not run them, or launch the app, until the owner says so (`.claude/AGENTS.md`).

| Script | Covers |
|---|---|
| `drive.ps1` | The guides list, the guide page, a new entry, an entry with every markdown feature in Edit and Read, `[[` suggestions, pasted images, the album and the image viewer, a web link dialog, a long title, the sidebar folded, Trash and a confirm dialog, the Tag editor, the palette, the cover and its crop box, Settings (Appearance, Editor, Data, Media cleanup), Dim images, a second launch. |
| `board.ps1` | Relations on an entry, Vocabulary, the board list, a new board, the picker, Add and Add all, the View menu, the panel's Entry and Relations tabs, a relation typed in the panel, nudge and remove with Undo, the four backgrounds, rename, the delete confirm, reopening. |
| `notes.ps1` | Board notes in five colors, a check box, a pasted image in its polaroid frame, strings in four colors tied by keyboard, the four backgrounds, an index card and a picture card, delete with Undo, Dim images on a board, reopening. |

| `trash.ps1` | Move guide to Trash with Undo, the pages inside a trashed guide reached by Back, a taken name, the guides' Trash with its purge and Empty trash dialogs, what a purge leaves in another guide, a board moved to Trash from the list and from its page, a guide's Trash with an entry and a board, the Vocabulary count for a relation into a trashed guide. Written 2026-11-13 and parsed; not run yet. |

The scripts take the keyboard, the mouse focus, and the clipboard while they run. If another window comes to the front, the script stops, sends no more keys, and closes the copy of the app it started.

## The run folder

`run/` is ignored by git. A run folder holds a copy of the app and an empty file named `portable`. With `portable` beside the executable, everything the app writes stays in that folder, and the real data directory is never touched.

The scripts need no existing data. `drive.ps1` opens the first guide whose name starts with "Minecraft" and makes one if there is none; `board.ps1` and `notes.ps1` each make a guide named with the time of the run. An empty folder with the app and `portable` is enough to start.

`trash.ps1` is the exception. It needs three guides tied to each other by relations, qualifier values, board cards, and strings, and the app has no control that makes those across guides. `seed/` is a small program that writes that vault through `core`. It is not a member of the workspace, so it is not part of the build or the gates. Each run of `trash.ps1` purges guides, so seed a new vault before every run:

```
cargo run --offline --manifest-path tools\walkthrough\seed\Cargo.toml --target-dir target\seed -- tools\walkthrough\run\t2\vault\vault.db
```

The seed refuses a vault file that exists; remove the run folder's `vault` folder first. It prints the counts core gives for each guide. The dialog lines `trash.ps1` checks are those counts; change both together.

- `run/t1/` is the folder for the T1 walkthrough of ce8a1cc.
- `run/` itself holds an older build and the vault that earlier runs left behind, one guide per run.
- `run/shots-19b8dae/` holds the screenshots of the last clean runs at commit 19b8dae.

## Build and run

1. Check that no `fieldguide.exe` is running (`tasklist`). The app is single-instance, and a running copy locks its executable. The scripts refuse to start while one runs.
2. From `app/`: `cargo tauri build --no-bundle`. This needs the Tauri command line tool. A plain `cargo build --release` loads the dev address and shows an error page.
3. Copy `target/release/fieldguide.exe` into the run folder.
4. Run one script, once per scheme:

   ```
   powershell.exe -STA -File tools\walkthrough\drive.ps1 -Exe tools\walkthrough\run\t1\fieldguide.exe -OutDir tools\walkthrough\run\shots-<commit>\dark\drive -Theme dark
   ```

`-Theme dark` or `-Theme light` switches the theme through the palette row before the first screenshot. Without it the script keeps the saved theme and skips the "Theme: System" step. To start in a scheme without the palette, set `theme = "dark"` under `[ui]` in the run folder's `config.toml`.

## Reading a run

- Each control the script cannot find or click prints a line that starts with `MISS:`. The script keeps going.
- The last line is `done, misses: <n>`. The exit code is 0 with no miss; 2 with at least one, or when the guides list never appeared; 3 when another window took focus; 1 when the script did not start the app (no executable, no `-OutDir`, the app already running, or no window).
- `Dump <element> <label>` prints the names and control types under an element. Add a call where a control is not found.

## Not scripted

- A file drop, resizing a note, the browser launch after "Open in browser", and switching Windows between light and dark.
- The two restart checks of the Theme section (Dark saved with no white frame at startup; an old `config.toml` without `theme`). T1 ran them from a one-off script that is not in the repository.
- The scripts flip "Line numbers" and "Dim images in dark" by a click and do not read their state first. After a run that stopped early, check both in Settings before the next run.
- The scripts do not check that a file named `portable` sits beside `-Exe`. Without it they would drive the real data directory.

## Fixture

`fixtures/markdown.md` is the body with every markdown feature. `drive.ps1` fills in `{{OAK}}` (an entry title) and `{{IMG1}}` to `{{IMG5}}` (media references it reads back from the editor after pasting pictures), then pastes the text.

## Rules for editing the scripts

- The files are UTF-8 with a byte order mark and CRLF line endings; Windows PowerShell 5.1 misreads them otherwise. `.gitattributes` keeps the line endings. Keep the text ASCII.
- The helper block at the top is the same in all four files. Change it in all four.
- A function must not share its name with a PowerShell alias (`move`, `type`, `select`); the alias wins.
- Never call UI Automation `SetFocus` on a board note's editor. A blur saves the note and closes the editor.
- A menu row with `role="menuitemradio"` is a RadioButton to UI Automation. `MenuItem` tries each kind.
- Confirm a dialog by its button, never with Enter. Confirm dialogs start with focus on Cancel.
- Long text goes in through the clipboard (`PasteText`). The editor pairs brackets and indents typed keys.
- Wait for the first screen in a retry loop. The helper block waits up to 15 seconds.

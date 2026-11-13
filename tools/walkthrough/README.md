# Walkthrough scripts

Three PowerShell UI Automation scripts that drive a release build of Field Guide and save numbered screenshots. They are a manual check, not a gate. Do not run them, or launch the app, until the owner says so (`.claude/AGENTS.md`).

| Script | Covers |
|---|---|
| `drive.ps1` | The guide page, a new entry, Edit, a pasted and a dropped image, the image viewer, the Tag editor, Trash, Settings, the palette. |
| `board.ps1` | The board list, a new board, the picker, Add and Add all, dragging, rename, delete. |
| `notes.ps1` | Board notes, check boxes, colors, tying a string by keyboard, string label and color, backgrounds, delete with Undo, a pasted image, reopening. |

Which controls the scripts no longer find, and what no script covers yet, is tracked in `.claude/tasks.md`.

## The run folder

`run/` is ignored by git. It holds a copy of the app and an empty file named `portable`. With `portable` beside the executable, everything the app writes stays in `run/`, and the real data directory is never touched.

The scripts need no existing data. `drive.ps1` makes a guide named "Minecraft" if there is none; `board.ps1` and `notes.ps1` each make a guide named with the time of the run. An empty `run/` with the app and `portable` is enough to start. The vault in `run/` is what earlier runs left behind, one guide per run.

`run/shots-19b8dae/` holds the screenshots of the last clean runs, at commit 19b8dae.

## Build and run

1. Check that no `fieldguide.exe` is running (`tasklist`). The app is single-instance, and a running copy locks its executable.
2. From `app/`: `cargo tauri build --no-bundle`. This needs the Tauri command line tool. A plain `cargo build --release` loads the dev address and shows an error page.
3. Copy `target/release/fieldguide.exe` into `tools/walkthrough/run/`.
4. Run one script:

   ```
   powershell.exe -STA -File tools\walkthrough\drive.ps1 -Exe tools\walkthrough\run\fieldguide.exe -OutDir tools\walkthrough\run\shots-<commit>\drive
   ```

To start in the dark scheme without clicking, add `theme = "dark"` under `[ui]` in `run/config.toml`.

## Rules for editing the scripts

- The files are UTF-8 with a byte order mark and CRLF line endings; Windows PowerShell 5.1 misreads them otherwise. `.gitattributes` keeps the line endings.
- Never call UI Automation `SetFocus` on a board note's editor. A blur saves the note and closes the editor.
- A menu row with `role="menuitemradio"` is a RadioButton to UI Automation.
- Confirm a dialog by its button, never with Enter. Confirm dialogs start with focus on Cancel.
- Wait for the first screen in a retry loop, up to 10 seconds.

# Manual checklist

Run before calling a UI phase done. Core behavior is covered by `cargo test`; this list covers what only a window shows.

## How to run the app for these checks

- `cargo tauri dev` for day-to-day work. It starts the Vite dev server and a debug window with live reload.
- The debug binary in `target/debug/` is not standalone: it loads the UI from the dev server and shows a 404 page without it.
- `cargo tauri build` produces a standalone `target/release/fieldguide.exe`. Copy it anywhere. A file named `portable` beside it keeps all data beside it, which is the easy way to test with a fresh or unusual data directory.
- Sync warning: copy the release binary and a `portable` file into a folder under OneDrive or Dropbox, launch, and the warning shows. Launch again and it does not.
- Vault lock: hold the database from another process, then launch Field Guide. From PowerShell, with the path adjusted:

  ```
  python -c "import sqlite3, time; c = sqlite3.connect(r'C:\Users\me\AppData\Local\FieldGuide\vault\vault.db'); c.execute('PRAGMA locking_mode = EXCLUSIVE'); c.execute('BEGIN IMMEDIATE'); c.execute('SELECT count(*) FROM note').fetchone(); time.sleep(300)"
  ```

## Shell

- [ ] First launch creates the data directory layout (`config.toml`, `vault/vault.db`, `vault/media/`, `backups/`) and one backup for today.
- [ ] A second launch while the first is open focuses the first window and exits.
- [ ] With a file named `portable` beside the executable, Settings shows "Portable" and the data path beside the executable.
- [ ] Settings shows "Default" and the platform path otherwise.
- [ ] Starting from a data directory under a OneDrive or Dropbox folder shows the sync warning once; restarting does not show it again.
- [ ] Closing and reopening the window restores its size and position without first appearing elsewhere.
- [ ] A vault that another process holds open produces the "open in another Field Guide" screen, not a blank window.

## Guides

- [ ] Create a guide; it appears in cards and table views; the view choice survives a restart.
- [ ] Name filter narrows the list as you type.
- [ ] Add a global tag to a guide from Edit guide; the tag chip filters the guides list.
- [ ] Copy vocabulary from another guide when creating one (checkable once the relation builder exists in phase 5).

## Notes

- [ ] Create a note from the guide page and land on it.
- [ ] Edit mode shows the editor and a live preview that updates while typing; "Saved" appears after a pause.
- [ ] Leaving the page while unsaved saves the body.
- [ ] A `[[Title]]` link to an existing note renders as a link and navigates; a link to a missing note renders dashed and offers to create it.
- [ ] Rename a note; a note that linked to it shows the new title; "Undo rename" puts the old title back and removes the alias.
- [ ] Rename a note and then rename it back by typing the original title; the original title returns and the interim title becomes the alias.
- [ ] Add and remove an alias; linking by alias resolves; the note filter finds the note by its alias.
- [ ] Tag a note by typing a new path such as `Blocks/Natural`; the path appears under Tags for the guide.
- [ ] The guide note list filters by text and by `#Tag/Path`, and "Stubs only" hides described notes.
- [ ] Delete moves a note to the trash after confirmation and leaves the guide page (Trash view arrives in phase 6).

## Tags

- [ ] Rename, add child, delete with children (children move up a level).
- [ ] Drag a tag onto another; it nests under it and note tag paths follow.
- [ ] Drop a nested tag on the top-level bar; it becomes top-level.
- [ ] Dragging a tag onto its own child is refused with a message.

## Settings

- [ ] "Back up now" adds a backup; a sixth removes the oldest.
- [ ] Restore the oldest backup; it replaces the vault after taking one more backup; the guides list reflects the restored state.
- [ ] "Check for updates" is present and looks disabled.

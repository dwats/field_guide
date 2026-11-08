# Manual checklist

Run before calling a UI phase done. Core behavior is covered by `cargo test`; this list covers what only a window shows.

## Shell

- [ ] First launch creates the data directory layout (`config.toml`, `vault/vault.db`, `vault/media/`, `backups/`) and one backup for today.
- [ ] A second launch while the first is open focuses the first window and exits.
- [ ] With a file named `portable` beside the executable, Settings shows "Portable" and the data path beside the executable.
- [ ] Settings shows "Default" and the platform path otherwise.
- [ ] Starting from a data directory under a OneDrive or Dropbox folder shows the sync warning once; restarting does not show it again.
- [ ] Closing and reopening the window restores its size and position.
- [ ] A vault that another process holds open produces the "open in another Field Guide" screen, not a blank window.

## Guides

- [ ] Create a guide; it appears in cards and table views; the view choice survives a restart.
- [ ] Name filter narrows the list as you type.
- [ ] Add a global tag to a guide from Edit guide; the tag chip filters the guides list.
- [ ] Copy vocabulary from another guide when creating one (visible once the relation builder exists).

## Notes

- [ ] Create a note from the guide page and land on it.
- [ ] Edit mode shows the editor and a live preview that updates while typing; "Saved" appears after a pause.
- [ ] Leaving the page while unsaved saves the body.
- [ ] A `[[Title]]` link to an existing note renders as a link and navigates; a link to a missing note renders dashed and offers to create it.
- [ ] Rename a note; a note that linked to it shows the new title; "Undo rename" puts the old title back and removes the alias.
- [ ] Add and remove an alias; linking by alias resolves.
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
- [ ] Restore replaces the vault after taking one more backup; the guides list reflects the restored state.
- [ ] "Check for updates" is present and disabled.

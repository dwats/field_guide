# Manual checklist

Run before calling a UI phase done. Core behavior is covered by `cargo test`; this list covers what only a window shows.

## How to run the app for these checks

- `cargo tauri dev` for day-to-day work. It starts the Vite dev server and a debug window with live reload.
- The debug binary in `target/debug/` is not standalone: it loads the UI from the dev server and shows a 404 page without it.
- `cargo tauri build` produces a standalone `target/release/fieldguide.exe`. Copy it anywhere. A file named `portable` beside it keeps all data beside it, which is the easy way to test with a fresh or unusual data directory.
- Sync warning: copy the release binary and a `portable` file into a folder under OneDrive or Dropbox, launch, and the warning shows. Launch again and it does not.
- Vault lock: hold the database from another process, then launch Field Guide. From PowerShell, with the path adjusted:

  ```
  python -c "import sqlite3, time; c = sqlite3.connect(r'C:\Users\me\AppData\Local\FieldGuide\vault\vault.db'); c.execute('PRAGMA locking_mode = EXCLUSIVE'); c.execute('BEGIN IMMEDIATE'); c.execute('SELECT count(*) FROM entry').fetchone(); time.sleep(300)"
  ```

## Shell

- [ ] First launch creates the data directory layout (`config.toml`, `vault/vault.db`, `vault/media/`, `backups/`) and one backup for today.
- [ ] A second launch while the first is open focuses the first window and exits.
- [ ] With a file named `portable` beside the executable, Settings shows "Portable" and the data path beside the executable.
- [ ] Settings shows "Default" and the platform path otherwise.
- [ ] Starting from a data directory under a OneDrive or Dropbox folder shows the sync warning once; restarting does not show it again.
- [ ] Closing and reopening the window restores its size and position without first appearing elsewhere.
- [ ] A vault that another process holds open produces the "open in another Field Guide" screen, not a blank window.

## Sidebar and Trash

- [ ] Inside a guide, the sidebar shows the search line, Stubs only, the tag tree with All entries at the top and Untagged and Trash at the bottom, and the entries of the selected scope below a splitter.
- [ ] Each tag row shows a count that includes the tags under it; a tag the open entry carries shows a dot.
- [ ] Selecting a tag narrows the entry list and the guide page table to the entries under it; the guide page summary line names the scope and the count, and Clear returns to All entries.
- [ ] Typing in the search narrows the list on each keystroke and prunes the tree to matching tags with their parents open; clearing it restores the tree.
- [ ] Opening an entry from a wikilink moves the scope to the entry's first tag when the current scope does not hold it; opening one from the list keeps the scope.
- [ ] The sidebar collapses to a rail and expands again; the width and the splitter position survive a restart.
- [ ] Trash lists trashed entries with their deletion time and tags; Restore puts one back with its tags and relations and offers Undo; Purge and Empty trash ask first.
- [ ] Breadcrumbs in the top bar read Guides / guide / page; Back and Forward work.
- [ ] Edit guide opens a modal; Delete guide asks first.

## Right panel

- [ ] An entry with headings opens on the Contents tab; a click scrolls the body to that heading. An entry without headings opens on Relations with Contents disabled.
- [ ] The Relations tab holds the relation line at the top, the grouped list, Edit qualifiers in a dialog, Show external, and the Mentioned by and Unresolved links folds, which are absent when empty.
- [ ] Hide panel folds it to a rail and the choice survives a restart; Show panel brings it back.
- [ ] Switching to Edit folds the panel; the rail's button opens it for that edit only; the Settings toggle keeps it open always.
- [ ] Settings, Appearance: Compact and Roomy change type size and row heights everywhere; the two entry page toggles show their effect in their pictures and apply at once.
- [ ] A quote (`> text`) shows a bar down its left and an opening quote mark; nested quotes add a bar each.

## Palette, keys, and lists

- [ ] Ctrl+K (Cmd+K on macOS) or the Search button opens the palette; typing lists matching entries with their tag paths, commands with their shortcuts, and guides; Enter opens the highlighted row; Escape closes; a typed name with no exact match offers "Create entry".
- [ ] `>` in the palette limits the list to commands.
- [ ] Ctrl+N opens New entry inside a guide and New guide on the guides list; Ctrl+E toggles Read and Edit; Ctrl+R puts the caret in the relation line (opening the panel if folded); Ctrl+T opens Add tag; Ctrl+F focuses the sidebar search; Ctrl+, opens Settings; Alt+Left and Alt+Right go back and forward; F2 opens Rename outside a text field.
- [ ] Guides list: the name filter and the global tags sit in the sidebar with guide counts; the toolbar holds the view toggle and New guide; "No guides match." offers Clear filter.
- [ ] Tag editor: selecting a row fills the detail pane; Rename, Parent, and Colour apply at once; Add child tag and New tag open a dialog; Merge into... lists target paths and merges on pick; Delete tag asks first; arrow keys, F2, and Delete work in the tree; a coloured tag shows a dot in the sidebar, chips, and cards.
- [ ] Settings: the sidebar lists the sections and marks the one in view; a click scrolls to it.

## Guides

- [ ] Create a guide; it appears in cards and table views; the view choice survives a restart.
- [ ] Name filter narrows the list as you type.
- [ ] Add a global tag to a guide from Edit guide; the tag chip filters the guides list.
- [ ] Copy vocabulary from another guide when creating one (checkable once the relation builder exists in phase 5).

## Entries

- [ ] Create an entry from the guide page and land on it.
- [ ] Edit mode shows the editor and a live preview that updates while typing; "Saved" appears after a pause.
- [ ] Leaving the page while unsaved saves the body.
- [ ] A `[[Title]]` link to an existing entry renders as a link and navigates; a link to a missing entry renders dashed and offers to create it.
- [ ] Rename an entry; an entry that linked to it shows the new title; "Undo rename" puts the old title back and removes the alias.
- [ ] Rename an entry and then rename it back by typing the original title; the original title returns and the interim title becomes the alias.
- [ ] Add and remove an alias; linking by alias resolves; the entry filter finds the entry by its alias.
- [ ] Tag an entry by typing a new path such as `Blocks/Natural` and choosing the Create row; the path appears under Tags for the guide. Enter on a typed prefix picks the highlighted match.
- [ ] The guide entry list filters by text and by `#Tag/Path`, and "Stubs only" hides described entries.
- [ ] More, Move to Trash moves the entry without a confirmation, returns to the guide page, and offers Undo in the notice; Back does not return to the trashed entry.
- [ ] Rename shows a notice with Undo; Edit aliases opens a dialog; the aliases line appears only when there are aliases.
- [ ] In Edit mode the status reads Unsaved, then Saved, with no third state; the More button does not move between modes.

## Relations

- [ ] On Stone Block, type `drops` and pick "Create verb", inverse `dropped by`; type `Cobblestone Block` and create it as a new entry; type `mined with`, create the label with inverse `mines`; type `Pickaxe` and create it; press Enter. The line reads `DROPS Cobblestone Block · mined with Pickaxe`.
- [ ] Cobblestone Block reads `DROPPED BY Stone Block · mined with Pickaxe`; Pickaxe reads `MINES Stone Block → Cobblestone Block`.
- [ ] From Cobblestone Block, type `dropped by` then `Stone Block` and save; no second edge appears on either page.
- [ ] Backspace on the empty input removes the last staged item; Escape clears the line.
- [ ] Qualifiers on an edge: add one, change a value, remove one; all three pages follow.
- [ ] Delete an edge; it disappears from all three pages.
- [ ] The stub created for Cobblestone Block shows the "stub" chip until its body is edited.

## Tags

- [ ] Rename, add child, delete with children (children move up a level) through the in-app confirm dialog, not the browser's.
- [ ] Drag a tag onto another; it nests under it and entry tag paths follow.
- [ ] Drop a nested tag on the top-level bar; it becomes top-level.
- [ ] Dragging a tag onto its own child is refused with a message.

## Settings

- [ ] "Back up now" adds a backup; a sixth removes the oldest.
- [ ] Restore the oldest backup through the in-app confirm dialog; it replaces the vault after taking one more backup; the guides list reflects the restored state.
- [ ] Notices show one at a time with "1 of n" and Next when several are queued; a warning stays until dismissed; an info notice is replaced by the next one.
- [ ] "Check for updates" is present and looks disabled.

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
- [ ] In Trash, Enter restores the focused row and Delete asks before deleting it for good, with focus on Cancel.
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
- [ ] Tag editor: selecting a row fills the detail pane; Rename, Parent, and Color apply at once; Add child tag and New tag open a dialog; Merge into... lists target paths and merges on pick; Delete tag asks first; arrow keys, F2, and Delete work in the tree; a colored tag shows a dot in the sidebar, chips, and cards.
- [ ] Tag editor, Merge into...: Enter with a partial path and no row chosen does nothing; picking a tag asks first, naming both tags and counting the entries and child tags that move, with focus on Cancel. The tree opens on the tag selected in the sidebar.
- [ ] Settings: the sidebar lists the sections and marks the one in view; a click scrolls to it.

## Images

- [ ] Paste a screenshot while the editor has focus: the reference lands at the caret, the preview shows the image, and the stored file is a `.webp` in the media folder.
- [ ] Drop an image file onto the editor: it lands where dropped; drop one onto the page in Read mode: it is appended to the body.
- [ ] Drop the same picture twice, once as PNG and once as BMP: one file in the media folder.
- [ ] An animated GIF stays a `.gif` and still animates; an SVG stays an `.svg`.
- [ ] Click anywhere on a floated figure, including the part beside a quote or an alert: the viewer opens.
- [ ] Paste the first image into an entry that has no gallery: a board card for the entry shows it; Make primary image on another image changes the card.
- [ ] The Images button in the entry header opens the album: every image as a thumbnail, the primary marked; a click opens the viewer on it; the button is disabled when the entry has no images.
- [ ] Click an image, or Tab to it and press Enter: the viewer opens over the page with the counter, caption, and file details; Tab stays inside the viewer; Left and Right move between the body's images; Escape closes; Make primary image puts it first in the gallery.
- [ ] Drop or paste an image on a guide page: it becomes the cover and the crop box opens; drag and zoom, Save; the poster on the guide page and on the card shows that frame; Adjust cover... in More reopens the box; Remove cover undoes it with Undo.
- [ ] An image alone on its line in a body renders as a bordered figure floated right with its alt text as caption and text wrapping beside it; `"left"` as the image title floats it left, `"center"` centers it, `"full"` spans the width; an image inside a sentence stays inline.
- [ ] The guides list shows 2:3 posters in rows that fill the window from the left, titles below; a guide without a cover shows a placeholder poster.
- [ ] A dropped file that is not an image is skipped with a notice; a drop on the guides list or Settings shows where drops go.
- [ ] Tag editor: drag a tag onto another with the mouse to nest it, and onto the Top level bar to move it up; a click without movement only selects.
- [ ] Settings, Media: the totals line counts files and unused bytes; Clean up unused media lists only files no body, gallery, or cover refers to; Remove asks first and removes exactly those; the threshold field changes when the start-up notice appears.
- [ ] Settings, Media: each file in the cleanup list shows a thumbnail, and a click opens it in the image viewer.
- [ ] At start, a notice appears when unused media exceeds the threshold, and another when the drive has under 1 GB free; each opens Settings.
- [ ] Sidebar: with a tag row focused, Up and Down move between rows, Right opens a tag or enters its children, Left closes it or goes to its parent, Home and End jump, typing a letter goes to the next row that starts with it; the entry list takes Up and Down and letters too.

## Boards

- [ ] The sidebar tree has a Boards row above Trash that expands to the guide's boards; a board's row opens it; the Boards row and the palette's Boards command open the guide's board list; New board asks for a name and opens the board; a name already used by another board of the guide is refused with a message.
- [ ] The picker panel lists the guide's entries; the search line narrows by text and `#Tag` chips, Stubs only narrows further; Add puts a card in the middle of the window, Add all fills the cells around it; an entry already on the board shows a check and Remove instead of Add.
- [ ] Two cards whose entries are related show a line between them labeled with the verb, qualifiers on a second line, an arrowhead at the object unless the verb is symmetric; a body that mentions another member draws a dashed line; the Mentions toggle hides and shows that layer.
- [ ] Adding a card whose entry is related to members already there draws those lines at once.
- [ ] An entry from another guide added to the board shows only with Other guides on, with a dashed frame and the edge in the accent color; the count in the header says how many are hidden.
- [ ] Dragging a card moves it and the lines follow; after a restart the card is where it was left. The wheel zooms around the pointer, dragging the background pans, Fit shows every card, and clicking the zoom readout returns to 100%.
- [ ] A card takes focus: Enter or Space opens the entry, Delete removes the card with Undo in the notice and focus moves to the canvas, arrows nudge it, Escape clears the selection; Tab to a card off the edge pans it into view; with the canvas focused, arrows pan and plus and minus zoom; a second click on a card opens the entry.
- [ ] Hide panel on the board folds it to a rail and the choice survives a restart, shared with the entry page's panel; the panel's own fold button opens and closes it, and the header's plus does not.
- [ ] The board header shows the whole name; View holds Mentions, Other guides, and the background; zoom out, the readout, zoom in, and Fit sit at the canvas's bottom right and a notice does not cover them; leaving the board and coming back keeps its pan and zoom.
- [ ] A line's label is horizontal in a pill at the middle of the line, readable on every background in both schemes; a card without a picture is an index card with a red rule.
- [ ] The selected card's panel has Entry and Relations tabs; typing a relation in the panel draws its line on the board at once; Ctrl+R puts the caret in the relation line; Ctrl+F puts it in the picker's search.
- [ ] The selected card shows in the panel with its relations, Open entry, and Remove from board; a card with a gallery shows its first image as the picture, proportions following the image; one without shows the title alone.
- [ ] Trashing a member entry takes its card and lines off the board; restoring brings them back in place.
- [ ] The picker's search line, chips, and Stubs only come back when the board is reopened; Rename board and Delete board (asks first) work from More on the board page and from the list.
- [ ] Export and import a guide with a board: the board, its filters, the card positions, the notes, the strings, and the background survive the round trip.
- [ ] The header's plus makes a square note at the middle of the window and opens it for editing; typing markdown with a task list and clicking elsewhere saves and shows the rendered note; Escape saves too and focus returns to the note; double-click edits again; clicking a check box flips it and the change survives a reload; the panel's color swatches recolor the note; Delete removes it with Undo.
- [ ] A note resizes from its corners on both axes and from its edges on one; Shift on a corner keeps the proportions; the size survives a reload; long content scrolls inside the note.
- [ ] Drop an image file onto the board: it appears in a polaroid frame at the drop point, 200 wide; paste one: it lands at the middle; drag, corner-resize (proportions kept), tie a string to it, Delete with Undo; after a reload it is where it was; Settings, Media does not list its file as unused.
- [ ] With a card or a note selected, dragging its pin onto another item ties a string drawn as a sagging curve above the relation lines; the panel's label and color apply to it; a click on the string selects it; Delete removes it with Undo; "Tie a string" in the panel does the same from the keyboard.
- [ ] Removing a card or deleting a note takes its strings with it.
- [ ] More, Background: Dots, Grid, Cork, and Blank change the canvas, follow light and dark, pan with the board, and come back after a reload.

## Guides

- [ ] Create a guide; it appears in cards and table views; the view choice survives a restart.
- [ ] Name filter narrows the list as you type.
- [ ] Add a global tag to a guide from Edit guide; the tag chip filters the guides list.
- [ ] Copy vocabulary from another guide when creating one (checkable once the relation builder exists in phase 5).

## Entries

- [ ] A long entry title wraps on its own line and is never cut off; the controls sit on the line below and do not move between Read and Edit.
- [ ] Paste an image on the guide page: New entry opens with the image attached; the cover changes only by a drop on the cover or More, Set cover.
- [ ] After opening an entry from the palette, focus is on the page heading, not on the element that had it before; Tab from the sidebar search reaches the body in a few presses.
- [ ] A notice appears over the bottom of the content and nothing on the page moves when it comes or goes.
- [ ] Create an entry from the guide page and land on it; it shows the "stub" chip and is listed by Stubs only until its body is edited.
- [ ] A click on a web link in a body asks first, showing the whole address with focus on Cancel; Open in browser opens the default browser; the app itself makes no request.
- [ ] The right panel keeps the last chosen tab and the open folds from one entry to the next.
- [ ] Line numbers are off in the editor; Settings, Editor turns them on at once.
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
- [ ] Delete an edge, by its button or the Delete key on the selected row; it disappears from all three pages without a question, and Undo in the notice brings it back with its qualifiers in order.
- [ ] Edit qualifiers: the value field suggests entries, "Use as text", and "Create as a new entry"; a click on the backdrop after a change does not close the dialog.
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

## Theme

- [ ] Settings, Appearance shows Theme first, with System, Light, and Dark pictured. The pictures show a light, a dark, and a split window in either scheme.
- [ ] Dark with Windows in light mode: the page, the sidebars, the panel, and the title bar turn dark at once, with no reload.
- [ ] Restart with Dark saved: no white frame appears before the first paint, and the title bar is dark from the start.
- [ ] System: switching Windows between light and dark changes the app while it runs.
- [ ] Light with Windows in dark mode: the app is light, and so is the title bar.
- [ ] The palette rows "Theme: System", "Theme: Light", and "Theme: Dark" work, and Settings reflects them.
- [ ] Editor in dark: the caret is visible, the selection is readable, the gutter is dark, links are blue, and markup marks are muted.
- [ ] An entry using every markdown feature reads in dark; a hidden spoiler is a gray block, not a bright one.
- [ ] Menus, the palette, and every dialog stand out from the page in dark.
- [ ] Text fields and selects have an edge that is easy to see in both schemes; buttons have a lighter one; dividers are the faintest.
- [ ] Board in dark on each of the four backgrounds: cards, five note colors, text and links in notes, polaroids (warm off-white), relation and mention edges, all four string colors including black, pins, and handles.
- [ ] Image viewer: the stage is black in both schemes and the bottom bar follows the theme.
- [ ] "Dim images in dark" on: pictures in entries, galleries, covers, and boards are slightly darker in dark and unchanged in light; the image viewer shows them at full brightness.
- [ ] Check boxes, the crop zoom slider, selects, and the number field look native and dark.
- [ ] An old `config.toml` without `theme` opens as System with dimming off.

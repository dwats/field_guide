# UX review

The owner decided on the open questions and on the change list on 2026-11-08; `docs/design.md` section 5, "Interface principles", records the decisions and wins where it differs from this review (tabs with Contents first; the panel collapses in Edit; contents shown from one heading; manual boards).

Reviewed 2026-11-08 against commit `1bcbb25` (phases 0 to 5) and the screenshots of the portable build taken 2026-11-07. The screenshots predate commit `95a4c38`, so they still say "note" where the source now says "entry"; the source is clean. Fixed requirements come from the owner interview recorded in `docs/design.md` section 5, "Interface principles". Every proposal below is designed for those requirements. Where this review disagrees with one, it says so once, in section 7.

Terms used throughout: **top bar**, **sidebar** (left), **content** (middle column), **right panel**, **relation line** (the staged input), **relation list**, **command palette**, **notice**, **More menu**, **Read mode**, **Edit mode**, **Split**, **In place**, **Tag editor**, **Trash**. **Mod** means Ctrl on Windows and Linux and Cmd on macOS. Effort: S is under half a day, M is one to three days, L is a week or more.

## 1. Summary

1. Build the three-column shell (sidebar, content, right panel) with one top bar, and never change column widths when the mode or the view changes.
2. Move the relation line and the relation list into the right panel, give the line a visible label and a stage hint, and render suggestions in flow so nothing is covered.
3. Replace every hover-only action and every native `confirm()` with selection plus a fixed action row and an in-app confirm dialog; confirm only purge, empty trash, delete relation, delete tag, restore backup, and media cleanup.
4. Fix the copy that leaks internals: the `"X" is already used by entry 018f...` message, the lowercase fatal-screen strings, the stale-route "not found" error, and the "Delete" button that means "Move to Trash".
5. Add the command palette on Mod+K and the keyboard map in section 4, and turn off the webview's own accelerator keys so F5 and Ctrl+R stop reloading the app.

## 2. Information architecture

### 2.1 The shell

One window, one top bar, three columns. The top bar is split into three segments aligned to the columns below it, so the breadcrumbs sit exactly above the content and the vault-level controls sit above the right panel.

```
+----------------------+------------------------------------------------+------------------------+
| [<] [>]              | Guides / Minecraft / Stone Block               | [Search    Mod+K] [Settings] |
+----------------------+------------------------------------------------+------------------------+
| notice slot: one notice at a time, full width, present only when there is one          [Dismiss] |
+----------------------+------------------------------------------------+------------------------+
| SIDEBAR              | CONTENT                                        | RIGHT PANEL            |
| [Search this guide ] | Stone Block           [Read|Edit]  Saved [More]| Add relation           |
| [ ] Stubs only       | Blocks/Natural   + Add tag                     | [verb, e.g. DROPS    ] |
| All entries     124  |------------------------------------------------| DROPS                  |
| v Blocks         12  | body text, capped at --measure                 |   Cobblestone Block    |
|     Natural       6  |                                                |     . mined with Pickaxe|
|     Crafted       6  |                                                | Referenced by          |
| > Items          40  |                                                | > Mentioned by (1)     |
| > Mobs           30  |                                                |------------------------|
|                      |                                                | On this page           |
| Boards               |                                                |   Overview             |
| Tag editor           |                                                |   Drops                |
| Trash             2  |                                                |                        |
+----------------------+------------------------------------------------+------------------------+
```

Regions and their rules:

- **Top bar.** 36 px. Left segment: Back and Forward buttons. Middle segment: breadcrumbs. Right segment: a "Search" button that opens the command palette (its label shows the shortcut) and "Settings". Nothing else. The brand text "Field Guide" is dropped; the window title already carries it.
- **Notice slot.** Directly under the top bar, full width. Holds one notice at a time. Present only when a notice exists. Section 4.6 has the rules.
- **Sidebar.** Width `--sidebar-w` (section 3.8), resizable by a drag handle between 200 px and 360 px, remembered in `config.toml`. Contents depend on the level: at the vault level it filters guides; inside a guide it holds the search field, the "Stubs only" toggle, the tag tree, and a footer with Boards, Tag editor, and Trash. In Settings it lists the sections. The sidebar is always present at widths of 760 px and above; below that it collapses to a button in the top bar's left segment.
- **Content.** Fluid width, fills what the other two columns leave. Body text is capped at `--measure`; lists and tables use the full width. The content column scrolls on its own. Its title row is sticky.
- **Right panel.** Width `--panel-w`. Present on the entry page (relations, mentions, table of contents) and the guide page (vocabulary, section 3.2). Absent on the guides list, Tag editor, Trash, and Settings. Presence depends on the view type, never on the data, so the same view always has the same layout. Below 1000 px window width the panel moves under the content in the same order.
- **Command palette.** Overlay centred horizontally, top edge at 15 vh, 560 px wide, at most 10 rows, opened with Mod+K or the Search button. Section 4.7.
- **Modals.** The existing `Modal.svelte` for New guide, New entry, Rename, Edit guide, Edit aliases, the relation prompts, and the confirm dialog. No backdrop fade.

Each column scrolls independently. Nothing in the shell animates, slides, or fades.

### 2.2 How things are reached

| Thing | Reached from | Route |
|---|---|---|
| Guides list | breadcrumb "Guides", Back, palette "Go to guides" | `#/guides` |
| A guide | guide card or table row, palette "Go to guide: X", breadcrumb | `#/guide/<id>` |
| Filtered entry list | sidebar tag click, sidebar search, `#Tag` chips in the search field | `#/guide/<id>?tag=<id>&q=<text>&stubs=1` (replaceState, section 2.3) |
| An entry | list row, wikilink, relation list link, palette, "Mentioned by" | `#/entry/<id>` |
| New entry | "New entry" button on the guide page, Mod+N, palette "Create entry 'typed text'" | modal, then `#/entry/<id>` |
| Tag editor | sidebar footer "Tag editor", palette | `#/guide/<id>/tags` |
| Trash | sidebar footer "Trash (n)", palette | `#/guide/<id>/trash` |
| Boards (Phase 7) | sidebar footer "Boards", palette | `#/guide/<id>/boards`, `#/board/<id>` |
| Settings | top bar "Settings", Mod+, | `#/settings` |
| Export (Phase 9) | More menu on the guides list (vault), on a guide, on an entry; palette | modal |
| Import (Phase 9) | More menu on the guides list; palette | full content view, steps listed on the left of the content area |

One word per thing in every label: "guide", "entry", "tag", "relation", "qualifier", "verb", "Trash", "Tag editor", "board". The footer link and the page heading use the same words ("Tag editor" and "Tag editor", "Trash" and "Trash").

### 2.3 Breadcrumbs and back

- Breadcrumbs are structural, never historical: `Guides / <guide> / <entry>`, `Guides / <guide> / Tag editor`, `Guides / <guide> / Trash`, `Guides / <guide> / Boards / <board>`. The last crumb is plain text. Crumbs are links; the current one is not.
- A tag filter is not a crumb. The guide page shows the active filter in a fixed summary line in the content (section 3.2).
- Back and Forward use the webview history, which hash routing gives for free. Alt+Left and Alt+Right on Windows and Linux, Cmd+[ and Cmd+] on macOS, mouse buttons 4 and 5, and the two top-bar buttons all do the same thing.
- Filter changes (typing in the search field, clicking a tag) update the route with `history.replaceState`, not `pushState`. Back then returns to the previous page, not to the previous keystroke.
- After Move to Trash, purge, or restore, the app navigates with `replaceState`, so Back never lands on an entry that no longer exists.
- A route whose entry does not exist shows a "not found" view, not an error string (section 5, item 5).

### 2.4 Views that arrive later

- **Board (Phase 7).** A content view. The sidebar's search and tag tree are the board's live filters (same component, same behaviour). Board-only filters (topic entries, hop depth, External, Mentions) sit at the top of the right panel; selecting a node fills the rest of the panel with that entry's relation list through the same `RelationList` component. The board name and a switcher between named boards sit in the content title row where the mode toggle sits on an entry page.
- **Media (Phase 8).** The gallery is a strip under the body in Read mode, heading "Gallery (n)", hidden when empty. Paste and drop land inline at the caret in Edit mode and in the gallery otherwise. "Clean up unused media" is a Settings section with the list, sizes, and total before the confirm.
- **Import and export (Phase 9).** Export is a modal with a scope picker and a destination. Import is a full content view whose steps (File, Target, Mapping, Report) are a fixed vertical list on the left of the content area with the current step marked; no progress animation.

## 3. Per-view layouts

### 3.1 Guides list

```
+----------------------+-----------------------------------------------------------------------+
| [<] [>]              | Guides                                                 [Search] [Settings] |
+----------------------+-----------------------------------------------------------------------+
| [Filter guides     ] | Guides                                        [New guide] [Cards|Table] [More] |
| Tags                 |-----------------------------------------------------------------------|
|   Games           3  | +------------------+ +------------------+ +------------------+        |
|   Courses         1  | | Minecraft        | | Valheim          | | Rust (course)    |        |
|                      | | Survival notes   | | Base building    | | Ownership, traits|        |
|                      | | Games            | | Games            | | Courses          |        |
|                      | +------------------+ +------------------+ +------------------+        |
| Import archive       |                                                                       |
+----------------------+-----------------------------------------------------------------------+
```

- The name filter and the global tag tree move from the toolbar into the sidebar, so the list toolbar holds only New guide, the Cards or Table toggle, and a More menu (Export vault, Import archive, both Phase 9).
- Cards: 220 px minimum, 12 px gap. A card shows the cover only when the guide has one. The grey 80 px placeholder block goes; until Phase 8 a card is name, description, tag chips.
- Table columns: Name, Description, Tags, Updated. Dates everywhere use one format, `YYYY-MM-DD HH:mm` (section 5, item 16).
- Empty state in the content area: "No guides yet." with a "New guide" button under it. Filtered to nothing: "No guides match." with "Clear filter".
- No right panel on this view.

### 3.2 Guide page with the tag tree sidebar

```
+----------------------+-----------------------------------------------+------------------------+
| [<] [>]              | Guides / Minecraft                            | [Search] [Settings]    |
+----------------------+-----------------------------------------------+------------------------+
| [Search this guide ] | Minecraft                    [New entry] [More]| Vocabulary             |
| [ ] Stubs only       | Survival notes                                | Verbs              2   |
| All entries     124  |-----------------------------------------------|   DROPS / DROPPED BY 12|
| v Blocks         12  | Blocks/Natural . "stone"       6 entries [Clear]|   ADJACENT TO       3 |
|     Natural       6  | Title                 Tags           Updated   | Qualifier labels   1   |
|     Crafted       6  | Cobblestone Block stub Blocks/Natural 2026-11-07 |   mined with / mines 4|
| > Items          40  | Stone Block            Blocks/Natural 2026-11-07 |                      |
| > Mobs           30  |                                               | [Rename] [Merge into...] [Delete] |
|                      |                                               |                        |
| Boards               |                                               |                        |
| Tag editor           |                                               |                        |
| Trash             2  |                                               |                        |
+----------------------+-----------------------------------------------+------------------------+
```

Sidebar:

- Search field at the top, placeholder "Search this guide". It is the current filter line (`ui/src/lib/filter.ts`): free text plus `#Tag/Path` chips. Clicking a tag in the tree adds its chip; Ctrl+click adds a second tag. Escape clears the text, then the chips.
- "Stubs only" checkbox under the search field.
- "All entries (n)" row, then the tree. Top-level tags expanded, children collapsed; expansion is remembered for the session. Each row: chevron, name, ancestor-inclusive entry count right-aligned and muted. The selected tag has a 3 px accent bar on its left edge and `--surface` background; no fill animation.
- Footer, always in the same order: Boards (Phase 7), Tag editor, Trash (n).

Content:

- Title row: guide name, spacer, New entry, More (Edit guide, Export guide, Delete guide). "Edit guide" opens a modal with Name, Description, and Guide tags, Save and Cancel. The inline form and its "Save" and "Done" pair go.
- Filter summary line, fixed height, always present: "All entries · 124" when nothing is filtered, otherwise the chips and text in effect, the count, and a "Clear" button.
- Entry list columns: Title (with the "stub" chip), Tags, Updated. Click a row to open; Up and Down move a selection; Enter opens.
- Empty states: "No entries yet. Press Mod+N or choose New entry." and "No entries match this filter." with Clear.

Right panel: **Vocabulary**. Verbs with their inverses and edge counts, then qualifier labels with their inverses and counts. Selecting a row enables a fixed action row at the bottom: Rename, Merge into..., Delete. Delete on a word in use opens the confirm dialog with a "Merge instead" button, which `docs/design.md` section 4 requires and no view offers today.

### 3.3 Entry page, Read mode

```
+----------------------+-----------------------------------------------+------------------------+
| [<] [>]              | Guides / Minecraft / Stone Block              | [Search] [Settings]    |
+----------------------+-----------------------------------------------+------------------------+
| [Search this guide ] | Stone Block  stub   [Read|Edit] [      ] Saved [More] | Add relation    |
| [ ] Stubs only       | Blocks/Natural x   + Add tag                  | [verb, e.g. DROPS    ] |
| All entries     124  | Aliases: Stone, Rock                          |  verb > target > qualifiers |
| v Blocks         12  |-----------------------------------------------|------------------------|
|   * Natural       6  | Grey stone. Drops Cobblestone and sits near   | DROPS                  |
|     Crafted       6  | Oak Log.                                      |   Cobblestone Block    |
| > Items          40  |                                               |     . mined with Pickaxe|
| > Mobs           30  | ## Drops                                      | DROPPED BY             |
|                      | ...                                           |   Stone Block          |
|                      |                                               | Referenced by          |
|                      |                                               |   mined with . Stone Block DROPS ... |
|                      |                                               | [ ] Show external      |
| Boards               |                                               | > Mentioned by (1)     |
| Tag editor           |                                               | > Unresolved links (1) |
| Trash             2  |                                               |------------------------|
|                      |                                               | On this page           |
|                      |                                               |   Drops                |
|                      |                                               |   Where found          |
+----------------------+-----------------------------------------------+------------------------+
```

Where each element sits:

- **Title row** (sticky, 40 px): title, "stub" chip when `is_stub`, spacer, the Read or Edit toggle, an empty slot the width of the Split or In place toggle (filled only in Edit mode, so nothing shifts), the autosave slot (section 4.3), More. The More menu holds Rename..., Edit aliases..., Undo rename (from "Old Title") while available, Copy link, Export entry... (Phase 9), and, last and separated, Move to Trash. The red "Delete" button leaves the title row.
- **Tags line**: chips with a remove control, then an "Add tag" ghost button. The input appears when the button is clicked or Mod+T is pressed, with the shared suggestion list (section 5, item 9). It closes on Enter, Escape, or blur. Tag chips are links to the filtered guide page.
- **Aliases line**: "Aliases: Stone, Rock", muted, present only when the entry has aliases. Edited through More, Edit aliases..., a small modal with the chips and an input.
- **Body**: rendered HTML from `core`, capped at `--measure`. The empty state is "No body yet. Press Mod+E or choose Edit." Unresolved wikilinks keep the dashed style; clicking one creates the entry and navigates, with no confirm (section 4.4). The "Links to entries that do not exist yet" paragraph under the body goes; the same list lives in the right panel.
- **Right panel**, top to bottom, always in this order:
  1. "Add relation" label (12 px, uppercase, muted) and the relation line. Suggestions render in flow under the line (max 8 rows) and push the list down while the line has focus; nothing is overlaid. A stage hint under the line reads `verb > target > qualifiers` with the current stage in `--fg` and the others muted. The duplicate summary sentence goes; staged chips carry "(new)" where a word or entry will be created.
  2. Relation list grouped by the verb slot. Inside a group the leading verb segment is not repeated; a line reads `Cobblestone Block · mined with Pickaxe` under the heading `DROPS`. "Referenced by" lines keep their full text. Selecting a line (click or arrows) enables a fixed action row at the bottom of the list region: "Edit qualifiers", "Delete". Enter on a selected line opens the target.
  3. "Show external" checkbox, off by default.
  4. "Mentioned by (n)", collapsed by default, hidden when n is 0.
  5. "Unresolved links (n)", collapsed by default, hidden when n is 0. Each row has a "Create" button.
  6. "On this page", a fixed footer region, present only when the body has three or more headings, never taller than 40 percent of the panel, scrolls on its own. Built from `h1` to `h4` in the rendered HTML; the UI assigns ids at render time the way `Preview.svelte` rewrites media sources. Click scrolls the content column without smooth scrolling.

The panel region 1 is fixed; regions 2 to 5 scroll together; region 6 is fixed. This is how the right side holds both the relations (requirement 5) and the table of contents (requirement 2) without tabs: the relation line stays at the top as required, and the table of contents has one fixed home at the bottom. The alternative, tabs, is open question 1.

In the sidebar, a tag the entry carries shows a small marker (`*` in the wireframe) so the tree doubles as a "where does this sit" view.

### 3.4 Entry page, Edit mode

Split:

```
+----------------------+-----------------------------------------------+------------------------+
| [Search this guide ] | Stone Block   [Read|Edit] [Split|In place] Unsaved [More] | Add relation |
| ...                  | Blocks/Natural x   + Add tag                  | [verb, e.g. DROPS    ] |
|                      |-----------------------+-----------------------|------------------------|
|                      | 1 Grey stone. Drops   | Grey stone. Drops     | DROPS                  |
|                      |   [[Cobblestone]] and | Cobblestone and sits  |   Cobblestone Block    |
|                      |   sits near [[Oak     | near Oak Log.         | ...                    |
|                      |   Log]].              |                       |                        |
|                      | 2                     | Drops                 |                        |
|                      | 3 ## Drops            |                       |                        |
|                      |                       |                       |------------------------|
|                      |                       |                       | On this page           |
|                      |                     | |                       |   Drops                |
+----------------------+-----------------------+-----------------------+------------------------+
```

In place:

```
+----------------------+-----------------------------------------------+------------------------+
| [Search this guide ] | Stone Block   [Read|Edit] [Split|In place] Saved [More] | Add relation   |
| ...                  | Blocks/Natural x   + Add tag                  | [verb, e.g. DROPS    ] |
|                      |-----------------------------------------------|------------------------|
|                      | Grey stone. Drops Cobblestone and sits near   | DROPS                  |
|                      | Oak Log.                                      |   Cobblestone Block    |
|                      |                                               | ...                    |
|                      | ## Drops|                                     |                        |
|                      |                                               |                        |
|                      | Stone drops [[Cobblestone Block]] when mined. |                        |
+----------------------+-----------------------------------------------+------------------------+
```

- The content column keeps its width. `.page.wide` in `EntryView.svelte` goes; today the title moves 31 px left when Edit is chosen (compare `4-read.png` and `3-edit.png`).
- The Split or In place toggle appears in its reserved slot. Mod+Shift+E switches. The choice is remembered in `config.toml`.
- **Split**: editor left, preview right, a 1 px divider that can be dragged, 50/50 by default, each pane at least 320 px. The preview is the same `Preview.svelte`. No scroll sync in the first version; the divider position is remembered.
- **In place**: one CodeMirror pane. Version 1 is a markdown theme plus decorations: headings in heading sizes, emphasis styled, links in `--accent`, code in monospace, markup characters kept visible. Version 2 hides markup on lines that do not hold the caret and shows images under their line. Version 1 ships with the toggle; version 2 is later (section 6).
- Line numbers are off by default and are a setting under Editor. `highlightActiveLine` stays. `drawSelection({ cursorBlinkRate: 0 })` is offered as the Editor setting "Caret blinks", on by default.
- The right panel stays in Edit mode. At a 1200 px window with roomy density that leaves about 620 px for the content column, so Split panes are about 310 px each. That is narrow; In place is the better fit at that width, Split suits a maximized window. Whether the panel should collapse in Edit mode is open question 2.
- Leaving Edit mode, leaving the page, and closing the window all flush the draft as today.

### 3.5 Tag editor

```
+----------------------+-----------------------------------------------+------------------------+
| [<] [>]              | Guides / Minecraft / Tag editor               | [Search] [Settings]    |
+----------------------+-----------------------------------------------+------------------------+
| [Search this guide ] | Tag editor                          [New tag] |                        |
| ...                  |-------------------------------+---------------|                        |
|                      | v Blocks                   12 | Natural       |                        |
|                      |     Natural                 6 | Name [Natural    ] |                   |
|                      |     Crafted                 6 | Parent [Blocks  v] |                   |
|                      | > Items                    40 | Colour [none    v] |                   |
|                      | > Mobs                     30 | Entries: 6         |                   |
|                      |                               | [Add child tag]    |                   |
|                      |                               | [Merge into...]    |                   |
|                      |                               | [Delete tag]       |                   |
|                      |-------------------------------|                    |                   |
|                      | Top level: drop a tag here    |                    |                   |
+----------------------+-------------------------------+--------------------+-------------------+
```

- The content area has two parts: the tree (left, 60 percent) and a detail pane for the selected tag (right, 40 percent, fixed). The detail pane replaces the hover-only Rename, Add child, and Delete links and the inline forms that pushed rows around.
- Keyboard: Up and Down move the selection, Left and Right collapse and expand, F2 focuses the Name field, Delete opens the confirm dialog, Enter in Name saves. Drag to nest stays for the mouse and moves to pointer events (Phase 8 needs that anyway). The "Top level" drop bar stays at the bottom of the tree, fixed.
- Parent is also editable from the dropdown, which is the keyboard way to nest.
- "Merge into..." opens a modal with the shared suggestion list over tag paths.
- Empty state: "No tags yet. Add a tag to an entry, or choose New tag."
- The sidebar's tree and the content tree show the same data. The sidebar remains navigation; the content tree is the editor. No right panel.

### 3.6 Trash

```
+----------------------+-----------------------------------------------------------------------+
| [<] [>]              | Guides / Minecraft / Trash                             [Search] [Settings] |
+----------------------+-----------------------------------------------------------------------+
| [Search this guide ] | Trash                                                   [Empty trash] |
| ...                  | 2 entries. Restore puts an entry back with its tags and relations.    |
|                      | Purge removes it for good.                                            |
|                      |-----------------------------------------------------------------------|
|                      | Title                  Deleted            Tags                        |
|                      | Oak Log 194335         2026-11-07 19:44   Blocks/Natural              |
|                      | Stone Block 194335     2026-11-07 19:45                               |
|                      |                                                                       |
|                      | Selected: Oak Log 194335              [Restore] [Purge...]            |
+----------------------+-----------------------------------------------------------------------+
```

- Selection plus a fixed action row, same pattern as the relation list and the Tag editor. Restore does not confirm (it is reversible by trashing again). Purge and Empty trash confirm (section 4.4).
- Empty state: "Trash is empty." and the Empty trash button disabled.
- Trash is per guide, as `docs/design.md` section 3 says; the sidebar footer count is that guide's.

### 3.7 Settings

```
+----------------------+-----------------------------------------------------------------------+
| [<] [>]              | Settings                                               [Search] [Settings] |
+----------------------+-----------------------------------------------------------------------+
| Data                 | Data                                                                  |
| Backups              |   Mode            Default                                             |
| Appearance           |   Data directory  C:\Users\...\FieldGuide                             |
| Editor               |   Vault           ...\vault                                           |
| Media (Phase 8)      |   Version         0.1.0                                               |
| Updates              | Backups                                                [Back up now]  |
|                      |   Taken             Size    File                                      |
|                      |   2026-11-07 19:31  212 KB  vault-2026-11-07_193100-123.db  [Restore] |
|                      | Appearance                                                            |
|                      |   Density  ( ) Compact  (o) Roomy                                     |
|                      |   Theme    Follows the system setting. A manual choice comes later.   |
|                      | Editor                                                                |
|                      |   [ ] Line numbers   [x] Caret blinks   Edit opens in (o) Split ( ) In place |
|                      | Updates                                                               |
|                      |   [Check for updates]                                                 |
+----------------------+-----------------------------------------------------------------------+
```

- One scrolling page. The sidebar lists the sections and scrolls to them; the current section is marked.
- New sections: Appearance (density, theme line), Editor (line numbers, caret, default Edit variant). Media arrives with Phase 8.
- The Mode row keeps its explanation but as a second muted line, not in brackets inside the value.

### 3.8 Density presets

Two presets, stored as `density` in `UiPrefs` (`core/src/config.rs`) and applied as `data-density` on `<html>` as soon as prefs load at boot. Default is roomy. Everything in `app.css` and the components reads these properties; no component hard-codes a pixel size.

```
:root[data-density="compact"] {
  --font-size: 14px;      /* base; the body and lists */
  --line-height: 1.4;
  --h1: 1.35rem;
  --small: 0.86rem;       /* chips, counts, hints */
  --space-1: 2px;
  --space-2: 4px;
  --space-3: 8px;
  --space-4: 12px;
  --space-5: 16px;
  --row-h: 26px;          /* list rows, tree rows, suggestion rows */
  --control-h: 26px;      /* buttons, inputs */
  --measure: 76ch;        /* body line length */
  --sidebar-w: 220px;
  --panel-w: 280px;
  --radius: 4px;
}

:root[data-density="roomy"] {
  --font-size: 16px;
  --line-height: 1.6;
  --h1: 1.6rem;
  --small: 0.875rem;
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-5: 24px;
  --row-h: 34px;
  --control-h: 32px;
  --measure: 66ch;
  --sidebar-w: 260px;
  --panel-w: 320px;
  --radius: 4px;
}
```

The sidebar and panel widths set by the user override `--sidebar-w` and `--panel-w` through an inline style, so a resize is kept across preset changes.

### 3.9 Theme and motion

- Replace the `Canvas` and `CanvasText` `color-mix` tokens with explicit values per scheme. System colours plus `color-mix` give a muted text colour near 4:1 contrast in dark mode, under what body text needs.

```
:root {
  --bg: #ffffff;  --fg: #1b1b1b;  --muted: #5c5f63;  --border: #d8d8d8;
  --surface: #f4f4f4;  --accent: #0b57d0;  --danger: #b3261e;
  --warning-bg: #fff4d6;  --chip-bg: #e9e9e9;
}
@media (prefers-color-scheme: dark) {
  :root:not([data-theme="light"]) {
    --bg: #1e1e1e;  --fg: #e6e6e6;  --muted: #a3a3a3;  --border: #3a3a3a;
    --surface: #262626;  --accent: #8ab4f8;  --danger: #f28b82;
    --warning-bg: #4a3b12;  --chip-bg: #333333;
  }
}
:root[data-theme="dark"] { /* same values as the dark block; the manual override hook */ }
```

- Add once, globally: `*, *::before, *::after { animation: none !important; transition: none !important; }` and `html { scroll-behavior: auto; }`. Nothing in the app may animate, including the modal backdrop and the palette.
- Focus is always visible: `:focus-visible { outline: 2px solid var(--accent); outline-offset: 1px; }`. CodeMirror keeps its border change on focus.
- Relation chips get their own style, `chip relation`: transparent background, 1 px `--border`, verb uppercase. Tag chips keep the filled style. Today both use `.chip`, so a staged verb looks like a tag (`r1-staged.png`).

## 4. Interaction details

### 4.1 Keyboard map

Mod is Ctrl on Windows and Linux, Cmd on macOS. One key, one action, everywhere in the app.

| Keys | Action | Where |
|---|---|---|
| Mod+K | Open the command palette | everywhere |
| Mod+N | New entry (New guide on the guides list) | everywhere |
| Mod+Shift+N | New guide | everywhere |
| Mod+F | Focus the sidebar search field | everywhere inside a guide |
| Mod+S | Save now | Edit mode |
| Mod+E | Toggle Read and Edit | entry page |
| Mod+Shift+E | Toggle Split and In place | Edit mode |
| Mod+R | Focus the relation line | entry page |
| Mod+T | Open the Add tag input | entry page |
| Mod+, | Settings | everywhere |
| Alt+Left, Alt+Right (Cmd+[, Cmd+] on macOS) | Back, Forward | everywhere |
| F2 | Rename the selected thing (entry title, tag, board) | lists and the entry page |
| Delete | Delete or trash the selected row (confirm where section 4.4 says so) | lists |
| Up, Down | Move the selection | lists, suggestions, palette |
| Left, Right | Collapse, expand | tag tree |
| Enter | Open the selected row; pick the highlighted suggestion; save a complete relation line; confirm a dialog | lists, relation line, dialogs |
| Backspace on an empty relation line | Remove the last staged item | relation line |
| Tab, Shift+Tab | Move focus in the order of section 4.2 | everywhere |
| Escape | Section 4.1.1 | everywhere |

Two shell requirements make this map work:

- Turn off the webview's own accelerator keys on Windows (wry's `with_browser_accelerator_keys(false)`, which Tauri 2 exposes on the window builder for Windows). Today F5 and Ctrl+R reload the UI, Ctrl+P opens a print dialog, and Ctrl+F opens the webview's find bar. If the builder flag is not reachable, a window-level `keydown` handler that calls `preventDefault()` on F5, Mod+R, Mod+P, and Mod+F has the same effect.
- Register the shortcuts in one place (`ui/src/lib/keys.ts`) and show them in the palette rows and in button tooltips, so the mouse path teaches the keyboard path.

Shortcuts outside this table are not added. Everything else is a palette command.

#### 4.1.1 Escape

Escape closes or clears the innermost thing. It never navigates and never discards saved data.

1. A modal or the palette is open: close it. For the relation prompts that means the staged line is unchanged.
2. The relation line has text: clear the text. The relation line is empty but has staged chips: clear the line. The line is empty with no chips: move focus to the body.
3. The sidebar search has text or chips: clear the text, then the chips, then blur.
4. The Add tag input is open: close it without adding.
5. A list has a selection: clear the selection.
6. Focus is in the editor: nothing. Leaving Edit mode is Mod+E only.

### 4.2 Focus order on the entry page

DOM order equals visual order, left to right, top to bottom. Composite widgets are one tab stop with arrow keys inside.

1. Back, Forward.
2. Breadcrumb links.
3. Search (palette) button, Settings.
4. Sidebar search field, Stubs only, the tree (one stop, arrows move, Enter filters), Boards, Tag editor, Trash.
5. Title row: Read or Edit toggle (one stop, arrows), Split or In place toggle (Edit mode only), Retry (only when a save failed), More.
6. Tags line: each chip's remove control, Add tag.
7. Body: links in document order. In Edit mode the editor is one stop; Tab inside it indents, Escape then Tab leaves it.
8. Right panel: relation line, Show external, relation list (one stop, arrows select, Enter opens), Mentioned by toggle, Unresolved links toggle, table of contents links.

When a navigation lands on an entry, focus goes to the content region (`tabindex="-1"` on the content column) so Page Down scrolls the body and Tab reaches the title row next. Mod+R, Mod+T, Mod+F, and Mod+E jump past the sidebar, which is why they exist.

### 4.3 Autosave indicator

- Place: the title row, in a slot right of the mode toggles, `min-width: 8em`, left-aligned. The slot exists in both modes and is empty in Read mode, so the More button never moves.
- States, with exact wording: "Unsaved" (muted) while the draft differs from the stored body, including the moment the write is in flight; "Saved" (muted) after the write lands; "Save failed" (in `--danger`) with a "Retry" button inside the slot. There is no "Saving" state: the write is local and takes milliseconds, and a label that flickers three times per pause is motion.
- Leaving the page with an unsaved draft keeps the current behaviour: the body is written on the way out, and a failure becomes a notice naming the entry.

### 4.4 Confirmations

Confirm only what cannot be undone. Every confirm is the in-app `ConfirmDialog` (a variant of `Modal.svelte`), never the webview's `confirm()`, which shows an origin line in WebView2 and ignores the app's density and theme.

| Action | Confirms | Dialog title and body | Buttons |
|---|---|---|---|
| Move to Trash | No. A notice "Moved 'X' to Trash. [Undo]" appears instead. | | |
| Restore from Trash | No. | | |
| Create an entry from an unresolved link | No. A notice "Created 'X'. [Undo]" appears; Undo trashes it. | | |
| Rename | No. Undo rename is in the notice and in More for the session. | | |
| Purge | Yes | "Purge 'X'?" / "The entry, its relations, and its gallery rows are removed. Links to it in other entries stay as text. This cannot be undone." | Cancel, Purge |
| Empty trash | Yes | "Purge 3 entries?" / same body in the plural | Cancel, Empty trash |
| Delete relation | Yes | "Delete this relation?" / the display line / "Relations have no trash." | Cancel, Delete |
| Delete tag | Yes | "Delete the tag 'Blocks/Natural'?" / "6 entries lose it. 2 child tags move up one level." | Cancel, Delete tag |
| Delete verb or qualifier label | Yes | "Delete 'DROPS'?" or, when in use, "'DROPS' is used by 12 relations." | Cancel, Delete; or Cancel, Merge instead |
| Delete guide | Yes | "Delete the guide 'X'?" / "Its 124 entries, tags, relations, and boards are removed. Guides have no trash. Export it first if you may want it back." | Cancel, Delete guide |
| Restore backup | Yes | "Replace the vault with the backup from 2026-11-07 19:31?" / "The current vault is backed up first." | Cancel, Restore |
| Clean up media (Phase 8) | Yes | the file list with sizes and the total | Cancel, Delete 14 files |

Dialog rules: the title is a question naming the thing; the body says what happens; the confirming button carries the verb, never "OK" or "Yes"; focus starts on the confirming button so Enter completes the action the user just asked for; Escape cancels. The confirming button uses `--danger`.

### 4.5 Empty states

Each empty state is one sentence, names the next action, and includes the shortcut when there is one.

| View | Wording |
|---|---|
| Guides list, no guides | "No guides yet." with a "New guide" button under it |
| Guides list, filtered | "No guides match." with "Clear filter" |
| Guide page, no entries | "No entries yet. Press Mod+N or choose New entry." |
| Guide page, filtered | "No entries match this filter." with "Clear" |
| Entry body | "No body yet. Press Mod+E or choose Edit." |
| Relation list | "No relations yet. Type a verb above, such as DROPS." |
| Vocabulary panel | "No verbs yet. The first one is created from an entry's relation line." |
| Tag tree (sidebar) | "No tags yet." |
| Tag editor | "No tags yet. Add a tag to an entry, or choose New tag." |
| Trash | "Trash is empty." |
| Backups | "No backups yet." |
| Mentioned by, Unresolved links, On this page | hidden, no text |
| Palette, no match | "No entries or commands match." plus the "Create entry 'typed text'" row |

### 4.6 Notices

- One slot under the top bar. One notice visible at a time; when more are queued the slot shows "1 of 2" and a "Next" button. Nothing auto-dismisses and nothing slides in.
- Kinds: **warning** (`--warning-bg`: sync folder), **error** (`--danger` text on `--surface`: backup failed, save failed), **info** (`--surface`: moved to Trash, created, renamed, restored). Each has at most one action button and a Dismiss button. Info notices with an Undo action are replaced by the next info notice and disappear when their Undo is no longer possible; warnings and errors stay until dismissed.
- The palette has "Dismiss notice". Escape does not dismiss a notice, because Escape is for the focused thing.
- Rewritten copy. Sync: "Your data is in a OneDrive folder. A sync client can corrupt the database while Field Guide is open. Move the data folder somewhere else." with "Open Settings". Backup failed: "Today's backup failed: <reason>." with "Back up now". Save failed on leaving: "The body of 'X' could not be saved: <reason>." with "Open entry".

### 4.7 Command palette

- Mod+K or the Search button. One input. Rows are grouped under fixed headings in a fixed order: Entries (this guide; title and alias matches first, body matches after, each row showing title, "stub" chip, and the tag path), Commands (every action in this document, with its shortcut on the right), Guides (switch guide). Up and Down move, Enter runs, Escape closes.
- The last row is always "Create entry 'typed text'" when the text does not match a title exactly. That is the "new entry" path required by the interview, and it needs no modal.
- The `>` prefix limits the list to commands, as in VS Code. It is an accelerator, not the only way; the Commands group is always present.
- Outside a guide (guides list, Settings) the Entries group is replaced by Guides at the top.

## 5. Critique of the current build

Ordered by impact. File names are under `ui/src/` unless stated.

1. **No sidebar, no persistent place for search and tags.** `GuideView.svelte` holds the filter line and `TagsView.svelte` is a separate page reached by a bare "Tags" link beside two buttons. Browse by tag, the primary way into a guide per `docs/design.md` section 5, is not visible anywhere on an entry page. Change: the shell in section 2.1; this is Phase 6 work and every other layout change hangs on it.
2. **The content jumps between Read and Edit.** `EntryView.svelte` toggles `.page.wide` (`max-width: none`), so the title and every control moves 31 px left when Edit is chosen (`4-read.png` against `3-edit.png`). Change: remove `.wide`; the split lives inside a fixed-width column.
3. **Hover-only actions.** `RelationList.svelte` and `TagsView.svelte` hide their action buttons with `visibility: hidden` until hover or `focus-within`. Controls that appear and disappear are motion and are undiscoverable; a keyboard user finds them only by tabbing into the row. Change: selection plus a fixed action row (sections 3.3, 3.5, 3.6).
4. **Native confirms, and confirms on reversible actions.** `confirm()` is used in `EntryView.svelte` (trash, create from link), `TagsView.svelte` (delete), `RelationList.svelte` (delete), and `SettingsView.svelte` (restore). The webview dialog carries an origin line and ignores the theme. Trashing an entry and creating an entry from a link are reversible and still confirm. Change: section 4.4; update `docs/checklist.md` ("Delete moves an entry to the trash after confirmation") to match.
5. **Internals leak into copy.** `core/src/error.rs` `NameTaken` renders `"Stone" is already used by entry 018f6c2e-... in this guide`; the UI shows it verbatim under the alias input and in the New entry modal. `NotFound` renders `entry 018f... not found` when a stale route is opened after a purge or a restore. The fatal screen (`App.svelte`) prints core's lowercase strings as sentences: "this vault was created by a pre-release build...", "vault is open in another Field Guide: C:\...\vault.db" (`g1-old-vault.png`, `s1-locked.png`). Change: `NameTaken` carries the other entry's title and whether the clash is a title or an alias, so the message reads `"Stone" is already an alias of "Stone Block"` and the UI links the title; `EntryView.svelte` renders `kind === "not_found"` as "This entry no longer exists." with a link to the guide; the fatal screen owns its sentences keyed on `kind`, sentence-cased, with the path in its own `<code>` block and the action on its own line.
6. **"Delete" means Move to Trash.** The red "Delete" button in `EntryView.svelte` sits beside the Read or Edit toggle, at equal weight, 10 px from "Edit". The word is wrong (the entry goes to Trash) and the position invites a misclick. Change: "Move to Trash" as the last item of the More menu; Undo through a notice.
7. **The relation line is a bare input between Tags and the body.** `RelationBuilder.svelte` has no label; the placeholder "Relation: verb, e.g. DROPS" is the only cue, and it disappears when typing starts. Suggestions are `position: absolute` and cover the "Relations" heading (`r2-stone.png`, `r5-merged.png`). The staged chips use the tag chip style, so a verb looks like a tag. The summary sentence under the line repeats the chips. Change: section 3.3 region 1: label, stage hint, in-flow suggestions, relation chip style, drop the summary, Mod+R.
8. **The relation list repeats the verb.** Under the heading "DROPS" every line begins "DROPS ..." (`r2-stone.png`). `core` builds the segments; `RelationList.svelte` can drop the first segment when its text equals the group label. "Qualifiers" as the edit button's label names a noun, not an action: "Edit qualifiers".
9. **Two autocomplete styles.** `TagPicker.svelte` uses a native `<datalist>`, which WebView2 draws as a browser dropdown; `RelationBuilder.svelte` draws its own list. Change: one `Suggestions.svelte` used by the relation line, Add tag, Edit aliases, Merge into, and the palette.
10. **Always-open inputs for Aliases and Tags.** Every entry page shows two empty text fields with placeholders (`2-oak-new.png`). Change: tag chips plus an "Add tag" button that opens the input; the aliases line only when there are aliases, edited from More.
11. **Edit guide is an inline form with "Save" and "Done".** `GuideView.svelte` pushes the whole list down while editing. "Done" discards the name and description drafts while tag changes have already been saved, so the two buttons do different things to different fields. Change: a modal with Save and Cancel; tag changes apply on Save too.
12. **Breadcrumbs are inconsistent.** The entry page and the Tag editor show "Guides / Minecraft"; the guide page shows none; Settings shows none. Change: crumbs in the top bar on every view (section 2.3).
13. **Top bar carries little and the back control is a text link.** `App.svelte` shows an arrow styled as a link, the brand, and an underlined "Settings". Change: Back and Forward as buttons, crumbs, Search, Settings (section 2.1).
14. **Guide cards show an empty grey block.** `GuidesView.svelte` draws an 80 px `.cover` even with no cover. Change: no block until there is a cover.
15. **Timestamps to the second.** `format.ts` `whenTime` renders "11/7/2026, 7:42:21 PM" in the entry list. Change: `YYYY-MM-DD HH:mm` from one function used everywhere.
16. **The "Undo rename from X" link sits in the title row.** `EntryView.svelte` adds a long link beside the title for the rest of the session. Change: a notice right after the rename, and "Undo rename (from X)" in More.
17. **The unresolved-links paragraph repeats the body's cue.** `EntryView.svelte` prints "Links to entries that do not exist yet: Cobblestone. Click one to create it." under the body on every render. Change: the collapsed "Unresolved links (n)" region in the right panel.
18. **No way to rename, merge, or delete verbs and qualifier labels.** `docs/design.md` section 4 says the UI offers merge when a word in use is deleted; `api.ts` has `merge_verbs` and `merge_qualifier_labels` and no view calls them. Change: the Vocabulary panel (section 3.2).
19. **The webview's accelerator keys are on.** F5 and Ctrl+R reload the UI mid-edit (the hash survives, the 800 ms autosave usually does), Ctrl+P prints, Ctrl+F opens the find bar. Change: section 4.1, shell side.
20. **Theme tokens are mixed from system colours.** `app.css` uses `color-mix` over `Canvas`; muted text in dark mode lands near 4:1. Change: section 3.9.
21. **Prompt buttons say "Use".** "Use verb", "Use label", "Create and use" in `RelationBuilder.svelte`. Change: "Add verb", "Add label", "Create entry".
22. **Modal focus workaround.** `Modal.svelte` blurs and refocuses after `showModal()` to work around a WebView2 focus bug; `RelationBuilder.svelte` repeats the trick in `refocus()`. Keep it, but move it into `Modal.svelte` alone and expose an `onopened` callback so components stop timing their own focus.

## 6. Ranked change list

| # | Change | Reason | Effort | Phase |
|---|---|---|---|---|
| 1 | Three-column shell with one aligned top bar; `Sidebar.svelte`, a right panel slot in `App.svelte`, independent scrolling | Every fixed requirement about placement depends on it | M | Phase 6 |
| 2 | Remove `.page.wide`; fixed content width in both modes | Content jumps on mode toggle | S | now |
| 3 | "Move to Trash" in the More menu, no confirm, Undo notice; update `docs/checklist.md` | Wrong word, misclick risk, confirm on a reversible action | S | now |
| 4 | `ConfirmDialog` variant of `Modal.svelte`; replace all five `confirm()` calls; wording from section 4.4 | Native dialogs break theme and density; confirms only for destructive actions | S | now |
| 5 | `NameTaken` carries title and clash type; "not found" view; fatal screen owns its sentences | Raw ids and lowercase strings reach the user | S | now |
| 6 | Relation line: label, stage hint, in-flow suggestions, relation chip style, drop the summary | Discoverability; suggestions cover the list | M | now |
| 7 | Relation list: drop the repeated verb; selection plus fixed action row; "Edit qualifiers" | Repetition; hover-only actions | M | now |
| 8 | Turn off webview accelerator keys; `keys.ts` with the map from section 4.1 | F5 reloads the app; keyboard first | S now, M for the map | now / Phase 6 |
| 9 | Autosave slot with three states and Retry | Label flicker; moving controls | S | now |
| 10 | Notice slot: one at a time, kinds, actions, rewritten copy | Stacking banners; long sync text | S | now |
| 11 | Explicit theme tokens; global no-animation rule; `:focus-visible` | Contrast in dark mode; no motion | S | now |
| 12 | `Suggestions.svelte` shared by tags, aliases, relation line, merge, palette; drop `<datalist>` | Two autocomplete styles | M | now |
| 13 | Tags line with Add tag button; aliases line only when present; Edit aliases modal; Mod+T | Two empty inputs on every entry | S | now |
| 14 | Create entry from an unresolved link without confirm; Undo notice; `replaceState` after trash, purge, restore | Confirm on a reversible action; Back lands on a gone entry | S | now |
| 15 | One date format, `YYYY-MM-DD HH:mm` | Seconds are noise; one format everywhere | S | now |
| 16 | Prompt button labels "Add verb", "Add label", "Create entry" | "Use" is vague | S | now |
| 17 | Right panel on the entry page: regions 1 to 6 of section 3.3, Mentioned by and Unresolved links collapsed, table of contents footer | Requirements 2 and 5 | M | Phase 6 |
| 18 | Command palette: entries, commands, guides, create row, `>` prefix | Requirement 7 | M | Phase 6 |
| 19 | Density presets as custom properties; Appearance section in Settings | Requirement 3 | S | Phase 6 |
| 20 | Guide page: Edit guide modal, filter summary line, Tags column, selection in the list | Inline form moves the list; filter state invisible | M | Phase 6 |
| 21 | Guides list: filters move to the sidebar; no empty cover block; empty-state button | Toolbar clutter; grey block | S | Phase 6 |
| 22 | Tag editor: tree plus detail pane, keyboard map, pointer-event drag | Hover-only actions; inline forms; Phase 8 needs pointer events | M | Phase 6 |
| 23 | Trash view as in section 3.6 | Planned for Phase 6; layout given | M | Phase 6 |
| 24 | Settings: section nav in the sidebar; Editor section | Shell consistency | S | Phase 6 |
| 25 | Edit mode toggle Split or In place; In place version 1 (styled editor) | Requirement 6 | L | Phase 6 |
| 26 | In place version 2 (hide markup off the caret line, inline images) | Requirement 6, full form | L | later |
| 27 | Vocabulary panel on the guide page: rename, merge, delete verbs and labels | Design section 4 requires merge; no UI exists | M | later |
| 28 | Board: sidebar as filters, board toggles and selected node's relations in the right panel, board switcher in the title row | Keeps the shell's placement rules in Phase 7 | M | Phase 7 |
| 29 | Modal `onopened` callback; remove the `refocus()` copy | Two copies of a focus workaround | S | later |

## 7. Open questions for the owner

1. **How the right panel holds the table of contents.** Options: (a) fixed footer under the relations, present only with three or more headings, never over 40 percent of the panel; (b) two tabs, "Relations" and "Contents", with Relations selected on arrival. Recommended: (a). It keeps the relation line at the top and gives the table of contents one fixed home without a click. Build two mockups of the Stone Block page with a six-heading body, one per option, and compare.
2. **The right panel in Edit mode with Split.** Options: (a) the panel stays, and the two editing panes share what is left (about 310 px each in a 1200 px window at roomy density); (b) the panel collapses whenever Edit mode is entered and returns in Read mode. Recommended: (a), with a manual collapse toggle whose state is remembered separately for Edit mode, so the layout only changes when the owner changes it. Build two mockups at 1200 px wide.
3. **Row actions.** Options: (a) selection plus one fixed action row per list, as proposed; (b) always-visible muted text buttons on every row. Recommended: (a). (b) puts "Edit qualifiers  Delete" on every line of a 280 px panel. The answer sets the width of every list.
4. **Whether the table of contents earns its space.** This is the one place this review disagrees with a fixed requirement. Entries in a game guide are mostly short, and a third navigation aid is chrome under requirement 8. Options: (a) show it only when the body has three or more headings, as designed above; (b) always show it, with "No headings" when empty. Recommended: (a).
5. **Where aliases live.** Options: (a) a muted line under the tags, present only when there are aliases, edited through More; (b) a "Details" region at the bottom of the right panel with aliases, created, and updated. Recommended: (a). (b) makes the panel taller and competes with the table of contents footer. The answer sets the title block's height.

## 8. Comparable products

**Obsidian** uses a left sidebar with tabs (files, search, bookmarks, tags) and a right sidebar with tabs (backlinks, outgoing links, outline, tags). Its outline is built from headings, exactly what requirement 2 asks for, and Ctrl+E toggles editing and reading, which this review adopts. Its live preview, which hides markup off the caret line, is the model for the In place variant. Two things to avoid: it has two palettes (Ctrl+P for commands, Ctrl+O for files), and this design merges them under Mod+K; and most of its controls appear on hover, with animations that need a setting to turn off. Its sidebars also depend on tabs, which is why open question 1 offers tabs only as the alternative.

**Notion** puts breadcrumbs in the top bar, properties (tags, dates) directly under the title, and backlinks as a collapsed "n backlinks" line at the top of the page. The breadcrumbs-in-the-top-bar placement and the collapsed count for mentions are borrowed here. Its search is Ctrl+K, the shortcut chosen for the palette. What to avoid: drag handles and a "+" that appear on hover, transitions on nearly every state change, and block-level editing that turns a page into a tree of widgets. Field Guide's body is one markdown document, and the relation line replaces the database property grid as the structured part of a page.

**A classic wiki** (MediaWiki, DokuWiki) shows Read and Edit as tabs in a fixed spot above the article, which the mode toggle in the title row copies. MediaWiki inserts a table of contents only when the article has four or more headings; the three-heading threshold in section 3.3 follows that rule. Red links for missing pages are the ancestor of the dashed unresolved wikilink, including click to create. The infobox at the right of an article is the nearest relative of the relation panel: structured facts beside prose. What to avoid: "What links here" as a separate page (mentions belong beside the body), categories listed at the bottom (tags belong under the title where they are read first), and a breadcrumb that records the reader's path rather than the page's place, which DokuWiki's "trace" does.

// Shell choices that last while the app runs and are not saved: which sidebar regions are open
// and whether the tree's Boards node is expanded.

export const shell = $state({
  /** The sidebar's Tags region (the tree) is open. */
  tagsOpen: true,
  /** The sidebar's Entries region (the list) is open. */
  entriesOpen: true,
  /** The tree's Boards node shows the guide's boards. */
  boardsOpen: false,
});

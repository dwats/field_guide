// Process-wide UI state: what the shell reported at startup, saved preferences, and the renames
// that can still be undone this session (design section 3, "Rename is undoable ... for the rest
// of the session").

import type { RenameRecord, Startup, UiPrefs } from "./api";

export const session = $state({
  startup: null as Startup | null,
  prefs: { guide_view: "cards" } as UiPrefs,
  /** Most recent rename per entry id. */
  renames: {} as Record<string, RenameRecord>,
});

// The theme preference as attributes on <html> (app.css reads them). Pure, so it is testable
// without a window.

import type { ThemePref } from "./api";

/** The value of `data-theme` on <html>, or null to remove it so the OS decides. */
export function themeAttribute(pref: ThemePref): "light" | "dark" | null {
  return pref === "system" ? null : pref;
}

// The keyboard map (UX review 4.1, design section 5 "Input"): one place, read by the key handler,
// the palette rows, and button labels. Mod is Ctrl on Windows and Linux, Cmd on macOS.

export type Action =
  | "palette"
  | "new"
  | "new-guide"
  | "focus-search"
  | "toggle-mode"
  | "focus-relation"
  | "add-tag"
  | "rename"
  | "settings"
  | "back"
  | "forward"
  | "toggle-sidebar"
  | "toggle-panel"
  | "tag-editor"
  | "trash"
  | "dismiss-notice"
  | "density-compact"
  | "density-roomy";

export interface Shortcut {
  action: Action;
  /** `Mod`, `Shift`, `Alt` in that order, then the key as `KeyboardEvent.key`, letters uppercase. */
  keys: string;
}

export const SHORTCUTS: Shortcut[] = [
  { action: "palette", keys: "Mod+K" },
  { action: "new", keys: "Mod+N" },
  { action: "new-guide", keys: "Mod+Shift+N" },
  { action: "focus-search", keys: "Mod+F" },
  { action: "toggle-mode", keys: "Mod+E" },
  { action: "focus-relation", keys: "Mod+R" },
  { action: "add-tag", keys: "Mod+T" },
  { action: "settings", keys: "Mod+," },
  { action: "back", keys: "Alt+ArrowLeft" },
  { action: "forward", keys: "Alt+ArrowRight" },
  { action: "rename", keys: "F2" },
];

export const IS_MAC =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad/.test(navigator.platform);

export interface KeyLike {
  key: string;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
  altKey: boolean;
}

/** The chord an event spells in the map's notation, or null for a modifier on its own. */
export function chord(event: KeyLike, mac = IS_MAC): string | null {
  if (["Control", "Meta", "Shift", "Alt"].includes(event.key)) return null;
  const key = event.key.length === 1 ? event.key.toUpperCase() : event.key;
  const parts: string[] = [];
  if (mac ? event.metaKey : event.ctrlKey) parts.push("Mod");
  if (event.shiftKey) parts.push("Shift");
  if (event.altKey) parts.push("Alt");
  parts.push(key);
  return parts.join("+");
}

export function actionFor(event: KeyLike, mac = IS_MAC): Action | null {
  const c = chord(event, mac);
  return SHORTCUTS.find((s) => s.keys === c)?.action ?? null;
}

export function shortcutFor(action: Action): string | null {
  return SHORTCUTS.find((s) => s.action === action)?.keys ?? null;
}

/** The action's chord as shown to the user, or null when it has none. */
export function keysFor(action: Action, mac = IS_MAC): string | null {
  const keys = shortcutFor(action);
  return keys ? displayKeys(keys, mac) : null;
}

/** `Mod+K` shown as `Ctrl+K`, or `⌘K` on a Mac. */
export function displayKeys(keys: string, mac = IS_MAC): string {
  const names: Record<string, string> = { ArrowLeft: "←", ArrowRight: "→" };
  const parts = keys.split("+").map((part) => {
    if (part === "Mod") return mac ? "⌘" : "Ctrl";
    if (part === "Shift") return mac ? "⇧" : "Shift";
    if (part === "Alt") return mac ? "⌥" : "Alt";
    return names[part] ?? part;
  });
  return mac ? parts.join("") : parts.join("+");
}

/** Whether a key event comes from a place where typing is expected, so plain keys stay there. */
export function isEditable(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  return (
    target instanceof HTMLInputElement ||
    target instanceof HTMLTextAreaElement ||
    target instanceof HTMLSelectElement ||
    target.isContentEditable
  );
}

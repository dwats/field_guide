// Sentences for the start-up failures the shell can report, keyed on the error kind. The word
// "vault" stays out of these; it belongs to Settings only.

export interface FatalText {
  what: string;
  next: string;
}

export function fatalText(kind: string, message: string): FatalText {
  switch (kind) {
    case "vault_locked":
      return {
        what: "Your data is open in another Field Guide window.",
        next: "Close the other window, then start this one again.",
      };
    case "newer_schema":
      return {
        what: "This data was written by a newer Field Guide.",
        next: "Update Field Guide to open it.",
      };
    case "portable_dir_not_writable":
      return {
        what: "Portable mode is on, and the folder beside the executable cannot be written.",
        next: "Move Field Guide to a writable folder, or remove the file named portable.",
      };
    case "data_dir_not_writable":
      return {
        what: "The data folder cannot be written.",
        next: "Check the folder's permissions, then start Field Guide again.",
      };
    case "config_parse":
      return {
        what: "The settings file could not be read.",
        next: "Fix or delete config.toml, then start Field Guide again.",
      };
    default: {
      const sentence = message.trim();
      if (!sentence) return { what: "Field Guide could not start.", next: "" };
      const cased = sentence.charAt(0).toUpperCase() + sentence.slice(1);
      return { what: cased.endsWith(".") ? cased : `${cased}.`, next: "" };
    }
  }
}

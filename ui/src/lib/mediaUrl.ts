// The URL the webview can load a stored file from. Separate from lib/media.ts because it reaches
// into the Tauri runtime, which the pure helpers and their tests do not.

import { convertFileSrc } from "@tauri-apps/api/core";

export function mediaUrl(mediaDir: string | null, name: string): string | null {
  if (!mediaDir) return null;
  const separator = mediaDir.includes("\\") ? "\\" : "/";
  return convertFileSrc(`${mediaDir}${separator}${name}`);
}

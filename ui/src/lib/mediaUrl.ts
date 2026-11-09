// The URL the webview can load a stored file from. Separate from lib/media.ts because it reaches
// into the Tauri runtime, which the pure helpers and their tests do not.

import { convertFileSrc } from "@tauri-apps/api/core";

import type { Media } from "./api";
import { fileName } from "./media";
import { session } from "./session.svelte";

export function mediaUrl(mediaDir: string | null, name: string): string | null {
  if (!mediaDir) return null;
  const separator = mediaDir.includes("\\") ? "\\" : "/";
  return convertFileSrc(`${mediaDir}${separator}${name}`);
}

/** The URL of a stored file in the open vault, or null before the shell has said where media lives. */
export function mediaSrc(media: Pick<Media, "sha256" | "ext">): string | null {
  return mediaUrl(session.startup?.media_dir ?? null, fileName(media));
}

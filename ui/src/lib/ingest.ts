// Storing a dropped or pasted image through the shell. Beside mediaUrl.ts because it reaches the
// API, which the pure helpers in media.ts do not.

import { api, type Media } from "./api";
import { extensionHint } from "./media";

/** A pasted or picked File: the bytes go as the raw request body with the extension as a hint. */
export async function ingestFile(file: File): Promise<Media> {
  return api.ingestMedia(new Uint8Array(await file.arrayBuffer()), extensionHint(file.name, file.type));
}

/** A dropped file: core reads it from disk, so the bytes never pass through the webview. */
export function ingestPath(path: string): Promise<Media> {
  return api.ingestMediaPath(path);
}

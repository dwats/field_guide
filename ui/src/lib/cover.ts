// The cover crop box's maths (design section 3): a picture shown behind a frame, panned and
// zoomed, and the crop the frame then holds, in thousandths of the picture. Pure, so it is tested
// without a DOM; the dialog and the poster frame call these.

import type { CoverCrop } from "./api";

export interface Size {
  width: number;
  height: number;
}

/** Where the picture sits behind the frame: a zoom over the cover scale and an offset in pixels. */
export interface Pan {
  zoom: number;
  ox: number;
  oy: number;
}

export const MIN_ZOOM = 1;
export const MAX_ZOOM = 4;

/** The scale at which the picture just covers the frame. */
export function coverScale(image: Size, frame: Size): number {
  return Math.max(frame.width / image.width, frame.height / image.height);
}

/** The picture's displayed size at a zoom. */
export function displayed(image: Size, frame: Size, zoom: number): Size {
  const scale = coverScale(image, frame) * zoom;
  return { width: image.width * scale, height: image.height * scale };
}

/** Keeps the frame inside the picture. */
export function clampOffset(ox: number, oy: number, image: Size, frame: Size, zoom: number): Pan {
  const d = displayed(image, frame, zoom);
  return {
    zoom,
    ox: Math.min(0, Math.max(frame.width - d.width, ox)),
    oy: Math.min(0, Math.max(frame.height - d.height, oy)),
  };
}

/** The pan that centres the picture at a zoom. */
export function centred(image: Size, frame: Size, zoom = MIN_ZOOM): Pan {
  const d = displayed(image, frame, zoom);
  return { zoom, ox: (frame.width - d.width) / 2, oy: (frame.height - d.height) / 2 };
}

/** The same picture point stays under the frame's centre while the zoom changes. */
export function rezoom(pan: Pan, image: Size, frame: Size, zoom: number): Pan {
  const before = displayed(image, frame, pan.zoom);
  const fx = (frame.width / 2 - pan.ox) / before.width;
  const fy = (frame.height / 2 - pan.oy) / before.height;
  const z = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, zoom));
  const after = displayed(image, frame, z);
  return clampOffset(frame.width / 2 - fx * after.width, frame.height / 2 - fy * after.height, image, frame, z);
}

/** The crop the frame shows, in thousandths of the picture, kept inside it. */
export function cropFor(pan: Pan, image: Size, frame: Size): CoverCrop {
  const d = displayed(image, frame, pan.zoom);
  const { ox, oy } = clampOffset(pan.ox, pan.oy, image, frame, pan.zoom);
  const w = Math.max(1, Math.min(1000, Math.round((frame.width / d.width) * 1000)));
  const h = Math.max(1, Math.min(1000, Math.round((frame.height / d.height) * 1000)));
  const x = Math.max(0, Math.min(1000 - w, Math.round((-ox / d.width) * 1000)));
  const y = Math.max(0, Math.min(1000 - h, Math.round((-oy / d.height) * 1000)));
  return { x, y, w, h };
}

/** The pan that shows a stored crop; the centre at zoom 1 when there is none. */
export function panFor(crop: CoverCrop | null, image: Size, frame: Size): Pan {
  if (!crop) return centred(image, frame);
  const width = (frame.width * 1000) / crop.w;
  const zoom = Math.min(MAX_ZOOM, Math.max(MIN_ZOOM, width / image.width / coverScale(image, frame)));
  const d = displayed(image, frame, zoom);
  return clampOffset((-crop.x / 1000) * d.width, (-crop.y / 1000) * d.height, image, frame, zoom);
}

/** Inline style for the picture inside a frame, so any size of frame shows the same crop. */
export function cropStyle(crop: CoverCrop | null): string {
  if (!crop) return "width:100%;height:100%;object-fit:cover;";
  const width = 100000 / crop.w;
  const height = 100000 / crop.h;
  const left = -(crop.x / crop.w) * 100;
  const top = -(crop.y / crop.h) * 100;
  return `width:${width}%;height:${height}%;left:${left}%;top:${top}%;`;
}

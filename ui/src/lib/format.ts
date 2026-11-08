// Presentation helpers shared by the views.

/** Whatever stands in the verb slot reads uppercase (design section 4). */
export function verbSlot(label: string): string {
  return label.toUpperCase();
}

export function whenDate(iso: string): string {
  return new Date(iso).toLocaleDateString();
}

export function whenTime(iso: string): string {
  return new Date(iso).toLocaleString();
}

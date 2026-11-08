// Presentation helpers shared by the views.

/** Whatever stands in the verb slot reads uppercase (design section 4). */
export function verbSlot(label: string): string {
  return label.toUpperCase();
}

/** The one timestamp format: local time, to the minute, `YYYY-MM-DD HH:mm`. */
export function when(iso: string): string {
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return iso;
  const pad = (n: number) => String(n).padStart(2, "0");
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

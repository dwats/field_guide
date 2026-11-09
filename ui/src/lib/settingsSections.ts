// The Settings page's sections, in page order. The page gives each section its id from here and
// the sidebar lists and scrolls to them, so the two cannot drift apart.

export const SECTIONS = [
  { id: "appearance", label: "Appearance" },
  { id: "data", label: "Data" },
  { id: "backups", label: "Backups" },
  { id: "updates", label: "Updates" },
] as const;

export type SectionId = (typeof SECTIONS)[number]["id"];

export function sectionElementId(id: SectionId): string {
  return `settings-${id}`;
}

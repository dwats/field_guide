// Typed wrappers around the shell's commands. Field names are snake_case on both sides; the
// shapes mirror the structs in `core`. The UI calls these, renders the result, and sends actions
// back; no rules live here.

import { invoke } from "@tauri-apps/api/core";

export type Id = string;

export interface AppError {
  kind: string;
  message: string;
  /** The file or folder the error is about, when the shell knows one. */
  path?: string;
}

export function isAppError(e: unknown): e is AppError {
  return typeof e === "object" && e !== null && "kind" in e && "message" in e;
}

export function errorMessage(e: unknown): string {
  if (isAppError(e)) return e.message;
  if (e instanceof Error) return e.message;
  return String(e);
}

// ---- shell ----

export interface BackupInfo {
  path: string;
  created_at: string;
  bytes: number;
}

export interface Startup {
  version: string;
  fatal: AppError | null;
  mode: "default" | "portable" | null;
  data_dir: string | null;
  vault_dir: string | null;
  media_dir: string | null;
  backups_dir: string | null;
  backup_taken: BackupInfo | null;
  backup_error: AppError | null;
  sync_warning: string | null;
}

export type GuideView = "cards" | "table";
export type Density = "compact" | "roomy";

export interface UiPrefs {
  guide_view: GuideView;
  density: Density;
  sidebar_collapsed: boolean;
  panel_collapsed: boolean;
  /** Keep the right panel open while editing in Split. */
  panel_open_in_edit: boolean;
  /** Never show the table of contents. */
  hide_contents: boolean;
  /** Column widths in CSS pixels once dragged; null takes the density preset's. */
  sidebar_width: number | null;
  panel_width: number | null;
  /** Share of the sidebar's height given to the tag tree, in percent; null is half. */
  sidebar_split: number | null;
  /** Unused media above this many megabytes raises a notice at start; null is 250. */
  unreferenced_warning_mb: number | null;
}

// ---- guides ----

/** What cards show of a cover, in thousandths of the picture's width and height. */
export interface CoverCrop {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Guide {
  id: Id;
  name: string;
  description: string;
  cover_media_id: Id | null;
  cover_crop: CoverCrop | null;
  tag_ids: Id[];
  created_at: string;
  updated_at: string;
}

export interface GuideFilter {
  name_contains: string | null;
  tag_ids: Id[];
}

// ---- tags ----

export interface Tag {
  id: Id;
  scope_guide_id: Id | null;
  parent_id: Id | null;
  name: string;
  color: string | null;
}

export interface TagNode {
  tag: Tag;
  path: string;
  children: TagNode[];
}

export interface TagCount {
  tag_id: Id;
  /** Live entries carrying the tag itself. */
  direct: number;
  /** Distinct live entries carrying the tag or any tag under it. */
  subtree: number;
}

export interface EntryCounts {
  total: number;
  untagged: number;
  trashed: number;
  by_tag: TagCount[];
}

// ---- entries ----

export interface Entry {
  id: Id;
  guide_id: Id;
  title: string;
  body_md: string;
  is_stub: boolean;
  deleted_at: string | null;
  created_at: string;
  updated_at: string;
  aliases: string[];
  tag_ids: Id[];
}

export interface EntrySummary {
  id: Id;
  guide_id: Id;
  title: string;
  is_stub: boolean;
  deleted_at: string | null;
  created_at: string;
  updated_at: string;
  /** The entry's tags, by name. */
  tag_ids: Id[];
}

export interface EntryQuery {
  tag_ids: Id[];
  text: string | null;
  stubs_only: boolean;
  /** Only entries with no tag at all. */
  untagged: boolean;
}

export interface RenameRecord {
  entry_id: Id;
  old_title: string;
  new_title: string;
  alias_added: boolean;
  alias_promoted: string | null;
  rewritten_entry_ids: Id[];
}

export interface NameMatch {
  entry_id: Id;
  title: string;
  matched: string;
  is_alias: boolean;
  is_stub: boolean;
}

export interface Heading {
  /** 1 to 6. */
  level: number;
  text: string;
  /** The id on the heading element. */
  id: string;
}

export interface Rendered {
  html: string;
  unresolved_links: string[];
  linked_entry_ids: Id[];
  /** The body's headings in order. */
  headings: Heading[];
}

// ---- media ----

export interface MediaSummary {
  files: number;
  bytes: number;
  unreferenced_files: number;
  unreferenced_bytes: number;
  /** Free space on the vault's drive, or null when unknown. */
  free_space: number | null;
  /** Unused media is over the warning threshold (core applies the setting). */
  warn_unreferenced: boolean;
  /** The vault's drive has under a gigabyte free. */
  warn_low_space: boolean;
}

export interface CleanupReport {
  removed: number;
  bytes: number;
}

export interface Media {
  id: Id;
  /** Lowercase hex; the file is `media/<sha256>.<ext>`. */
  sha256: string;
  ext: string;
  mime: string;
  bytes: number;
  width: number | null;
  height: number | null;
  duration_ms: number | null;
  created_at: string;
}

// ---- boards ----

/** The picker's last filters and the display toggles. */
export interface BoardFilters {
  tag_ids: Id[];
  text: string | null;
  stubs_only: boolean;
  /** Show cards from other guides and the edges that reach them. */
  show_external: boolean;
  /** Show the dashed mention layer. */
  show_mentions: boolean;
  background: Background;
}

/** The canvas pattern behind the cards. */
export type Background = "dots" | "cork" | "grid" | "blank";

export interface Board {
  id: Id;
  guide_id: Id;
  name: string;
  filters: BoardFilters;
  created_at: string;
  updated_at: string;
}

export interface BoardNode {
  entry_id: Id;
  x: number;
  y: number;
}

export interface BoardCard {
  entry: EntryRef;
  x: number;
  y: number;
  /** The entry's primary image, or null for a plain card. */
  image: Media | null;
  /** The entry belongs to another guide. */
  external: boolean;
}

export type EdgeKind = "relation" | "mention";

export interface BoardEdge {
  kind: EdgeKind;
  /** The relation id, or `mention:<from>:<to>`. */
  id: string;
  from: Id;
  to: Id;
  /** The words in the verb slot; empty for a mention. */
  label: string;
  /** The qualifiers as one line; empty without qualifiers. */
  detail: string;
  symmetric: boolean;
  external: boolean;
}

/** A free-standing markdown note on a board; content taller than `h` scrolls inside it. */
export interface BoardNote {
  id: Id;
  board_id: Id;
  body_md: string;
  x: number;
  y: number;
  w: number;
  h: number;
  color: string;
  created_at: string;
  updated_at: string;
}

/** Only the fields that are set change. */
export interface BoardNoteUpdate {
  body_md?: string;
  x?: number;
  y?: number;
  w?: number;
  h?: number;
  color?: string;
}

export type EndKind = "entry" | "note" | "image";

/** One end of a string: an entry card (its entry id), a note, or a board image (their ids). */
export interface StringEnd {
  kind: EndKind;
  id: Id;
}

export interface BoardString {
  id: Id;
  board_id: Id;
  from: StringEnd;
  to: StringEnd;
  label: string;
  color: string;
  created_at: string;
}

/** Only the fields that are set change. */
export interface BoardStringUpdate {
  label?: string;
  color?: string;
}

/** A note as drawn: the row plus its rendered body; `tasks` counts its task markers. */
export interface BoardNoteView {
  note: BoardNote;
  html: string;
  tasks: number;
}

export const NOTE_COLORS = ["yellow", "pink", "blue", "green", "white"] as const;
export const STRING_COLORS = ["red", "blue", "green", "black"] as const;
export const NOTE_DEFAULT_SIZE = 200;
export const NOTE_MIN_SIZE = 80;
export const IMAGE_MIN_SIZE = 40;
export const IMAGE_DEFAULT_WIDTH = 200;

/** A picture placed on a board by itself, drawn in a polaroid frame. */
export interface BoardImage {
  id: Id;
  board_id: Id;
  media: Media;
  x: number;
  y: number;
  w: number;
  h: number;
  created_at: string;
}

/** Only the fields that are set change. */
export interface BoardImageUpdate {
  x?: number;
  y?: number;
  w?: number;
  h?: number;
}

export interface BoardView {
  board: Board;
  cards: BoardCard[];
  edges: BoardEdge[];
  notes: BoardNoteView[];
  strings: BoardString[];
  images: BoardImage[];
}

// ---- relations and vocabulary (used from phase 5 on) ----

export interface Verb {
  id: Id;
  guide_id: Id;
  label: string;
  inverse_label: string | null;
  symmetric: boolean;
  subject_default_tag_id: Id | null;
  object_default_tag_id: Id | null;
}

export interface QualifierLabel {
  id: Id;
  guide_id: Id;
  label: string;
  inverse_label: string | null;
}

export interface EntryRef {
  id: Id;
  guide_id: Id;
  title: string;
  is_stub: boolean;
  deleted_at: string | null;
}

export type QualifierValueView =
  | { kind: "entry"; value: EntryRef }
  | { kind: "text"; value: string }
  | { kind: "number"; value: number };

export interface Qualifier {
  id: Id;
  label: QualifierLabel;
  value: QualifierValueView;
  sort: number;
}

export interface Relation {
  id: Id;
  subject: EntryRef;
  verb: Verb;
  object: EntryRef;
  qualifiers: Qualifier[];
  created_at: string;
}

export interface Segment {
  text: string;
  entry_id?: Id;
  /** The words in the verb slot; a list grouped by verb does not repeat them per line. */
  verb_slot?: boolean;
}

export interface RelationView {
  relation: Relation;
  perspective: "subject" | "object" | "qualifier_value";
  qualifier_id: Id | null;
  section: "relations" | "referenced_by";
  label: string;
  inverted: boolean;
  external: boolean;
  text: string;
  segments: Segment[];
}

export type Direction = "forward" | "inverse";

export interface VerbMatch {
  verb: Verb;
  direction: Direction;
}

export interface NewVerb {
  label: string;
  inverse_label: string | null;
  symmetric: boolean;
  subject_default_tag_id: Id | null;
  object_default_tag_id: Id | null;
}

export interface NewQualifierLabel {
  label: string;
  inverse_label: string | null;
}

export type QualifierValue =
  | { kind: "entry"; value: Id }
  | { kind: "text"; value: string }
  | { kind: "number"; value: number }
  | { kind: "stub"; value: { title: string; tag_id: Id | null } };

/** A verb from the pool, or a new one that core creates with the edge. */
export type VerbSpec = { kind: "existing"; id: Id } | ({ kind: "new" } & NewVerb);

/** A qualifier label from the pool, or a new one that core creates with the qualifier. */
export type LabelSpec = { kind: "existing"; id: Id } | ({ kind: "new" } & NewQualifierLabel);

export interface NewQualifier {
  label: LabelSpec;
  value: QualifierValue;
}

export type Target = { kind: "entry"; id: Id } | { kind: "stub"; title: string; tag_id: Id | null };

export interface CreateRelation {
  origin: Id;
  verb: VerbSpec;
  direction: Direction;
  target: Target;
  qualifiers: NewQualifier[];
}

// ---- calls ----

export const api = {
  bootstrap: () => invoke<Startup>("bootstrap"),
  getPrefs: () => invoke<UiPrefs>("get_prefs"),
  setPrefs: (prefs: UiPrefs) => invoke<UiPrefs>("set_prefs", { prefs }),

  listGuides: (filter: Partial<GuideFilter> = {}) =>
    invoke<Guide[]>("list_guides", {
      filter: { name_contains: filter.name_contains ?? null, tag_ids: filter.tag_ids ?? [] },
    }),
  getGuide: (id: Id) => invoke<Guide>("get_guide", { id }),
  createGuide: (name: string, description = "", copyVocabularyFrom: Id | null = null) =>
    invoke<Guide>("create_guide", {
      new: { name, description },
      copy_vocabulary_from: copyVocabularyFrom,
    }),
  updateGuide: (id: Id, update: { name?: string; description?: string }) =>
    invoke<Guide>("update_guide", {
      id,
      update: { name: update.name ?? null, description: update.description ?? null },
    }),
  deleteGuide: (id: Id) => invoke<void>("delete_guide", { id }),
  addGuideTag: (id: Id, tag_id: Id) => invoke<Guide>("add_guide_tag", { id, tag_id }),
  removeGuideTag: (id: Id, tag_id: Id) => invoke<Guide>("remove_guide_tag", { id, tag_id }),

  // `scope` is a guide id, or null for the global tags applied to guides.
  tagTree: (scope: Id | null) => invoke<TagNode[]>("tag_tree", { scope }),
  listTags: (scope: Id | null) => invoke<Tag[]>("list_tags", { scope }),
  tagPath: (id: Id) => invoke<string>("tag_path", { id }),
  createTag: (scope: Id | null, parent: Id | null, name: string) =>
    invoke<Tag>("create_tag", { scope, parent, name }),
  findTagByPath: (scope: Id | null, path: string) =>
    invoke<Tag | null>("find_tag_by_path", { scope, path }),
  ensureTagPath: (scope: Id | null, path: string) =>
    invoke<Tag>("ensure_tag_path", { scope, path }),
  renameTag: (id: Id, name: string) => invoke<Tag>("rename_tag", { id, name }),
  moveTag: (id: Id, new_parent: Id | null) => invoke<Tag>("move_tag", { id, new_parent }),
  setTagColor: (id: Id, color: string | null) => invoke<Tag>("set_tag_color", { id, color }),
  deleteTag: (id: Id) => invoke<void>("delete_tag", { id }),
  mergeTags: (source: Id, target: Id) => invoke<void>("merge_tags", { source, target }),

  createEntry: (guide_id: Id, title: string, body_md = "", tag_ids: Id[] = []) =>
    invoke<Entry>("create_entry", { new: { guide_id, title, body_md, tag_ids } }),
  getEntry: (id: Id) => invoke<Entry>("get_entry", { id }),
  updateEntryBody: (id: Id, body_md: string) =>
    invoke<Entry>("update_entry_body", { id, body_md }),
  renameEntry: (id: Id, new_title: string) =>
    invoke<RenameRecord>("rename_entry", { id, new_title }),
  undoRename: (record: RenameRecord) => invoke<Entry>("undo_rename", { record }),
  addAlias: (id: Id, alias: string) => invoke<Entry>("add_alias", { id, alias }),
  removeAlias: (id: Id, alias: string) => invoke<Entry>("remove_alias", { id, alias }),
  setEntryTags: (id: Id, tag_ids: Id[]) => invoke<Entry>("set_entry_tags", { id, tag_ids }),
  addEntryTag: (id: Id, tag_id: Id) => invoke<Entry>("add_entry_tag", { id, tag_id }),
  removeEntryTag: (id: Id, tag_id: Id) => invoke<Entry>("remove_entry_tag", { id, tag_id }),
  listEntries: (guide_id: Id, query: Partial<EntryQuery> = {}) =>
    invoke<EntrySummary[]>("list_entries", {
      guide_id,
      query: {
        tag_ids: query.tag_ids ?? [],
        text: query.text ?? null,
        stubs_only: query.stubs_only ?? false,
        untagged: query.untagged ?? false,
      },
    }),
  lookupEntry: (guide_id: Id, text: string) =>
    invoke<EntrySummary | null>("lookup_entry", { guide_id, text }),
  mentionedBy: (id: Id) => invoke<EntrySummary[]>("mentioned_by", { id }),
  mentionsFrom: (id: Id) => invoke<EntrySummary[]>("mentions_from", { id }),
  trashEntry: (id: Id) => invoke<EntrySummary>("trash_entry", { id }),
  restoreEntry: (id: Id) => invoke<EntrySummary>("restore_entry", { id }),
  purgeEntry: (id: Id) => invoke<void>("purge_entry", { id }),
  listTrash: (guide_id: Id) => invoke<EntrySummary[]>("list_trash", { guide_id }),
  entryCounts: (guide_id: Id) => invoke<EntryCounts>("entry_counts", { guide_id }),
  emptyTrash: (guide_id: Id) => invoke<number>("empty_trash", { guide_id }),

  getMedia: (id: Id) => invoke<Media>("get_media", { id }),
  mediaByHash: (sha256: string) => invoke<Media | null>("media_by_hash", { sha256 }),
  entryGallery: (entry_id: Id) => invoke<Media[]>("entry_gallery", { entry_id }),
  attachMedia: (entry_id: Id, media_id: Id, sort: number | null = null) =>
    invoke<void>("attach_media", { entry_id, media_id, sort }),
  detachMedia: (entry_id: Id, media_id: Id) =>
    invoke<void>("detach_media", { entry_id, media_id }),
  setPrimaryMedia: (entry_id: Id, media_id: Id) =>
    invoke<void>("set_primary_media", { entry_id, media_id }),
  /** The bytes go as the raw request body; `ext` is a hint for formats without magic bytes. */
  ingestMedia: (bytes: Uint8Array, ext: string) =>
    invoke<Media>("ingest_media", bytes, { headers: { "x-ext": ext } }),
  ingestMediaPath: (path: string) => invoke<Media>("ingest_media_path", { path }),
  mediaSummary: () => invoke<MediaSummary>("media_summary"),
  unreferencedMedia: () => invoke<Media[]>("unreferenced_media"),
  purgeMedia: (ids: Id[]) => invoke<CleanupReport>("purge_media", { ids }),
  setGuideCover: (id: Id, media_id: Id | null, crop: CoverCrop | null = null) =>
    invoke<Guide>("set_guide_cover", { id, media_id, crop }),

  listBoards: (guide_id: Id) => invoke<Board[]>("list_boards", { guide_id }),
  createBoard: (guide_id: Id, name: string) => invoke<Board>("create_board", { guide_id, name }),
  getBoard: (id: Id) => invoke<Board>("get_board", { id }),
  renameBoard: (id: Id, name: string) => invoke<Board>("rename_board", { id, name }),
  setBoardFilters: (id: Id, filters: BoardFilters) =>
    invoke<Board>("set_board_filters", { id, filters }),
  deleteBoard: (id: Id) => invoke<void>("delete_board", { id }),
  boardView: (id: Id) => invoke<BoardView>("board_view", { id }),
  /** Puts entries on the board; the ones already there keep their place. Returns how many were added. */
  addBoardNodes: (id: Id, nodes: BoardNode[]) => invoke<number>("add_board_nodes", { id, nodes }),
  setBoardPositions: (id: Id, nodes: BoardNode[]) =>
    invoke<void>("set_board_positions", { id, nodes }),
  removeBoardNode: (id: Id, entry_id: Id) => invoke<void>("remove_board_node", { id, entry_id }),
  /** A new note, 200 by 200 and yellow, with its top-left corner at (x, y). */
  createBoardNote: (board_id: Id, body_md: string, x: number, y: number) =>
    invoke<BoardNote>("create_board_note", { board_id, body_md, x, y }),
  updateBoardNote: (id: Id, update: BoardNoteUpdate) =>
    invoke<BoardNote>("update_board_note", { id, update }),
  /** Deletes the note and every string tied to it. */
  deleteBoardNote: (id: Id) => invoke<void>("delete_board_note", { id }),
  /** Flips the index-th task marker (0-based, document order) in the note's markdown. */
  toggleNoteTask: (id: Id, index: number) => invoke<BoardNote>("toggle_note_task", { id, index }),
  createBoardString: (board_id: Id, from: StringEnd, to: StringEnd, label = "") =>
    invoke<BoardString>("create_board_string", { board_id, from, to, label }),
  updateBoardString: (id: Id, update: BoardStringUpdate) =>
    invoke<BoardString>("update_board_string", { id, update }),
  deleteBoardString: (id: Id) => invoke<void>("delete_board_string", { id }),
  /** Places stored media on a board with its top-left corner at (x, y). */
  createBoardImage: (board_id: Id, media_id: Id, x: number, y: number, w: number, h: number) =>
    invoke<BoardImage>("create_board_image", { board_id, media_id, x, y, w, h }),
  updateBoardImage: (id: Id, update: BoardImageUpdate) =>
    invoke<BoardImage>("update_board_image", { id, update }),
  /** Deletes the board image and every string tied to it; the media stays. */
  deleteBoardImage: (id: Id) => invoke<void>("delete_board_image", { id }),

  renderEntry: (id: Id) => invoke<Rendered>("render_entry", { id }),
  renderMarkdown: (guide_id: Id, body: string) =>
    invoke<Rendered>("render_markdown", { guide_id, body }),

  suggestNames: (guide_id: Id, prefix: string, limit = 8) =>
    invoke<NameMatch[]>("suggest_names", { guide_id, prefix, limit }),

  relationsForEntry: (id: Id, include_external = false) =>
    invoke<RelationView[]>("relations_for_entry", { id, filter: { include_external } }),
  createRelation: (request: CreateRelation) =>
    invoke<Relation>("create_relation", { request }),
  deleteRelation: (id: Id) => invoke<void>("delete_relation", { id }),
  replaceQualifiers: (id: Id, qualifiers: NewQualifier[]) =>
    invoke<Relation>("replace_qualifiers", { id, qualifiers }),
  suggestedStubTag: (verb_id: Id, direction: Direction) =>
    invoke<Id | null>("suggested_stub_tag", { verb_id, direction }),
  resolveQualifierValue: (guide_id: Id, text: string) =>
    invoke<QualifierValue>("resolve_qualifier_value", { guide_id, text }),

  listVerbs: (guide_id: Id) => invoke<Verb[]>("list_verbs", { guide_id }),
  lookupVerb: (guide_id: Id, text: string) =>
    invoke<VerbMatch | null>("lookup_verb", { guide_id, text }),
  listQualifierLabels: (guide_id: Id) =>
    invoke<QualifierLabel[]>("list_qualifier_labels", { guide_id }),
  lookupQualifierLabel: (guide_id: Id, text: string) =>
    invoke<QualifierLabel | null>("lookup_qualifier_label", { guide_id, text }),

  listBackups: () => invoke<BackupInfo[]>("list_backups"),
  backupNow: () => invoke<BackupInfo>("backup_now"),
  restoreBackup: (path: string) => invoke<BackupInfo[]>("restore_backup", { path }),
};

// Typed wrappers around the shell's commands. Field names are snake_case on both sides; the
// shapes mirror the structs in `core`. The UI calls these, renders the result, and sends actions
// back; no rules live here.

import { invoke } from "@tauri-apps/api/core";

export type Id = string;

export interface AppError {
  kind: string;
  message: string;
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

export interface UiPrefs {
  guide_view: GuideView;
}

// ---- guides ----

export interface Guide {
  id: Id;
  name: string;
  description: string;
  cover_media_id: Id | null;
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

// ---- notes ----

export interface Note {
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

export interface NoteSummary {
  id: Id;
  guide_id: Id;
  title: string;
  is_stub: boolean;
  deleted_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface NoteQuery {
  tag_id: Id | null;
  text: string | null;
  stubs_only: boolean;
}

export interface RenameRecord {
  note_id: Id;
  old_title: string;
  new_title: string;
  alias_added: boolean;
  alias_promoted: string | null;
  rewritten_note_ids: Id[];
}

export interface Rendered {
  html: string;
  unresolved_links: string[];
  linked_note_ids: Id[];
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

export interface NoteRef {
  id: Id;
  guide_id: Id;
  title: string;
  is_stub: boolean;
  deleted_at: string | null;
}

export type QualifierValueView =
  | { kind: "note"; value: NoteRef }
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
  subject: NoteRef;
  verb: Verb;
  object: NoteRef;
  qualifiers: Qualifier[];
  created_at: string;
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
  ensureTagPath: (scope: Id | null, path: string) =>
    invoke<Tag>("ensure_tag_path", { scope, path }),
  renameTag: (id: Id, name: string) => invoke<Tag>("rename_tag", { id, name }),
  moveTag: (id: Id, new_parent: Id | null) => invoke<Tag>("move_tag", { id, new_parent }),
  setTagColor: (id: Id, color: string | null) => invoke<Tag>("set_tag_color", { id, color }),
  deleteTag: (id: Id) => invoke<void>("delete_tag", { id }),
  mergeTags: (source: Id, target: Id) => invoke<void>("merge_tags", { source, target }),

  createNote: (guide_id: Id, title: string, body_md = "", tag_ids: Id[] = []) =>
    invoke<Note>("create_note", { new: { guide_id, title, body_md, tag_ids } }),
  getNote: (id: Id) => invoke<Note>("get_note", { id }),
  updateNoteBody: (id: Id, body_md: string) =>
    invoke<Note>("update_note_body", { id, body_md }),
  renameNote: (id: Id, new_title: string) =>
    invoke<RenameRecord>("rename_note", { id, new_title }),
  undoRename: (record: RenameRecord) => invoke<Note>("undo_rename", { record }),
  addAlias: (id: Id, alias: string) => invoke<Note>("add_alias", { id, alias }),
  removeAlias: (id: Id, alias: string) => invoke<Note>("remove_alias", { id, alias }),
  setNoteTags: (id: Id, tag_ids: Id[]) => invoke<Note>("set_note_tags", { id, tag_ids }),
  addNoteTag: (id: Id, tag_id: Id) => invoke<Note>("add_note_tag", { id, tag_id }),
  removeNoteTag: (id: Id, tag_id: Id) => invoke<Note>("remove_note_tag", { id, tag_id }),
  listNotes: (guide_id: Id, query: Partial<NoteQuery> = {}) =>
    invoke<NoteSummary[]>("list_notes", {
      guide_id,
      query: {
        tag_id: query.tag_id ?? null,
        text: query.text ?? null,
        stubs_only: query.stubs_only ?? false,
      },
    }),
  lookupNote: (guide_id: Id, text: string) =>
    invoke<NoteSummary | null>("lookup_note", { guide_id, text }),
  mentionedBy: (id: Id) => invoke<NoteSummary[]>("mentioned_by", { id }),
  mentionsFrom: (id: Id) => invoke<NoteSummary[]>("mentions_from", { id }),
  trashNote: (id: Id) => invoke<NoteSummary>("trash_note", { id }),
  restoreNote: (id: Id) => invoke<NoteSummary>("restore_note", { id }),
  purgeNote: (id: Id) => invoke<void>("purge_note", { id }),
  listTrash: (guide_id: Id) => invoke<NoteSummary[]>("list_trash", { guide_id }),
  emptyTrash: (guide_id: Id) => invoke<number>("empty_trash", { guide_id }),

  renderNote: (id: Id) => invoke<Rendered>("render_note", { id }),
  renderMarkdown: (guide_id: Id, body: string) =>
    invoke<Rendered>("render_markdown", { guide_id, body }),

  relationsForNote: (id: Id, include_external = false) =>
    invoke<RelationView[]>("relations_for_note", { id, filter: { include_external } }),
  listVerbs: (guide_id: Id) => invoke<Verb[]>("list_verbs", { guide_id }),
  listQualifierLabels: (guide_id: Id) =>
    invoke<QualifierLabel[]>("list_qualifier_labels", { guide_id }),

  listBackups: () => invoke<BackupInfo[]>("list_backups"),
  backupNow: () => invoke<BackupInfo>("backup_now"),
  restoreBackup: (path: string) => invoke<BackupInfo[]>("restore_backup", { path }),
};

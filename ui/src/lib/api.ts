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
}

export interface EntryQuery {
  tag_ids: Id[];
  text: string | null;
  stubs_only: boolean;
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

export interface Rendered {
  html: string;
  unresolved_links: string[];
  linked_entry_ids: Id[];
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
  emptyTrash: (guide_id: Id) => invoke<number>("empty_trash", { guide_id }),

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

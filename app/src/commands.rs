//! IPC commands. Each one forwards to `fieldguide_core` and converts the error. Argument names are
//! snake_case on both sides so the UI can pass core's structs straight through.
//!
//! Commands that touch the disk for more than a moment (backups, archives, media) are `async` so
//! they run off the main thread; the rest are quick SQLite calls.

use std::path::{Path, PathBuf};

use fieldguide_core::backup::{self, BACKUPS_TO_KEEP, BackupInfo};
use fieldguide_core::board::{Board, BoardFilters, BoardNode};
use fieldguide_core::config::UiPrefs;
use fieldguide_core::export::{ExportReport, ExportScope};
use fieldguide_core::guide::{Guide, GuideFilter, GuideUpdate, NewGuide};
use fieldguide_core::import::{
    self, ArchiveSummary, GuideChoice, ImportOptions, ImportPlan, ImportReport,
};
use fieldguide_core::markdown::Rendered;
use fieldguide_core::media::{Media, MediaDimensions};
use fieldguide_core::note::{NameMatch, NewNote, Note, NoteQuery, NoteSummary, RenameRecord};
use fieldguide_core::relation::{
    CreateRelation, NewQualifier, QualifierValue, Relation, RelationFilter, RelationView,
};
use fieldguide_core::tag::{Tag, TagNode};
use fieldguide_core::vocabulary::{
    Direction, NewQualifierLabel, NewVerb, QualifierLabel, Verb, VerbMatch, VocabularyCopy,
};
use fieldguide_core::{
    BoardId, GuideId, MediaId, NoteId, QualifierId, QualifierLabelId, RelationId, TagId, Vault,
    VerbId,
};
use tauri::State;

use crate::error::{AppError, CmdResult};
use crate::state::{AppState, StartupReport};

type S<'a> = State<'a, AppState>;

// ---- shell ----

#[tauri::command(rename_all = "snake_case")]
pub fn bootstrap(state: S<'_>) -> StartupReport {
    state.startup.clone()
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_prefs(state: S<'_>) -> CmdResult<UiPrefs> {
    state.with_config(|c| (c.ui.clone(), false))
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_prefs(state: S<'_>, prefs: UiPrefs) -> CmdResult<UiPrefs> {
    state.with_config(|c| {
        c.ui = prefs;
        (c.ui.clone(), true)
    })
}

// ---- guides ----

#[tauri::command(rename_all = "snake_case")]
pub fn list_guides(state: S<'_>, filter: GuideFilter) -> CmdResult<Vec<Guide>> {
    state.with_vault(|v| v.list_guides(&filter))
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_guide(state: S<'_>, id: GuideId) -> CmdResult<Guide> {
    state.with_vault(|v| v.get_guide(&id))
}

/// Creates a guide, optionally copying another guide's verbs and qualifier labels into it.
#[tauri::command(rename_all = "snake_case")]
pub fn create_guide(
    state: S<'_>,
    new: NewGuide,
    copy_vocabulary_from: Option<GuideId>,
) -> CmdResult<Guide> {
    state.with_vault(|v| {
        let guide = v.create_guide(new)?;
        if let Some(from) = copy_vocabulary_from {
            v.copy_vocabulary(&from, &guide.id)?;
        }
        Ok(guide)
    })
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_guide(state: S<'_>, id: GuideId, update: GuideUpdate) -> CmdResult<Guide> {
    state.with_vault(|v| v.update_guide(&id, update))
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_guide_cover(state: S<'_>, id: GuideId, media_id: Option<MediaId>) -> CmdResult<Guide> {
    state.with_vault(|v| v.set_guide_cover(&id, media_id.as_ref()))
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_guide(state: S<'_>, id: GuideId) -> CmdResult<()> {
    state.with_vault(|v| v.delete_guide(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn add_guide_tag(state: S<'_>, id: GuideId, tag_id: TagId) -> CmdResult<Guide> {
    state.with_vault(|v| v.add_guide_tag(&id, &tag_id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_guide_tag(state: S<'_>, id: GuideId, tag_id: TagId) -> CmdResult<Guide> {
    state.with_vault(|v| v.remove_guide_tag(&id, &tag_id))
}

// ---- tags ----
// `scope` is a guide id for that guide's tree, or `None` for the global tags applied to guides.

#[tauri::command(rename_all = "snake_case")]
pub fn tag_tree(state: S<'_>, scope: Option<GuideId>) -> CmdResult<Vec<TagNode>> {
    state.with_vault(|v| v.tag_tree(scope.as_ref()))
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_tags(state: S<'_>, scope: Option<GuideId>) -> CmdResult<Vec<Tag>> {
    state.with_vault(|v| v.list_tags(scope.as_ref()))
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_tag(state: S<'_>, id: TagId) -> CmdResult<Tag> {
    state.with_vault(|v| v.get_tag(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn tag_path(state: S<'_>, id: TagId) -> CmdResult<String> {
    state.with_vault(|v| v.tag_path(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_tag(
    state: S<'_>,
    scope: Option<GuideId>,
    parent: Option<TagId>,
    name: String,
) -> CmdResult<Tag> {
    state.with_vault(|v| v.create_tag(scope.as_ref(), parent.as_ref(), &name))
}

#[tauri::command(rename_all = "snake_case")]
pub fn ensure_tag_path(state: S<'_>, scope: Option<GuideId>, path: String) -> CmdResult<Tag> {
    state.with_vault(|v| v.ensure_tag_path(scope.as_ref(), &path))
}

#[tauri::command(rename_all = "snake_case")]
pub fn rename_tag(state: S<'_>, id: TagId, name: String) -> CmdResult<Tag> {
    state.with_vault(|v| v.rename_tag(&id, &name))
}

#[tauri::command(rename_all = "snake_case")]
pub fn move_tag(state: S<'_>, id: TagId, new_parent: Option<TagId>) -> CmdResult<Tag> {
    state.with_vault(|v| v.move_tag(&id, new_parent.as_ref()))
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_tag_color(state: S<'_>, id: TagId, color: Option<String>) -> CmdResult<Tag> {
    state.with_vault(|v| v.set_tag_color(&id, color.as_deref()))
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_tag(state: S<'_>, id: TagId) -> CmdResult<()> {
    state.with_vault(|v| v.delete_tag(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn merge_tags(state: S<'_>, source: TagId, target: TagId) -> CmdResult<()> {
    state.with_vault(|v| v.merge_tags(&source, &target))
}

// ---- notes ----

#[tauri::command(rename_all = "snake_case")]
pub fn create_note(state: S<'_>, new: NewNote) -> CmdResult<Note> {
    state.with_vault(|v| v.create_note(new))
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_stub(
    state: S<'_>,
    guide_id: GuideId,
    title: String,
    tag_id: Option<TagId>,
) -> CmdResult<Note> {
    state.with_vault(|v| v.create_stub_note(&guide_id, &title, tag_id.as_ref()))
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_note(state: S<'_>, id: NoteId) -> CmdResult<Note> {
    state.with_vault(|v| v.get_note(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_note_body(state: S<'_>, id: NoteId, body_md: String) -> CmdResult<Note> {
    state.with_vault(|v| v.update_note_body(&id, &body_md))
}

#[tauri::command(rename_all = "snake_case")]
pub fn rename_note(state: S<'_>, id: NoteId, new_title: String) -> CmdResult<RenameRecord> {
    state.with_vault(|v| v.rename_note(&id, &new_title))
}

#[tauri::command(rename_all = "snake_case")]
pub fn undo_rename(state: S<'_>, record: RenameRecord) -> CmdResult<Note> {
    state.with_vault(|v| v.undo_rename(&record))
}

#[tauri::command(rename_all = "snake_case")]
pub fn add_alias(state: S<'_>, id: NoteId, alias: String) -> CmdResult<Note> {
    state.with_vault(|v| v.add_alias(&id, &alias))
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_alias(state: S<'_>, id: NoteId, alias: String) -> CmdResult<Note> {
    state.with_vault(|v| v.remove_alias(&id, &alias))
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_note_tags(state: S<'_>, id: NoteId, tag_ids: Vec<TagId>) -> CmdResult<Note> {
    state.with_vault(|v| v.set_note_tags(&id, &tag_ids))
}

#[tauri::command(rename_all = "snake_case")]
pub fn add_note_tag(state: S<'_>, id: NoteId, tag_id: TagId) -> CmdResult<Note> {
    state.with_vault(|v| v.add_note_tag(&id, &tag_id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_note_tag(state: S<'_>, id: NoteId, tag_id: TagId) -> CmdResult<Note> {
    state.with_vault(|v| v.remove_note_tag(&id, &tag_id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_notes(
    state: S<'_>,
    guide_id: GuideId,
    query: NoteQuery,
) -> CmdResult<Vec<NoteSummary>> {
    state.with_vault(|v| v.list_notes(&guide_id, &query))
}

#[tauri::command(rename_all = "snake_case")]
pub fn lookup_note(
    state: S<'_>,
    guide_id: GuideId,
    text: String,
) -> CmdResult<Option<NoteSummary>> {
    state.with_vault(|v| v.lookup_note(&guide_id, &text))
}

#[tauri::command(rename_all = "snake_case")]
pub fn suggest_names(
    state: S<'_>,
    guide_id: GuideId,
    prefix: String,
    limit: usize,
) -> CmdResult<Vec<NameMatch>> {
    state.with_vault(|v| v.suggest_names(&guide_id, &prefix, limit))
}

#[tauri::command(rename_all = "snake_case")]
pub fn lookup_note_any_guide(state: S<'_>, text: String) -> CmdResult<Vec<NoteSummary>> {
    state.with_vault(|v| v.lookup_note_any_guide(&text))
}

#[tauri::command(rename_all = "snake_case")]
pub fn mentioned_by(state: S<'_>, id: NoteId) -> CmdResult<Vec<NoteSummary>> {
    state.with_vault(|v| v.mentioned_by(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn mentions_from(state: S<'_>, id: NoteId) -> CmdResult<Vec<NoteSummary>> {
    state.with_vault(|v| v.mentions_from(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn trash_note(state: S<'_>, id: NoteId) -> CmdResult<NoteSummary> {
    state.with_vault(|v| v.trash_note(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn restore_note(state: S<'_>, id: NoteId) -> CmdResult<NoteSummary> {
    state.with_vault(|v| v.restore_note(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn purge_note(state: S<'_>, id: NoteId) -> CmdResult<()> {
    state.with_vault(|v| v.purge_note(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_trash(state: S<'_>, guide_id: GuideId) -> CmdResult<Vec<NoteSummary>> {
    state.with_vault(|v| v.list_trash(&guide_id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn empty_trash(state: S<'_>, guide_id: GuideId) -> CmdResult<usize> {
    state.with_vault(|v| v.empty_trash(&guide_id))
}

// ---- rendering ----

#[tauri::command(rename_all = "snake_case")]
pub fn render_note(state: S<'_>, id: NoteId) -> CmdResult<Rendered> {
    state.with_vault(|v| v.render_note(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn render_markdown(state: S<'_>, guide_id: GuideId, body: String) -> CmdResult<Rendered> {
    state.with_vault(|v| v.render_markdown(&guide_id, &body))
}

// ---- relations ----

#[tauri::command(rename_all = "snake_case")]
pub fn relations_for_note(
    state: S<'_>,
    id: NoteId,
    filter: RelationFilter,
) -> CmdResult<Vec<RelationView>> {
    state.with_vault(|v| v.relations_for_note(&id, &filter))
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_relation(state: S<'_>, request: CreateRelation) -> CmdResult<Relation> {
    state.with_vault(|v| v.create_relation(request))
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_relation(state: S<'_>, id: RelationId) -> CmdResult<Relation> {
    state.with_vault(|v| v.get_relation(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_relation(state: S<'_>, id: RelationId) -> CmdResult<()> {
    state.with_vault(|v| v.delete_relation(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn add_qualifier(state: S<'_>, id: RelationId, qualifier: NewQualifier) -> CmdResult<Relation> {
    state.with_vault(|v| v.add_qualifier(&id, qualifier))
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_qualifier(state: S<'_>, id: QualifierId) -> CmdResult<Relation> {
    state.with_vault(|v| v.remove_qualifier(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn replace_qualifiers(
    state: S<'_>,
    id: RelationId,
    qualifiers: Vec<NewQualifier>,
) -> CmdResult<Relation> {
    state.with_vault(|v| v.replace_qualifiers(&id, &qualifiers))
}

#[tauri::command(rename_all = "snake_case")]
pub fn suggested_stub_tag(
    state: S<'_>,
    verb_id: VerbId,
    direction: Direction,
) -> CmdResult<Option<TagId>> {
    state.with_vault(|v| v.suggested_stub_tag(&verb_id, direction))
}

#[tauri::command(rename_all = "snake_case")]
pub fn resolve_qualifier_value(
    state: S<'_>,
    guide_id: GuideId,
    text: String,
) -> CmdResult<QualifierValue> {
    state.with_vault(|v| v.resolve_qualifier_value(&guide_id, &text))
}

// ---- vocabulary ----

#[tauri::command(rename_all = "snake_case")]
pub fn list_verbs(state: S<'_>, guide_id: GuideId) -> CmdResult<Vec<Verb>> {
    state.with_vault(|v| v.list_verbs(&guide_id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_verb(state: S<'_>, guide_id: GuideId, new: NewVerb) -> CmdResult<Verb> {
    state.with_vault(|v| v.create_verb(&guide_id, new))
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_verb(state: S<'_>, id: VerbId, new: NewVerb) -> CmdResult<Verb> {
    state.with_vault(|v| v.update_verb(&id, new))
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_verb(state: S<'_>, id: VerbId) -> CmdResult<()> {
    state.with_vault(|v| v.delete_verb(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn merge_verbs(state: S<'_>, source: VerbId, target: VerbId) -> CmdResult<()> {
    state.with_vault(|v| v.merge_verbs(&source, &target))
}

#[tauri::command(rename_all = "snake_case")]
pub fn lookup_verb(state: S<'_>, guide_id: GuideId, text: String) -> CmdResult<Option<VerbMatch>> {
    state.with_vault(|v| v.lookup_verb(&guide_id, &text))
}

#[tauri::command(rename_all = "snake_case")]
pub fn list_qualifier_labels(state: S<'_>, guide_id: GuideId) -> CmdResult<Vec<QualifierLabel>> {
    state.with_vault(|v| v.list_qualifier_labels(&guide_id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_qualifier_label(
    state: S<'_>,
    guide_id: GuideId,
    new: NewQualifierLabel,
) -> CmdResult<QualifierLabel> {
    state.with_vault(|v| v.create_qualifier_label(&guide_id, new))
}

#[tauri::command(rename_all = "snake_case")]
pub fn update_qualifier_label(
    state: S<'_>,
    id: QualifierLabelId,
    new: NewQualifierLabel,
) -> CmdResult<QualifierLabel> {
    state.with_vault(|v| v.update_qualifier_label(&id, new))
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_qualifier_label(state: S<'_>, id: QualifierLabelId) -> CmdResult<()> {
    state.with_vault(|v| v.delete_qualifier_label(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn merge_qualifier_labels(
    state: S<'_>,
    source: QualifierLabelId,
    target: QualifierLabelId,
) -> CmdResult<()> {
    state.with_vault(|v| v.merge_qualifier_labels(&source, &target))
}

#[tauri::command(rename_all = "snake_case")]
pub fn lookup_qualifier_label(
    state: S<'_>,
    guide_id: GuideId,
    text: String,
) -> CmdResult<Option<QualifierLabel>> {
    state.with_vault(|v| v.lookup_qualifier_label(&guide_id, &text))
}

#[tauri::command(rename_all = "snake_case")]
pub fn copy_vocabulary(state: S<'_>, from: GuideId, to: GuideId) -> CmdResult<VocabularyCopy> {
    state.with_vault(|v| v.copy_vocabulary(&from, &to))
}

// ---- boards ----

#[tauri::command(rename_all = "snake_case")]
pub fn list_boards(state: S<'_>, guide_id: GuideId) -> CmdResult<Vec<Board>> {
    state.with_vault(|v| v.list_boards(&guide_id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn create_board(state: S<'_>, guide_id: GuideId, name: String) -> CmdResult<Board> {
    state.with_vault(|v| v.create_board(&guide_id, &name))
}

#[tauri::command(rename_all = "snake_case")]
pub fn get_board(state: S<'_>, id: BoardId) -> CmdResult<Board> {
    state.with_vault(|v| v.get_board(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn rename_board(state: S<'_>, id: BoardId, name: String) -> CmdResult<Board> {
    state.with_vault(|v| v.rename_board(&id, &name))
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_board_filters(state: S<'_>, id: BoardId, filters: BoardFilters) -> CmdResult<Board> {
    state.with_vault(|v| v.set_board_filters(&id, &filters))
}

#[tauri::command(rename_all = "snake_case")]
pub fn delete_board(state: S<'_>, id: BoardId) -> CmdResult<()> {
    state.with_vault(|v| v.delete_board(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn board_layout(state: S<'_>, id: BoardId) -> CmdResult<Vec<BoardNode>> {
    state.with_vault(|v| v.board_layout(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn set_board_positions(state: S<'_>, id: BoardId, nodes: Vec<BoardNode>) -> CmdResult<()> {
    state.with_vault(|v| v.set_board_positions(&id, &nodes))
}

#[tauri::command(rename_all = "snake_case")]
pub fn remove_board_node(state: S<'_>, id: BoardId, note_id: NoteId) -> CmdResult<()> {
    state.with_vault(|v| v.remove_board_node(&id, &note_id))
}

// ---- media ----

#[tauri::command(rename_all = "snake_case")]
pub fn get_media(state: S<'_>, id: MediaId) -> CmdResult<Media> {
    state.with_vault(|v| v.get_media(&id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn note_gallery(state: S<'_>, note_id: NoteId) -> CmdResult<Vec<Media>> {
    state.with_vault(|v| v.note_gallery(&note_id))
}

#[tauri::command(rename_all = "snake_case")]
pub fn attach_media(
    state: S<'_>,
    note_id: NoteId,
    media_id: MediaId,
    sort: Option<i64>,
) -> CmdResult<()> {
    state.with_vault(|v| v.attach_media(&note_id, &media_id, sort))
}

#[tauri::command(rename_all = "snake_case")]
pub fn detach_media(state: S<'_>, note_id: NoteId, media_id: MediaId) -> CmdResult<()> {
    state.with_vault(|v| v.detach_media(&note_id, &media_id))
}

/// Stores bytes as a media file. Phase 8 decides how large files cross the IPC boundary; until
/// then the bytes travel as a JSON array, which is fine for screenshots.
#[tauri::command(rename_all = "snake_case")]
pub async fn store_media(
    state: S<'_>,
    bytes: Vec<u8>,
    ext: String,
    mime: String,
    dimensions: MediaDimensions,
) -> CmdResult<Media> {
    state.with_vault(|v| v.store_media(&bytes, &ext, &mime, dimensions))
}

// ---- backups ----

#[tauri::command(rename_all = "snake_case")]
pub fn list_backups(state: S<'_>) -> CmdResult<Vec<BackupInfo>> {
    let dir = backups_dir(&state)?;
    backup::list_backups(&dir).map_err(AppError::from)
}

#[tauri::command(rename_all = "snake_case")]
pub async fn backup_now(state: S<'_>) -> CmdResult<BackupInfo> {
    let dir = backups_dir(&state)?;
    state.with_vault(|v| v.backup(&dir, BACKUPS_TO_KEEP))
}

/// Swaps the vault's database for a backup after backing up the current one. Returns the backup
/// list afterwards. On failure the vault is reopened as it was.
#[tauri::command(rename_all = "snake_case")]
pub async fn restore_backup(state: S<'_>, path: PathBuf) -> CmdResult<Vec<BackupInfo>> {
    let dir = backups_dir(&state)?;
    let mut guard = state.vault.lock().map_err(|_| AppError::poisoned())?;
    let vault = guard.take().ok_or_else(AppError::no_vault)?;
    let db_path = vault.path().map(Path::to_path_buf);
    match vault.restore_from_backup(&path, &dir, BACKUPS_TO_KEEP) {
        Ok(restored) => {
            *guard = Some(restored);
        }
        Err(e) => {
            if let Some(db_path) = db_path
                && let Ok(reopened) = Vault::open(&db_path)
            {
                *guard = Some(reopened);
            }
            return Err(e.into());
        }
    }
    drop(guard);
    backup::list_backups(&dir).map_err(AppError::from)
}

fn backups_dir(state: &AppState) -> CmdResult<PathBuf> {
    state
        .startup
        .backups_dir
        .clone()
        .ok_or_else(AppError::no_vault)
}

// ---- export and import ----

#[tauri::command(rename_all = "snake_case")]
pub async fn export_archive(
    state: S<'_>,
    scope: ExportScope,
    to: PathBuf,
) -> CmdResult<ExportReport> {
    state.with_vault(|v| v.export(&scope, &to))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn read_archive(path: PathBuf) -> CmdResult<ArchiveSummary> {
    Ok(import::read_archive(&path)?.summary())
}

#[tauri::command(rename_all = "snake_case")]
pub async fn propose_import(
    state: S<'_>,
    path: PathBuf,
    choices: Vec<GuideChoice>,
) -> CmdResult<ImportPlan> {
    let archive = import::read_archive(&path)?;
    state.with_vault(|v| v.propose_import(&archive, &choices))
}

#[tauri::command(rename_all = "snake_case")]
pub async fn apply_import(
    state: S<'_>,
    path: PathBuf,
    plan: ImportPlan,
    options: ImportOptions,
) -> CmdResult<ImportReport> {
    let archive = import::read_archive(&path)?;
    state.with_vault(|v| v.apply_import(&archive, &plan, &options))
}

//! Field Guide desktop shell: window, single-instance guard, and commands that forward to
//! `fieldguide_core`. Thin on purpose; business rules live in the core crate.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod error;
mod state;

use fieldguide_core::config::WindowState;
use tauri::{Manager, PhysicalPosition, PhysicalSize, WebviewWindow, WindowEvent};

use crate::state::AppState;

fn main() -> anyhow::Result<()> {
    let state = AppState::start();
    tauri::Builder::default()
        // Registered first so a second launch is answered before anything else runs.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .manage(state)
        .setup(|app| {
            let state = app.state::<AppState>();
            if let Some(media_dir) = &state.startup.media_dir {
                // The preview rewrites `media/...` to this protocol; only `media/` is reachable.
                app.asset_protocol_scope()
                    .allow_directory(media_dir, true)?;
            }
            if let Some(window) = app.get_webview_window("main") {
                let saved = state
                    .config
                    .lock()
                    .ok()
                    .and_then(|c| c.as_ref().and_then(|c| c.window.clone()));
                if let Some(saved) = saved {
                    apply_window_state(&window, &saved);
                }
                // The window starts hidden so it appears where it was, not at the default spot.
                let _ = window.show();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { .. } = event
                && let Ok(webview) = window.get_webview_window("main").ok_or(())
            {
                save_window_state(&webview);
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::bootstrap,
            commands::get_prefs,
            commands::set_prefs,
            commands::list_guides,
            commands::get_guide,
            commands::create_guide,
            commands::update_guide,
            commands::set_guide_cover,
            commands::delete_guide,
            commands::add_guide_tag,
            commands::remove_guide_tag,
            commands::tag_tree,
            commands::list_tags,
            commands::get_tag,
            commands::tag_path,
            commands::create_tag,
            commands::ensure_tag_path,
            commands::rename_tag,
            commands::move_tag,
            commands::set_tag_color,
            commands::delete_tag,
            commands::merge_tags,
            commands::create_entry,
            commands::create_stub,
            commands::get_entry,
            commands::update_entry_body,
            commands::rename_entry,
            commands::undo_rename,
            commands::add_alias,
            commands::remove_alias,
            commands::set_entry_tags,
            commands::add_entry_tag,
            commands::remove_entry_tag,
            commands::list_entries,
            commands::lookup_entry,
            commands::suggest_names,
            commands::lookup_entry_any_guide,
            commands::mentioned_by,
            commands::mentions_from,
            commands::trash_entry,
            commands::restore_entry,
            commands::purge_entry,
            commands::list_trash,
            commands::empty_trash,
            commands::render_entry,
            commands::render_markdown,
            commands::relations_for_entry,
            commands::create_relation,
            commands::get_relation,
            commands::delete_relation,
            commands::add_qualifier,
            commands::remove_qualifier,
            commands::replace_qualifiers,
            commands::suggested_stub_tag,
            commands::resolve_qualifier_value,
            commands::list_verbs,
            commands::create_verb,
            commands::update_verb,
            commands::delete_verb,
            commands::merge_verbs,
            commands::lookup_verb,
            commands::list_qualifier_labels,
            commands::create_qualifier_label,
            commands::update_qualifier_label,
            commands::delete_qualifier_label,
            commands::merge_qualifier_labels,
            commands::lookup_qualifier_label,
            commands::copy_vocabulary,
            commands::list_boards,
            commands::create_board,
            commands::get_board,
            commands::rename_board,
            commands::set_board_filters,
            commands::delete_board,
            commands::board_layout,
            commands::set_board_positions,
            commands::remove_board_node,
            commands::get_media,
            commands::entry_gallery,
            commands::attach_media,
            commands::detach_media,
            commands::store_media,
            commands::list_backups,
            commands::backup_now,
            commands::restore_backup,
            commands::export_archive,
            commands::read_archive,
            commands::propose_import,
            commands::apply_import,
        ])
        .run(tauri::generate_context!())?;
    Ok(())
}

/// Puts the window where it was last closed. Sizes below the configured minimum are ignored.
fn apply_window_state(window: &WebviewWindow, saved: &WindowState) {
    if saved.width >= 640 && saved.height >= 400 {
        let _ = window.set_size(PhysicalSize::new(saved.width, saved.height));
    }
    if let (Some(x), Some(y)) = (saved.x, saved.y) {
        let _ = window.set_position(PhysicalPosition::new(x, y));
    }
    if saved.maximized {
        let _ = window.maximize();
    }
}

/// Records size, position, and maximized state in `config.toml`. Best effort: a failure here must
/// never stop the window from closing.
fn save_window_state(window: &WebviewWindow) {
    let (Ok(size), Ok(position), Ok(maximized)) = (
        window.inner_size(),
        window.outer_position(),
        window.is_maximized(),
    ) else {
        return;
    };
    let state = window.state::<AppState>();
    let _ = state.with_config(|config| {
        let next = WindowState {
            width: size.width,
            height: size.height,
            x: Some(position.x),
            y: Some(position.y),
            maximized,
        };
        let changed = config.window.as_ref() != Some(&next);
        config.window = Some(next);
        ((), changed)
    });
}

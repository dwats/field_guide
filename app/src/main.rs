//! Field Guide desktop shell: window, menus, and commands that forward to `fieldguide_core`.
//! Thin on purpose; business rules live in the core crate.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> anyhow::Result<()> {
    tauri::Builder::default().run(tauri::generate_context!())?;
    Ok(())
}

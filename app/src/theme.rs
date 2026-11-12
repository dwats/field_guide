//! Native window theme: the title bar, WebView2's preferred color scheme, and the background
//! shown before the page paints.

use fieldguide_core::config::Theme;
use tauri::WebviewWindow;
use tauri::window::Color;

/// The page background per scheme. Must match `--bg` in `ui/src/app.css`.
const LIGHT_BG: Color = Color(0xff, 0xff, 0xff, 0xff);
const DARK_BG: Color = Color(0x1e, 0x1e, 0x1e, 0xff);

/// The window theme for a preference. `None` follows the OS.
pub fn native(theme: Theme) -> Option<tauri::Theme> {
    match theme {
        Theme::System => None,
        Theme::Light => Some(tauri::Theme::Light),
        Theme::Dark => Some(tauri::Theme::Dark),
    }
}

/// Sets the window theme (title bar, and WebView2's preferred scheme through the runtime) and the
/// background shown before the page paints.
pub fn apply(window: &WebviewWindow, theme: Theme) -> tauri::Result<()> {
    window.set_theme(native(theme))?;
    // When the preference is System this is the OS answer.
    let dark = window.theme()? == tauri::Theme::Dark;
    window.set_background_color(Some(if dark { DARK_BG } else { LIGHT_BG }))
}

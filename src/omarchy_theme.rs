//! Follow Omarchy's palette without restarting the player or changing widget geometry.
use gtk::{CssProvider, gdk::Display, glib};

pub fn start() {
    let state = std::env::var_os("XDG_STATE_HOME")
        .filter(|value| !value.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| glib::home_dir().join(".local/state"));
    let path = state.join("omarchy/current/theme/colors.toml");
    if !path.exists() {
        return;
    }
    let Some(display) = Display::default() else {
        return;
    };
    let provider = CssProvider::new();
    gtk::style_context_add_provider_for_display(
        &display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
    );
    let mut previous = String::new();
    let mut refresh = move || {
        // Read the path again: Omarchy replaces the entire theme directory atomically.
        // Keep the last valid palette while a theme is being staged or edited.
        if let Ok(source) = std::fs::read_to_string(&path) {
            if source != previous {
                if let Some((css, dark)) = palette_css(&source) {
                    provider.load_from_string(&css);
                    adw::StyleManager::default().set_color_scheme(if dark {
                        adw::ColorScheme::ForceDark
                    } else {
                        adw::ColorScheme::ForceLight
                    });
                    previous = source;
                    log::info!("Omarchy palette applied");
                }
            }
        }
    };
    refresh();
    // One small local file every two seconds, with no subprocesses and no redraw
    // unless its contents change. This also survives directory replacements.
    glib::timeout_add_seconds_local(2, move || {
        refresh();
        glib::ControlFlow::Continue
    });
}

fn palette_css(source: &str) -> Option<(String, bool)> {
    let colors: toml::Table = toml::from_str(source).ok()?;
    let get = |key: &str| {
        let value = colors.get(key)?.as_str()?;
        (value.len() == 7
            && value.starts_with('#')
            && value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit))
        .then_some(value)
    };
    let bg = get("background")?;
    let fg = get("foreground")?;
    let accent = get("accent")?;
    let panel = get("lighter_background").unwrap_or(bg);
    let dark_bg = get("dark_background").unwrap_or(bg);
    let selection = get("selection").unwrap_or(panel);
    let channel = |i| u8::from_str_radix(&accent[i..i + 2], 16).unwrap() as f64 / 255.0;
    let accent_fg = if 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5) > 0.5 {
        "#080910"
    } else {
        "#ffffff"
    };
    let palette = [
        ("window_bg_color", bg),
        ("window_fg_color", fg),
        ("view_bg_color", dark_bg),
        ("view_fg_color", fg),
        ("headerbar_bg_color", bg),
        ("headerbar_fg_color", fg),
        ("headerbar_backdrop_color", bg),
        ("sidebar_bg_color", dark_bg),
        ("sidebar_fg_color", fg),
        ("sidebar_backdrop_color", dark_bg),
        ("secondary_sidebar_bg_color", panel),
        ("secondary_sidebar_fg_color", fg),
        ("card_bg_color", panel),
        ("card_fg_color", fg),
        ("dialog_bg_color", panel),
        ("dialog_fg_color", fg),
        ("popover_bg_color", panel),
        ("popover_fg_color", fg),
        ("accent_bg_color", accent),
        ("accent_fg_color", accent_fg),
        ("accent_color", accent),
        ("theme_bg_color", bg),
        ("theme_fg_color", fg),
        ("theme_base_color", dark_bg),
        ("theme_text_color", fg),
        ("theme_selected_bg_color", selection),
        ("theme_selected_fg_color", fg),
    ];
    let mut css = String::new();
    for (key, value) in palette {
        css.push_str(&format!("@define-color {key} {value};\n"));
    }
    css.push_str(":root {\n");
    for (key, value) in palette {
        if !key.starts_with("theme_") {
            css.push_str(&format!("  --{}: {value};\n", key.replace('_', "-")));
        }
    }
    css.push_str("}\n");
    Some((
        css,
        colors.get("mode").and_then(toml::Value::as_str) != Some("light"),
    ))
}

#[cfg(test)]
mod tests {
    use super::palette_css;

    #[test]
    fn validates_palette_and_keeps_css_values_safe() {
        let source =
            "background = '#101020'\nforeground = '#eeeeff'\naccent = '#82FB9C'\nmode = 'light'\n";
        let (css, dark) = palette_css(source).unwrap();
        assert!(!dark);
        assert!(css.contains("--accent-bg-color: #82FB9C;"));
        assert!(css.contains("--accent-fg-color: #080910;"));
        assert!(palette_css(&source.replace("#82FB9C", "red; } button { opacity: 0")).is_none());
        assert!(palette_css("background = '#101020'").is_none());
        assert!(palette_css("background = '").is_none());
    }
}

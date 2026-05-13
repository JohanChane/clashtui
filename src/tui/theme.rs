use ratatui::style::{Color, Modifier, Style};
use std::collections::HashMap;
use std::sync::RwLock;

// ---- Deserialization types ----

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub(crate) struct StyleDef {
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    #[serde(default)]
    pub bold: bool,
}

impl Default for StyleDef {
    fn default() -> Self {
        Self { fg: None, bg: None, bold: false }
    }
}

impl From<StyleDef> for Style {
    fn from(def: StyleDef) -> Self {
        let mut s = Style::new();
        if let Some(fg) = def.fg {
            s = s.fg(fg);
        }
        if let Some(bg) = def.bg {
            s = s.bg(bg);
        }
        if def.bold {
            s = s.add_modifier(Modifier::BOLD);
        }
        s
    }
}

#[derive(serde::Deserialize, Default, Debug, Clone)]
#[serde(default)]
struct SectionPaletteDef {
    pub border: Option<StyleDef>,
    pub highlight: Option<StyleDef>,
    pub text: Option<StyleDef>,
    pub secondary: Option<StyleDef>,
    pub accent: Option<StyleDef>,
    #[serde(flatten)]
    pub extra: HashMap<String, StyleDef>,
}

#[derive(serde::Deserialize, Default, Debug, Clone)]
#[serde(default)]
struct TabbarDef {
    pub text: StyleDef,
    pub highlight: StyleDef,
}

#[derive(serde::Deserialize, Default, Debug, Clone)]
#[serde(default)]
struct PopupDef {
    pub border: StyleDef,
    pub text: StyleDef,
}

#[derive(serde::Deserialize, Default, Debug, Clone)]
#[serde(default)]
pub(crate) struct ThemeFile {
    pub tabbar: TabbarDef,
    pub popup: PopupDef,
    pub default: SectionPaletteDef,
    pub connections: Option<SectionPaletteDef>,
    pub proxies: Option<SectionPaletteDef>,
    pub settings: Option<SectionPaletteDef>,
    pub srvctl: Option<SectionPaletteDef>,
    pub logs: Option<SectionPaletteDef>,
    pub status: Option<SectionPaletteDef>,
}

// ---- Resolved runtime types ----

#[derive(Debug, Clone)]
pub(crate) struct ComputedSectionTheme {
    pub border: Style,
    pub highlight: Style,
    pub text: Style,
    pub secondary: Style,
    pub accent: Style,
    pub extra: HashMap<String, Style>,
}

impl ComputedSectionTheme {
    fn from_palette(palette: &SectionPaletteDef, default: &SectionPaletteDef) -> Self {
        fn resolve(def: &Option<StyleDef>, fallback: &Option<StyleDef>) -> Style {
            def.as_ref()
                .or(fallback.as_ref())
                .map(|d| Style::from(d.clone()))
                .unwrap_or_default()
        }
        Self {
            border: resolve(&palette.border, &default.border),
            highlight: resolve(&palette.highlight, &default.highlight),
            text: resolve(&palette.text, &default.text),
            secondary: resolve(&palette.secondary, &default.secondary),
            accent: resolve(&palette.accent, &default.accent),
            extra: {
                let mut merged = HashMap::new();
                for (k, v) in &default.extra {
                    merged.insert(k.clone(), Style::from(v.clone()));
                }
                for (k, v) in &palette.extra {
                    merged.insert(k.clone(), Style::from(v.clone()));
                }
                merged
            },
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Tabbar {
    pub text: Style,
    pub highlight: Style,
}

impl From<TabbarDef> for Tabbar {
    fn from(def: TabbarDef) -> Self {
        Self { text: Style::from(def.text), highlight: Style::from(def.highlight) }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Popup {
    pub border: Style,
    pub text: Style,
}

impl From<PopupDef> for Popup {
    fn from(def: PopupDef) -> Self {
        Self { border: Style::from(def.border), text: Style::from(def.text) }
    }
}

// ---- Global theme ----

#[derive(Debug, Clone)]
pub(crate) struct Theme {
    pub tabbar: Tabbar,
    pub popup: Popup,
    sections: HashMap<String, ComputedSectionTheme>,
}

impl Theme {
    pub fn section(&self, name: &str) -> &ComputedSectionTheme {
        self.sections
            .get(name)
            .unwrap_or_else(|| panic!("unknown theme section: {name}"))
    }
}

fn make_default_palette() -> SectionPaletteDef {
    let mut extra = HashMap::new();
    extra.insert("node_link".into(), StyleDef { fg: Some(Color::Rgb(100, 180, 150)), bg: None, bold: false });
    extra.insert("node_file".into(), StyleDef { fg: Some(Color::Rgb(220, 220, 220)), bg: None, bold: false });
    extra.insert("node_tcp".into(), StyleDef { fg: Some(Color::Cyan), bg: None, bold: false });
    extra.insert("node_udp".into(), StyleDef { fg: Some(Color::Yellow), bg: None, bold: false });

    SectionPaletteDef {
        border: Some(StyleDef { fg: Some(Color::Rgb(0, 204, 153)), bg: None, bold: false }),
        highlight: Some(StyleDef { fg: None, bg: Some(Color::Rgb(64, 64, 64)), bold: true }),
        text: None,
        secondary: Some(StyleDef { fg: Some(Color::Red), bg: None, bold: false }),
        accent: None,
        extra,
    }
}

const SECTION_NAMES: &[&str] = &[
    "connections", "proxies", "settings", "srvctl", "logs", "status", "file",
];

fn build_default_theme() -> Theme {
    let empty = SectionPaletteDef::default();
    let default = make_default_palette();
    let mut sections = HashMap::new();
    for name in SECTION_NAMES {
        sections.insert(name.to_string(), ComputedSectionTheme::from_palette(&empty, &default));
    }
    Theme {
        tabbar: Tabbar {
            text: Style::new().fg(Color::Rgb(0, 153, 153)),
            highlight: Style::new().fg(Color::Rgb(46, 204, 113)),
        },
        popup: Popup {
            border: Style::new().fg(Color::Rgb(0, 102, 102)),
            text: Style::new().fg(Color::Rgb(46, 204, 113)),
        },
        sections,
    }
}

static GLOBAL_THEME: std::sync::LazyLock<RwLock<Theme>> =
    std::sync::LazyLock::new(|| RwLock::new(build_default_theme()));

impl Theme {
    pub fn get() -> std::sync::RwLockReadGuard<'static, Theme> {
        GLOBAL_THEME.read().unwrap()
    }

    pub fn set(theme: Theme) {
        let mut lock = GLOBAL_THEME.write().unwrap();
        let _ = std::mem::replace(&mut *lock, theme);
    }

    pub fn enable_realtime() {
        // No-op: theme is loaded eagerly via load() in the new design.
        // Kept for backward compatibility with cli.rs call site.
    }
}

impl TryFrom<ThemeFile> for Theme {
    type Error = anyhow::Error;

    fn try_from(file: ThemeFile) -> Result<Self, Self::Error> {
        let default = file.default;
        let mut sections = HashMap::new();

        let table: &[(&str, Option<&SectionPaletteDef>)] = &[
            ("connections", file.connections.as_ref()),
            ("proxies", file.proxies.as_ref()),
            ("settings", file.settings.as_ref()),
            ("srvctl", file.srvctl.as_ref()),
            ("logs", file.logs.as_ref()),
            ("status", file.status.as_ref()),
            ("file", None), // always uses defaults, not user-configurable
        ];

        for (name, palette_opt) in table {
            let empty = SectionPaletteDef::default();
            let palette = palette_opt.unwrap_or(&empty);
            sections.insert(name.to_string(), ComputedSectionTheme::from_palette(palette, &default));
        }

        Ok(Self {
            tabbar: Tabbar::from(file.tabbar),
            popup: Popup::from(file.popup),
            sections,
        })
    }
}

// ---- Loading with old format detection ----

#[cfg(feature = "customized-theme")]
pub(crate) fn load_theme(path: &std::path::Path) -> anyhow::Result<Theme> {
    use crate::config::CoreType;

    let file = std::fs::File::open(path)?;
    let mut value: serde_yml::Mapping = serde_yml::from_reader(file)?;

    // Detect old format legacy keys
    let legacy: &[&str] = &["tab", "bars", "connection_tab", "profile_tab", "browser"];
    let has_legacy = value.keys().any(|k| {
        k.as_str().is_some_and(|s| legacy.contains(&s))
    });
    if has_legacy {
        anyhow::bail!("old theme format detected");
    }

    // Core-specific override merge (same logic as agent.rs)
    let core_type = crate::config::CONFIG.core_type();
    let core_key = match core_type {
        CoreType::Mihomo => "mihomo",
        CoreType::Singbox => "sing-box",
    };
    let other_key = match core_type {
        CoreType::Mihomo => "sing-box",
        CoreType::Singbox => "mihomo",
    };

    if let Some(core_section) = value.remove(core_key) {
        if let serde_yml::Value::Mapping(core_map) = core_section {
            value.remove(other_key);
            for (k, v) in core_map {
                value.insert(k, v);
            }
        }
    } else {
        value.remove("mihomo");
        value.remove("sing-box");
    }

    let theme_file: ThemeFile = serde_yml::from_value(serde_yml::Value::Mapping(value))?;
    Theme::try_from(theme_file)
}

impl Theme {
    pub fn load() {
        #[cfg(feature = "customized-theme")]
        {
            let path = crate::config::theme_path();
            if !path.exists() {
                return;
            }
            match load_theme(&path) {
                Ok(theme) => Self::set(theme),
                Err(err) => {
                    log::warn!("Failed to load theme: {err}");
                    log::warn!("Falling back to default theme");
                }
            }
        }
    }
}

// ---- Tests ----

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_def_default_is_empty() {
        let def = StyleDef::default();
        assert!(def.fg.is_none());
        assert!(def.bg.is_none());
        assert!(!def.bold);
    }

    #[test]
    fn style_def_to_style_converts() {
        let def = StyleDef { fg: Some(Color::Red), bg: Some(Color::Blue), bold: true };
        let style: Style = def.into();
        assert_eq!(style.fg, Some(Color::Red));
        assert_eq!(style.bg, Some(Color::Blue));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn section_palette_resolve_falls_back_to_default() {
        let default = SectionPaletteDef {
            border: Some(StyleDef { fg: Some(Color::Red), bg: None, bold: false }),
            highlight: Some(StyleDef { fg: Some(Color::White), bg: Some(Color::Black), bold: true }),
            text: Some(StyleDef { fg: Some(Color::Green), bg: None, bold: false }),
            secondary: None,
            accent: None,
            extra: HashMap::new(),
        };
        let palette = SectionPaletteDef::default();
        let resolved = ComputedSectionTheme::from_palette(&palette, &default);
        assert_eq!(resolved.border.fg, Some(Color::Red));
        assert_eq!(resolved.highlight.bg, Some(Color::Black));
        assert_eq!(resolved.text.fg, Some(Color::Green));
    }

    #[test]
    fn section_palette_override_replaces_default() {
        let default = SectionPaletteDef {
            border: Some(StyleDef { fg: Some(Color::Red), bg: None, bold: false }),
            ..Default::default()
        };
        let palette = SectionPaletteDef {
            border: Some(StyleDef { fg: Some(Color::Blue), bg: None, bold: true }),
            ..Default::default()
        };
        let resolved = ComputedSectionTheme::from_palette(&palette, &default);
        assert_eq!(resolved.border.fg, Some(Color::Blue));
        assert!(resolved.border.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn extra_fields_merge_inherits_default_extras() {
        let mut default_extras = HashMap::new();
        default_extras.insert("node_link".into(), StyleDef { fg: Some(Color::Cyan), bg: None, bold: false });
        let default = SectionPaletteDef { extra: default_extras, ..Default::default() };

        let mut palette_extras = HashMap::new();
        palette_extras.insert("node_file".into(), StyleDef { fg: Some(Color::Yellow), bg: None, bold: false });
        let palette = SectionPaletteDef { extra: palette_extras, ..Default::default() };

        let resolved = ComputedSectionTheme::from_palette(&palette, &default);
        assert_eq!(resolved.extra.get("node_link").unwrap().fg, Some(Color::Cyan));
        assert_eq!(resolved.extra.get("node_file").unwrap().fg, Some(Color::Yellow));
    }

    #[test]
    fn theme_file_deserializes_minimal_yaml() {
        let yaml = r##"
tabbar:
  text: { fg: "#009999" }
  highlight: { fg: "#2ecc71" }
popup:
  border: { fg: "#006666" }
  text: { fg: "#2ecc71" }
default:
  border: { fg: "#00cc99" }
  highlight: { fg: "#ffffff", bg: "#404040", bold: true }
"##;
        let file: ThemeFile = serde_yml::from_str(yaml).unwrap();
        let theme = Theme::try_from(file).unwrap();
        let conn = theme.section("connections");
        assert_eq!(conn.border.fg, Some(Color::Rgb(0, 204, 153)));
        assert!(conn.highlight.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn theme_file_section_overrides_default() {
        let yaml = r##"
tabbar:
  text: {}
  highlight: {}
popup:
  border: {}
  text: {}
default:
  border: { fg: "#ff0000" }
  highlight: { bg: "#0000ff" }
  text: {}
connections:
  border: { fg: "#00ff00" }
"##;
        let file: ThemeFile = serde_yml::from_str(yaml).unwrap();
        let theme = Theme::try_from(file).unwrap();
        let conn = theme.section("connections");
        assert_eq!(conn.border.fg, Some(Color::Rgb(0, 255, 0)));
        let logs = theme.section("logs");
        assert_eq!(logs.border.fg, Some(Color::Rgb(255, 0, 0)));
    }

    #[test]
    fn theme_file_proxies_extra_fields() {
        let yaml = r##"
tabbar:
  text: {}
  highlight: {}
popup:
  border: {}
  text: {}
default:
  border: {}
  highlight: {}
proxies:
  node_link: { fg: "#64b496" }
  node_file: { fg: "#dcdcdc" }
"##;
        let file: ThemeFile = serde_yml::from_str(yaml).unwrap();
        let theme = Theme::try_from(file).unwrap();
        let proxies = theme.section("proxies");
        assert_eq!(proxies.extra.get("node_link").unwrap().fg, Some(Color::Rgb(100, 180, 150)));
        assert_eq!(proxies.extra.get("node_file").unwrap().fg, Some(Color::Rgb(220, 220, 220)));
    }

    #[test]
    #[should_panic(expected = "unknown theme section")]
    fn unknown_section_panics() {
        let theme = build_default_theme();
        theme.section("nonexistent");
    }

    #[test]
    fn file_section_always_uses_defaults() {
        let yaml = r##"
tabbar:
  text: {}
  highlight: {}
popup:
  border: {}
  text: {}
default:
  border: { fg: "#abcdef" }
  highlight: { bg: "#123456" }
  text: {}
"##;
        let file: ThemeFile = serde_yml::from_str(yaml).unwrap();
        let theme = Theme::try_from(file).unwrap();
        let file_sec = theme.section("file");
        assert_eq!(file_sec.border.fg, Some(Color::Rgb(0xab, 0xcd, 0xef)));
        assert_eq!(file_sec.highlight.bg, Some(Color::Rgb(0x12, 0x34, 0x56)));
    }

    #[test]
    fn old_format_detected_via_tab_key() {
        let yaml = r##"tab:
  tab_focused: { fg: "#ff0000" }
"##;
        let mapping: serde_yml::Mapping = serde_yml::from_str(yaml).unwrap();
        let legacy: &[&str] = &["tab", "bars", "connection_tab", "profile_tab", "browser"];
        let has_legacy = mapping.keys().any(|k| k.as_str().is_some_and(|s| legacy.contains(&s)));
        assert!(has_legacy);
    }

    #[test]
    fn new_format_no_legacy_keys() {
        let yaml = "tabbar:\n  text: {}\n  highlight: {}\ndefault:\n  border: {}\n";
        let mapping: serde_yml::Mapping = serde_yml::from_str(yaml).unwrap();
        let legacy: &[&str] = &["tab", "bars", "connection_tab", "profile_tab", "browser"];
        let has_legacy = mapping.keys().any(|k| k.as_str().is_some_and(|s| legacy.contains(&s)));
        assert!(!has_legacy);
    }
}

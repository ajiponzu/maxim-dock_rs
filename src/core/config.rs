use super::{DockEdge, DockItem, TargetKind, Timing, valid_url};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, time::Duration};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub dock: DockSettings,
    pub appearance: Appearance,
    pub items: Vec<DockItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DockSettings {
    pub edge: DockEdge,
    pub icon_size: f32,
    pub spacing: f32,
    pub cursor_poll_interval_ms: u64,
    pub hot_zone_px: i32,
    pub hide_delay_ms: u64,
    pub reveal_hold_ms: u64,
    pub always_on_top: bool,
    pub auto_hide: bool,
}

impl Default for DockSettings {
    fn default() -> Self {
        Self {
            edge: DockEdge::Top,
            icon_size: 56.0,
            spacing: 10.0,
            cursor_poll_interval_ms: 75,
            hot_zone_px: 2,
            hide_delay_ms: 500,
            reveal_hold_ms: 200,
            always_on_top: true,
            auto_hide: true,
        }
    }
}

impl DockSettings {
    pub fn timing(&self) -> Timing {
        Timing {
            cursor_poll_interval: Duration::from_millis(self.cursor_poll_interval_ms),
            hide_delay: Duration::from_millis(self.hide_delay_ms),
            reveal_hold: Duration::from_millis(self.reveal_hold_ms),
            hot_zone_px: self.hot_zone_px,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Appearance {
    pub theme: String,
    pub background_opacity: f32,
    pub palette: String,
    pub colors: Box<CustomColors>,
}
impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            background_opacity: 0.86,
            palette: "default".into(),
            colors: Box::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CustomColors {
    pub background: String,
    pub panel: String,
    pub field: String,
    pub border: String,
    pub text: String,
    pub muted: String,
    pub accent: String,
    pub on_accent: String,
    pub hover: String,
}
impl Default for CustomColors {
    fn default() -> Self {
        Self {
            background: "#0F141D".into(),
            panel: "#18202D".into(),
            field: "#212C3C".into(),
            border: "#344256".into(),
            text: "#EDF2FA".into(),
            muted: "#9BACC4".into(),
            accent: "#5E9DFF".into(),
            on_accent: "#081830".into(),
            hover: "#284061".into(),
        }
    }
}
impl CustomColors {
    pub fn values(&self) -> [&str; 9] {
        [
            &self.background,
            &self.panel,
            &self.field,
            &self.border,
            &self.text,
            &self.muted,
            &self.accent,
            &self.on_accent,
            &self.hover,
        ]
    }
}
pub fn parse_hex_color(value: &str) -> Option<[u8; 3]> {
    let hex = value.strip_prefix('#')?;
    if hex.len() != 6 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    Some([
        u8::from_str_radix(&hex[0..2], 16).ok()?,
        u8::from_str_radix(&hex[2..4], 16).ok()?,
        u8::from_str_radix(&hex[4..6], 16).ok()?,
    ])
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ValidationError(pub &'static str);

impl Config {
    pub fn defaults(home: String) -> Self {
        Self {
            version: 1,
            dock: DockSettings::default(),
            appearance: Appearance::default(),
            items: vec![
                DockItem::new("Explorer", "C:\\Windows\\explorer.exe", TargetKind::Path),
                DockItem::new("Home", home, TargetKind::Path),
                DockItem::new("GitHub", "https://github.com/", TargetKind::Url),
            ],
        }
    }

    pub fn validate(&self) -> Result<(), ValidationError> {
        let fail = |s| Err(ValidationError(s));
        if self.version != 1 {
            return fail("Unsupported config version. Keep the original file and use version 1.");
        }
        self.dock.timing().validate().map_err(ValidationError)?;
        if !self.dock.icon_size.is_finite()
            || !(24.0..=128.0).contains(&self.dock.icon_size)
            || !self.dock.spacing.is_finite()
            || !(0.0..=32.0).contains(&self.dock.spacing)
            || !self.appearance.background_opacity.is_finite()
            || !(0.2..=1.0).contains(&self.appearance.background_opacity)
            || !["system", "dark", "light"].contains(&self.appearance.theme.as_str())
        {
            return fail("Display settings are outside supported bounds.");
        }
        if self.items.len() > 256 {
            return fail("At most 256 Dock items are supported.");
        }
        if !["default", "ocean", "forest", "rose", "custom"]
            .contains(&self.appearance.palette.as_str())
        {
            return fail("Unknown color palette.");
        }
        if self
            .appearance
            .colors
            .values()
            .iter()
            .any(|value| parse_hex_color(value).is_none())
        {
            return fail("Theme colors must use #RRGGBB, for example #5E9DFF.");
        }
        let mut ids = HashSet::new();
        let mut targets = HashSet::new();
        for item in &self.items {
            if item.id.is_nil() || !ids.insert(item.id) {
                return fail("Item IDs must be unique and nonzero.");
            }
            if item.label.trim().is_empty()
                || item.label.chars().count() > 80
                || item.label.chars().any(char::is_control)
            {
                return fail("Item labels must contain 1 to 80 printable characters.");
            }
            if item.target.trim().is_empty()
                || item.target.len() > 32768
                || item.target.chars().any(char::is_control)
            {
                return fail("An item has an empty or invalid target.");
            }
            // Missing paths are allowed in saved configs: unplugged drives must not invalidate all settings.
            if item.kind == TargetKind::Url && !valid_url(&item.target) {
                return fail("An item URL must be valid http:// or https:// without credentials.");
            }
            if !targets.insert(target_key(&item.target, item.kind)) {
                return fail("Duplicate target. Remove the duplicate item before saving.");
            }
        }
        Ok(())
    }

    pub fn add_item(&mut self, item: DockItem) -> Result<(), ValidationError> {
        if self.items.iter().any(|existing| {
            target_key(&existing.target, existing.kind) == target_key(&item.target, item.kind)
        }) {
            return Err(ValidationError("Target is already registered."));
        }
        self.items.push(item);
        if let Err(e) = self.validate() {
            self.items.pop();
            return Err(e);
        }
        Ok(())
    }

    pub fn move_item(&mut self, index: usize, to: usize) -> bool {
        if index >= self.items.len() || to >= self.items.len() || index == to {
            return false;
        }
        let item = self.items.remove(index);
        self.items.insert(to, item);
        true
    }
}

pub fn target_key(target: &str, kind: TargetKind) -> String {
    match kind {
        TargetKind::Path => format!(
            "path:{}",
            target
                .replace('/', "\\")
                .trim_end_matches('\\')
                .to_lowercase()
        ),
        TargetKind::Url => format!(
            "url:{}",
            url::Url::parse(target)
                .map(|u| u.to_string())
                .unwrap_or_else(|_| target.to_owned())
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn palettes_round_trip_and_legacy_configs_default_without_losing_items() {
        let mut config = Config::defaults("missing-home".into());
        let mut legacy = toml::Value::try_from(&config).unwrap();
        let appearance = legacy
            .get_mut("appearance")
            .unwrap()
            .as_table_mut()
            .unwrap();
        appearance.remove("palette");
        appearance.remove("colors");
        let restored: Config = toml::from_str(&toml::to_string(&legacy).unwrap()).unwrap();
        assert_eq!(restored, config);
        for palette in ["default", "ocean", "forest", "rose", "custom"] {
            config.appearance.palette = palette.into();
            config.appearance.colors.accent = "#abcdef".into();
            config.validate().unwrap();
            let restored: Config = toml::from_str(&toml::to_string(&config).unwrap()).unwrap();
            assert_eq!(restored, config);
        }
        config.appearance.palette = "unknown".into();
        assert!(config.validate().is_err());
        config.appearance.palette = "custom".into();
        for bad in ["red", "#abc", "#12345678", "#GGFFFF", "#あ123", " #abcdef"] {
            config.appearance.colors.panel = bad.into();
            assert!(config.validate().is_err());
        }
        assert_eq!(parse_hex_color("#abcdef"), Some([171, 205, 239]));
    }

    #[test]
    fn default_top_and_toml_round_trip_preserve_ids_order_and_edges() {
        assert_eq!(DockEdge::default(), DockEdge::Top);
        let mut c = Config::defaults("C:\\Users\\test".into());
        for edge in [
            DockEdge::Top,
            DockEdge::Bottom,
            DockEdge::Left,
            DockEdge::Right,
        ] {
            c.dock.edge = edge;
            let text = toml::to_string_pretty(&c).unwrap();
            let restored: Config = toml::from_str(&text).unwrap();
            assert_eq!(c, restored);
            restored.validate().unwrap();
        }
    }

    #[test]
    fn validation_rejects_unsafe_values_and_preserves_missing_paths() {
        let c = Config::defaults("missing-path".into());
        c.validate().unwrap();
        for change in [0, 1, 2, 3, 4, 5] {
            let mut bad = c.clone();
            match change {
                0 => bad.version = 2,
                1 => bad.dock.icon_size = f32::NAN,
                2 => bad.dock.hide_delay_ms = 0,
                3 => bad.items[0].label.clear(),
                4 => bad.items[2].target = "https://:".into(),
                _ => bad.items[1].id = bad.items[0].id,
            }
            assert!(bad.validate().is_err());
        }
    }

    #[test]
    fn duplicate_targets_and_reorder_preserve_stable_ids() {
        let mut c = Config::defaults("C:\\Users\\test".into());
        assert!(
            c.add_item(DockItem::new(
                "duplicate",
                "c:/WINDOWS/explorer.exe",
                TargetKind::Path
            ))
            .is_err()
        );
        assert!(
            c.add_item(DockItem::new(
                "duplicate",
                "https://GITHUB.com:443/",
                TargetKind::Url
            ))
            .is_err()
        );
        let id = c.items[0].id;
        assert!(c.move_item(0, 2));
        assert_eq!(c.items[2].id, id);
        assert!(!c.move_item(3, 0));
        c.items[2].label = "Renamed".into();
        assert_eq!(c.items[2].id, id);
        c.items.remove(2);
        assert_eq!(c.items.len(), 2);
        c.validate().unwrap();
    }
}

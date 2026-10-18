//! Persistent configuration: click settings, profiles and app preferences.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::i18n::Lang;
use crate::keys::{self, KeyCombo};
use crate::platform::{MouseButton, ScreenRect};

pub const MIN_CPS: f64 = 0.01;
pub const MAX_CPS: f64 = 1000.0;
pub const MAX_HOLD_MS: u32 = 10_000;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RateMode {
    #[default]
    Cps,
    Interval,
}

/// Interval split into the fields shown in the UI.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Interval {
    pub hours: u32,
    pub minutes: u32,
    pub seconds: u32,
    pub millis: u32,
}

impl Default for Interval {
    fn default() -> Self {
        Self { hours: 0, minutes: 0, seconds: 0, millis: 100 }
    }
}

impl Interval {
    pub fn total_ms(&self) -> u64 {
        self.hours as u64 * 3_600_000 + self.minutes as u64 * 60_000 + self.seconds as u64 * 1000 + self.millis as u64
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionKind {
    #[default]
    Mouse,
    Keyboard,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClickKind {
    #[default]
    Single,
    Double,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationMode {
    /// Wherever the cursor currently is.
    #[default]
    Cursor,
    /// One fixed point.
    Point,
    /// A random point inside a rectangle on every click.
    Area,
    /// A list of points clicked one after another.
    Sequence,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SeqOrder {
    #[default]
    InOrder,
    Random,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LimitMode {
    #[default]
    Infinite,
    Count,
    Time,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SeqPoint {
    pub x: i32,
    pub y: i32,
    pub button: MouseButton,
    pub click: ClickKind,
    /// Extra pause after this point, added on top of the main interval.
    pub pause_ms: u32,
}

impl Default for SeqPoint {
    fn default() -> Self {
        Self { x: 0, y: 0, button: MouseButton::Left, click: ClickKind::Single, pause_ms: 0 }
    }
}

/// Everything that describes *how* to click. One instance per profile.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub rate_mode: RateMode,
    pub cps: f64,
    pub interval: Interval,
    /// Random deviation of every interval, `0.0..=0.5` (fraction of the interval).
    pub jitter: f32,

    pub action: ActionKind,
    pub button: MouseButton,
    pub click: ClickKind,
    pub key: KeyCombo,
    /// How long the button / key is held down.
    pub hold_ms: u32,

    pub location: LocationMode,
    pub point: (i32, i32),
    pub area: ScreenRect,
    pub sequence: Vec<SeqPoint>,
    pub seq_order: SeqOrder,
    /// Put the cursor back where it was after every click.
    pub return_cursor: bool,

    pub limit: LimitMode,
    pub limit_count: u64,
    pub limit_secs: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rate_mode: RateMode::Cps,
            cps: 10.0,
            interval: Interval::default(),
            jitter: 0.0,
            action: ActionKind::Mouse,
            button: MouseButton::Left,
            click: ClickKind::Single,
            key: KeyCombo::key(keys::VK_SPACE),
            hold_ms: 0,
            location: LocationMode::Cursor,
            point: (500, 500),
            area: ScreenRect { x: 400, y: 300, w: 300, h: 200 },
            sequence: Vec::new(),
            seq_order: SeqOrder::InOrder,
            return_cursor: false,
            limit: LimitMode::Infinite,
            limit_count: 100,
            limit_secs: 60,
        }
    }
}

/// Reasons why a run can't be started with the current settings.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    NoKey,
    KeyIsHotkey,
    NoPoints,
    EmptyArea,
}

impl Settings {
    /// Base interval between clicks in milliseconds.
    pub fn interval_ms(&self) -> f64 {
        match self.rate_mode {
            RateMode::Cps => 1000.0 / self.cps.clamp(MIN_CPS, MAX_CPS),
            RateMode::Interval => self.interval.total_ms().max(1) as f64,
        }
    }

    pub fn effective_cps(&self) -> f64 {
        1000.0 / self.interval_ms()
    }

    /// A limit "unit" is a whole cycle in sequence mode and one click otherwise.
    pub fn counts_cycles(&self) -> bool {
        self.action == ActionKind::Mouse
            && self.location == LocationMode::Sequence
            && self.seq_order == SeqOrder::InOrder
    }

    pub fn problem(&self, hotkeys: &HotkeyConfig) -> Option<Problem> {
        match self.action {
            ActionKind::Keyboard => {
                if !self.key.is_set() {
                    return Some(Problem::NoKey);
                }
                if [hotkeys.toggle.vk, hotkeys.add_point.vk].contains(&self.key.vk) {
                    return Some(Problem::KeyIsHotkey);
                }
            }
            ActionKind::Mouse => match self.location {
                LocationMode::Sequence if self.sequence.is_empty() => return Some(Problem::NoPoints),
                LocationMode::Area if self.area.w <= 0 || self.area.h <= 0 => return Some(Problem::EmptyArea),
                _ => {}
            },
        }
        None
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub name: String,
    pub settings: Settings,
}

impl Default for Profile {
    fn default() -> Self {
        Self { name: "Default".into(), settings: Settings::default() }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HotkeyMode {
    /// Press once to start, press again to stop.
    #[default]
    Toggle,
    /// Click only while the key is held down.
    Hold,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct HotkeyConfig {
    pub toggle: KeyCombo,
    pub mode: HotkeyMode,
    pub add_point: KeyCombo,
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self { toggle: KeyCombo::key(keys::VK_F6), mode: HotkeyMode::Toggle, add_point: KeyCombo::key(keys::VK_F7) }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeChoice {
    #[default]
    Dark,
    Light,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tab {
    #[default]
    Click,
    Location,
    Settings,
}

/// The whole settings file.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    pub language: Lang,
    pub theme: ThemeChoice,
    pub always_on_top: bool,
    pub compact: bool,
    pub tab: Tab,
    pub hotkeys: HotkeyConfig,
    /// Seconds to wait after "start" before the first click.
    pub start_delay_s: u32,
    /// Stop when the user moves the mouse away from where the clicker put it.
    pub failsafe: bool,
    pub profiles: Vec<Profile>,
    pub active_profile: usize,
    pub total_clicks: u64,
    /// Outer window position in physical pixels.
    pub window_pos: Option<(i32, i32)>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: if crate::platform::prefers_russian() { Lang::Ru } else { Lang::En },
            theme: ThemeChoice::Dark,
            always_on_top: false,
            compact: false,
            tab: Tab::Click,
            hotkeys: HotkeyConfig::default(),
            start_delay_s: 0,
            failsafe: true,
            profiles: vec![Profile::default()],
            active_profile: 0,
            total_clicks: 0,
            window_pos: None,
        }
    }
}

const FILE_NAME: &str = "clkr.json";

impl AppConfig {
    /// Where the config lives: next to the executable if a `clkr.json` already
    /// exists there ("portable mode"), otherwise in the user's config directory.
    pub fn path() -> PathBuf {
        if let Some(portable) = std::env::current_exe().ok().and_then(|exe| Some(exe.parent()?.join(FILE_NAME)))
            && portable.is_file()
        {
            return portable;
        }
        config_dir().join("clkr").join(FILE_NAME)
    }

    pub fn load(path: &Path) -> Self {
        let mut cfg: AppConfig =
            std::fs::read_to_string(path).ok().and_then(|text| serde_json::from_str(&text).ok()).unwrap_or_default();
        cfg.sanitize();
        cfg
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let text = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        // Write to a temp file first so a crash never leaves a truncated config.
        let tmp = path.with_extension("json.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, path)
    }

    fn sanitize(&mut self) {
        if self.profiles.is_empty() {
            self.profiles.push(Profile::default());
        }
        self.active_profile = self.active_profile.min(self.profiles.len() - 1);
        for p in &mut self.profiles {
            let s = &mut p.settings;
            s.cps = s.cps.clamp(MIN_CPS, MAX_CPS);
            s.jitter = s.jitter.clamp(0.0, 0.5);
            s.hold_ms = s.hold_ms.min(MAX_HOLD_MS);
        }
    }

    pub fn settings(&self) -> &Settings {
        &self.profiles[self.active_profile].settings
    }

    pub fn settings_mut(&mut self) -> &mut Settings {
        &mut self.profiles[self.active_profile].settings
    }
}

fn config_dir() -> PathBuf {
    #[cfg(windows)]
    let base = std::env::var_os("APPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")));
    base.unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_from_cps_and_fields() {
        let mut s = Settings { cps: 20.0, ..Default::default() };
        assert_eq!(s.interval_ms(), 50.0);
        s.cps = 5000.0; // clamped to MAX_CPS
        assert_eq!(s.interval_ms(), 1.0);
        s.rate_mode = RateMode::Interval;
        s.interval = Interval { hours: 0, minutes: 1, seconds: 2, millis: 500 };
        assert_eq!(s.interval_ms(), 62_500.0);
        s.interval = Interval { hours: 0, minutes: 0, seconds: 0, millis: 0 };
        assert_eq!(s.interval_ms(), 1.0, "zero interval is bumped to 1 ms");
    }

    #[test]
    fn problems_are_detected() {
        let hk = HotkeyConfig::default();
        let mut s = Settings::default();
        assert_eq!(s.problem(&hk), None);

        s.location = LocationMode::Sequence;
        assert_eq!(s.problem(&hk), Some(Problem::NoPoints));
        s.sequence.push(SeqPoint::default());
        assert_eq!(s.problem(&hk), None);

        s.location = LocationMode::Area;
        s.area.w = 0;
        assert_eq!(s.problem(&hk), Some(Problem::EmptyArea));

        s.action = ActionKind::Keyboard;
        s.key = KeyCombo::default();
        assert_eq!(s.problem(&hk), Some(Problem::NoKey));
        s.key = hk.toggle;
        assert_eq!(s.problem(&hk), Some(Problem::KeyIsHotkey));
    }

    #[test]
    fn cycles_only_for_ordered_routes() {
        let mut s = Settings { location: LocationMode::Sequence, ..Default::default() };
        assert!(s.counts_cycles());
        s.seq_order = SeqOrder::Random;
        assert!(!s.counts_cycles());
        s.seq_order = SeqOrder::InOrder;
        s.action = ActionKind::Keyboard;
        assert!(!s.counts_cycles());
    }

    #[test]
    fn config_roundtrip_and_sanitize() {
        let dir = std::env::temp_dir().join(format!("clkr-test-{}", std::process::id()));
        let path = dir.join("clkr.json");
        let mut cfg = AppConfig::default();
        cfg.profiles.push(Profile { name: "Second".into(), settings: Settings { cps: 42.0, ..Default::default() } });
        cfg.active_profile = 1;
        cfg.save(&path).unwrap();
        assert_eq!(AppConfig::load(&path), cfg);

        // Broken values are repaired on load; unknown/missing fields fall back to defaults.
        std::fs::write(&path, r#"{"profiles":[],"active_profile":7,"bogus":1}"#).unwrap();
        let fixed = AppConfig::load(&path);
        assert_eq!(fixed.profiles.len(), 1);
        assert_eq!(fixed.active_profile, 0);
        std::fs::write(&path, "not json").unwrap();
        assert_eq!(AppConfig::load(&path).profiles.len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }
}

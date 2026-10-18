//! Keyboard / mouse-button identifiers.
//!
//! Keys are stored as Windows virtual-key codes on every platform: they are only
//! *interpreted* by the Windows backend, but keeping one representation makes the
//! settings file portable and the UI code platform independent.

use serde::{Deserialize, Serialize};

pub const VK_MBUTTON: u16 = 0x04;
pub const VK_XBUTTON1: u16 = 0x05;
pub const VK_XBUTTON2: u16 = 0x06;
pub const VK_BACK: u16 = 0x08;
pub const VK_TAB: u16 = 0x09;
pub const VK_RETURN: u16 = 0x0D;
pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_MENU: u16 = 0x12;
pub const VK_PAUSE: u16 = 0x13;
pub const VK_CAPITAL: u16 = 0x14;
pub const VK_ESCAPE: u16 = 0x1B;
pub const VK_SPACE: u16 = 0x20;
pub const VK_PRIOR: u16 = 0x21;
pub const VK_NEXT: u16 = 0x22;
pub const VK_END: u16 = 0x23;
pub const VK_HOME: u16 = 0x24;
pub const VK_LEFT: u16 = 0x25;
pub const VK_UP: u16 = 0x26;
pub const VK_RIGHT: u16 = 0x27;
pub const VK_DOWN: u16 = 0x28;
pub const VK_SNAPSHOT: u16 = 0x2C;
pub const VK_INSERT: u16 = 0x2D;
pub const VK_DELETE: u16 = 0x2E;
pub const VK_LWIN: u16 = 0x5B;
pub const VK_RWIN: u16 = 0x5C;
pub const VK_APPS: u16 = 0x5D;
pub const VK_NUMPAD0: u16 = 0x60;
pub const VK_MULTIPLY: u16 = 0x6A;
pub const VK_ADD: u16 = 0x6B;
pub const VK_SUBTRACT: u16 = 0x6D;
pub const VK_DECIMAL: u16 = 0x6E;
pub const VK_DIVIDE: u16 = 0x6F;
pub const VK_F1: u16 = 0x70;
pub const VK_F6: u16 = 0x75;
pub const VK_F7: u16 = 0x76;
pub const VK_F24: u16 = 0x87;
pub const VK_NUMLOCK: u16 = 0x90;
pub const VK_SCROLL: u16 = 0x91;
pub const VK_LSHIFT: u16 = 0xA0;
pub const VK_RMENU: u16 = 0xA5;

/// Keys that are only ever used as modifiers and therefore can't be the main
/// key of a combination.
pub fn is_modifier(vk: u16) -> bool {
    matches!(vk, VK_SHIFT | VK_CONTROL | VK_MENU | VK_LWIN | VK_RWIN) || (VK_LSHIFT..=VK_RMENU).contains(&vk)
}

/// Extended keys need `KEYEVENTF_EXTENDEDKEY` when synthesised via scan codes.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn is_extended(vk: u16) -> bool {
    matches!(
        vk,
        VK_PRIOR
            | VK_NEXT
            | VK_END
            | VK_HOME
            | VK_LEFT
            | VK_UP
            | VK_RIGHT
            | VK_DOWN
            | VK_INSERT
            | VK_DELETE
            | VK_DIVIDE
            | VK_NUMLOCK
            | VK_SNAPSHOT
            | VK_LWIN
            | VK_RWIN
            | VK_APPS
    )
}

/// A key plus optional modifiers. `vk == 0` means "not set".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyCombo {
    pub vk: u16,
    pub ctrl: bool,
    pub shift: bool,
    pub alt: bool,
}

impl KeyCombo {
    pub const fn key(vk: u16) -> Self {
        Self { vk, ctrl: false, shift: false, alt: false }
    }

    pub fn is_set(&self) -> bool {
        self.vk != 0
    }

    /// Human readable label, e.g. `Ctrl + Shift + F6`.
    pub fn label(&self) -> String {
        if !self.is_set() {
            return "—".to_owned();
        }
        let mut parts: Vec<String> = Vec::with_capacity(4);
        if self.ctrl {
            parts.push("Ctrl".into());
        }
        if self.shift {
            parts.push("Shift".into());
        }
        if self.alt {
            parts.push("Alt".into());
        }
        parts.push(key_name(self.vk));
        parts.join(" + ")
    }
}

/// Short English name of a virtual key.
pub fn key_name(vk: u16) -> String {
    let fixed = match vk {
        VK_MBUTTON => "Mouse 3",
        VK_XBUTTON1 => "Mouse 4",
        VK_XBUTTON2 => "Mouse 5",
        VK_BACK => "Backspace",
        VK_TAB => "Tab",
        VK_RETURN => "Enter",
        VK_PAUSE => "Pause",
        VK_CAPITAL => "Caps Lock",
        VK_ESCAPE => "Esc",
        VK_SPACE => "Space",
        VK_PRIOR => "Page Up",
        VK_NEXT => "Page Down",
        VK_END => "End",
        VK_HOME => "Home",
        VK_LEFT => "Left",
        VK_UP => "Up",
        VK_RIGHT => "Right",
        VK_DOWN => "Down",
        VK_SNAPSHOT => "Print Screen",
        VK_INSERT => "Insert",
        VK_DELETE => "Delete",
        VK_APPS => "Menu",
        VK_MULTIPLY => "Num *",
        VK_ADD => "Num +",
        VK_SUBTRACT => "Num -",
        VK_DECIMAL => "Num .",
        VK_DIVIDE => "Num /",
        VK_NUMLOCK => "Num Lock",
        VK_SCROLL => "Scroll Lock",
        _ => "",
    };
    if !fixed.is_empty() {
        return fixed.to_owned();
    }
    match vk {
        0x30..=0x39 | 0x41..=0x5A => char::from(vk as u8).to_string(),
        VK_NUMPAD0..=0x69 => format!("Num {}", vk - VK_NUMPAD0),
        VK_F1..=VK_F24 => format!("F{}", vk - VK_F1 + 1),
        _ => crate::platform::os_key_name(vk).unwrap_or_else(|| format!("Key 0x{vk:02X}")),
    }
}

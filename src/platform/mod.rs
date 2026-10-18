//! Thin OS layer. The real implementation is Windows-only; on other systems an
//! inert stub is used so the UI can still be built and previewed there.

use serde::{Deserialize, Serialize};

#[cfg(windows)]
mod picker_win;
#[cfg(windows)]
mod win;
#[cfg(windows)]
pub use win::*;

#[cfg(not(windows))]
mod stub;
#[cfg(not(windows))]
pub use stub::*;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum MouseButton {
    #[default]
    Left,
    Middle,
    Right,
}

/// Rectangle in physical screen pixels (virtual desktop coordinates).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

#[cfg_attr(not(windows), allow(dead_code))]
impl ScreenRect {
    pub fn from_corners(a: (i32, i32), b: (i32, i32)) -> Self {
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        Self { x: x0, y: y0, w: x1 - x0, h: y1 - y0 }
    }

    pub fn contains(&self, p: (i32, i32)) -> bool {
        p.0 >= self.x && p.1 >= self.y && p.0 < self.x + self.w && p.1 < self.y + self.h
    }
}

/// Font files loaded from the OS at startup (nothing is bundled but icons).
pub struct SystemFonts {
    pub regular: Vec<u8>,
    pub semibold: Vec<u8>,
    pub fallbacks: Vec<Vec<u8>>,
}

/// What the on-screen picker should collect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PickKind {
    Point,
    Area,
    /// Several points in a row (for the click route).
    Points,
}

#[derive(Clone, Debug)]
#[cfg_attr(not(windows), allow(dead_code))]
pub enum PickResult {
    Point(i32, i32),
    Area(ScreenRect),
    Points(Vec<(i32, i32)>),
    Cancelled,
}

#[derive(Clone, Debug)]
#[cfg_attr(not(windows), allow(dead_code))]
pub struct PickRequest {
    pub kind: PickKind,
    /// Short instruction shown at the top of the overlay.
    pub hint: String,
    /// Already existing route points, drawn for reference.
    pub existing: Vec<(i32, i32)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_from_any_corners() {
        let r = ScreenRect::from_corners((300, 50), (-100, 250));
        assert_eq!(r, ScreenRect { x: -100, y: 50, w: 400, h: 200 });
        assert!(r.contains((-100, 50)));
        assert!(!r.contains((300, 250)));
    }
}

//! Full-screen picker overlay.
//!
//! Shows a frozen, dimmed screenshot of the whole virtual desktop with a pixel
//! loupe and live coordinates, and lets the user click a point, drag an area or
//! click several points in a row. Plain GDI on a dedicated thread, everything in
//! physical pixels so it is exact on mixed-DPI multi-monitor setups.

use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{DWMWA_TRANSITIONS_FORCEDISABLED, DwmFlush, DwmSetWindowAttribute};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BeginPaint, BitBlt, CAPTUREBLT, COLORONCOLOR, CreateCompatibleDC,
    CreateDIBSection, CreateFontW, CreatePen, CreateRectRgn, CreateSolidBrush, DIB_RGB_COLORS, DeleteDC, DeleteObject,
    Ellipse, EndPaint, GdiFlush, GetDC, GetMonitorInfoW, GetStockObject, GetTextExtentPoint32W, GetUpdateRgn, HBITMAP,
    HDC, HFONT, HGDIOBJ, InvalidateRect, LineTo, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromPoint, MoveToEx,
    NULL_BRUSH, PAINTSTRUCT, PS_SOLID, Rectangle, ReleaseDC, RoundRect, SRCCOPY, SelectClipRgn, SelectObject,
    SetBkMode, SetStretchBltMode, SetTextColor, StretchBlt, TRANSPARENT, TextOutW,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Threading::Sleep;
use windows_sys::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, GetDpiForSystem, SetThreadDpiAwarenessContext,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{ReleaseCapture, SetCapture, SetFocus};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA, GetMessageW,
    GetWindowLongPtrW, IDC_CROSS, LoadCursorW, MSG, PostQuitMessage, RegisterClassExW, SW_SHOW, SetCursor,
    SetForegroundWindow, SetWindowLongPtrW, ShowWindow, TranslateMessage, WM_DESTROY, WM_ERASEBKGND, WM_KEYDOWN,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_NCCREATE, WM_PAINT, WM_RBUTTONDOWN, WM_SETCURSOR, WNDCLASSEXW,
    WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use super::win::{cursor_pos, hide_main_window, main_hwnd, set_cursor_pos, show_main_window, virtual_screen};
use super::{PickKind, PickRequest, PickResult, ScreenRect};

const fn rgb(r: u8, g: u8, b: u8) -> u32 {
    r as u32 | (g as u32) << 8 | (b as u32) << 16
}

const ACCENT: u32 = rgb(124, 108, 255);
const PANEL: u32 = rgb(22, 25, 34);
const PANEL_BORDER: u32 = rgb(52, 58, 76);
const TEXT: u32 = rgb(236, 238, 244);
const OLD_POINT: u32 = rgb(104, 110, 130);
const WHITE: u32 = rgb(255, 255, 255);
const BLACK: u32 = rgb(0, 0, 0);

/// Pixels sampled by the loupe in each direction.
const LOUPE_RADIUS: i32 = 7;

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().collect()
}

fn rect(l: i32, t: i32, r: i32, b: i32) -> RECT {
    RECT { left: l, top: t, right: r, bottom: b }
}

const EMPTY: RECT = RECT { left: 0, top: 0, right: 0, bottom: 0 };

/// A 32-bit top-down DIB selected into its own memory DC.
struct Dib {
    dc: HDC,
    bmp: HBITMAP,
    old: HGDIOBJ,
    bits: *mut u32,
    len: usize,
}

impl Dib {
    unsafe fn new(w: i32, h: i32) -> Option<Self> {
        unsafe {
            let screen = GetDC(null_mut());
            let dc = CreateCompatibleDC(screen);
            ReleaseDC(null_mut(), screen);
            let mut bmi: BITMAPINFO = zeroed();
            bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
            bmi.bmiHeader.biWidth = w;
            bmi.bmiHeader.biHeight = -h;
            bmi.bmiHeader.biPlanes = 1;
            bmi.bmiHeader.biBitCount = 32;
            bmi.bmiHeader.biCompression = BI_RGB;
            let mut bits = null_mut();
            let bmp = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
            if bmp.is_null() || bits.is_null() {
                DeleteDC(dc);
                return None;
            }
            let old = SelectObject(dc, bmp);
            Some(Self { dc, bmp, old, bits: bits.cast(), len: (w as usize) * (h as usize) })
        }
    }

    unsafe fn pixels(&mut self) -> &mut [u32] {
        unsafe {
            GdiFlush();
            std::slice::from_raw_parts_mut(self.bits, self.len)
        }
    }
}

impl Drop for Dib {
    fn drop(&mut self) {
        unsafe {
            SelectObject(self.dc, self.old);
            DeleteObject(self.bmp);
            DeleteDC(self.dc);
        }
    }
}

struct Overlay {
    kind: PickKind,
    hint: Vec<u16>,
    origin: (i32, i32),
    size: (i32, i32),
    bright: Dib,
    dim: Dib,
    back: Dib,
    scale: f32,
    font: HFONT,
    font_bold: HFONT,
    /// Cursor in client coordinates.
    cursor: (i32, i32),
    drag_from: Option<(i32, i32)>,
    existing: Vec<(i32, i32)>,
    added: Vec<(i32, i32)>,
    result: Option<PickResult>,
    /// Last painted bounds of the moving parts: loupe, selection, hint panel.
    parts: [RECT; 3],
}

/// Runs the picker to completion. Blocks the calling thread.
pub fn run(req: PickRequest) -> PickResult {
    unsafe {
        SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);

        // Hide clkr itself so it isn't part of the screenshot.
        let main = main_hwnd();
        if !main.is_null() {
            let off: i32 = 1;
            DwmSetWindowAttribute(main, DWMWA_TRANSITIONS_FORCEDISABLED as u32, &off as *const i32 as *const _, 4);
            hide_main_window();
            Sleep(60);
            DwmFlush();
        }

        let result = capture_and_pick(req).unwrap_or(PickResult::Cancelled);

        if !main.is_null() {
            show_main_window();
            let on: i32 = 0;
            DwmSetWindowAttribute(main, DWMWA_TRANSITIONS_FORCEDISABLED as u32, &on as *const i32 as *const _, 4);
        }
        result
    }
}

unsafe fn capture_and_pick(req: PickRequest) -> Option<PickResult> {
    unsafe {
        let vs = virtual_screen();
        let (w, h) = (vs.w, vs.h);

        let mut bright = Dib::new(w, h)?;
        let screen = GetDC(null_mut());
        BitBlt(bright.dc, 0, 0, w, h, screen, vs.x, vs.y, SRCCOPY | CAPTUREBLT);
        ReleaseDC(null_mut(), screen);

        // Dimmed, slightly blue-tinted copy used as the backdrop.
        let mut dim = Dib::new(w, h)?;
        let src = bright.pixels().to_vec();
        for (d, s) in dim.pixels().iter_mut().zip(&src) {
            let b = (s & 0xFF) * 112 / 256 + 10;
            let g = ((s >> 8) & 0xFF) * 100 / 256 + 4;
            let r = ((s >> 16) & 0xFF) * 100 / 256 + 4;
            *d = b | g << 8 | r << 16;
        }
        let back = Dib::new(w, h)?;

        let scale = GetDpiForSystem() as f32 / 96.0;
        let face = wide("Segoe UI\0");
        let font_px = |px: f32| -((px * scale).round() as i32);
        let font = CreateFontW(font_px(14.0), 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr());
        let font_bold = CreateFontW(font_px(13.0), 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr());

        let to_client = |(x, y): (i32, i32)| (x - vs.x, y - vs.y);
        let mut overlay = Box::new(Overlay {
            kind: req.kind,
            hint: wide(&req.hint),
            origin: (vs.x, vs.y),
            size: (w, h),
            bright,
            dim,
            back,
            scale,
            font,
            font_bold,
            cursor: to_client(cursor_pos()),
            drag_from: None,
            existing: req.existing.iter().copied().map(to_client).collect(),
            added: Vec::new(),
            result: None,
            parts: [EMPTY; 3],
        });

        let instance = GetModuleHandleW(null());
        let class = wide("clkr-picker\0");
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: instance,
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), IDC_CROSS),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: class.as_ptr(),
            hIconSm: null_mut(),
        };
        // Fails harmlessly with "class already exists" on the second pick.
        RegisterClassExW(&wc);

        let ptr: *mut Overlay = &mut *overlay;
        let hwnd = CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
            class.as_ptr(),
            class.as_ptr(),
            WS_POPUP,
            vs.x,
            vs.y,
            w,
            h,
            null_mut(),
            null_mut(),
            instance,
            ptr as *const _,
        );
        if hwnd.is_null() {
            DeleteObject(font);
            DeleteObject(font_bold);
            return None;
        }
        overlay.parts = overlay.current_parts();
        ShowWindow(hwnd, SW_SHOW);
        SetForegroundWindow(hwnd);
        SetFocus(hwnd);

        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        DeleteObject(font);
        DeleteObject(font_bold);
        overlay.result.take()
    }
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if msg == WM_NCCREATE {
            let cs = &*(lparam as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Overlay;
        if ptr.is_null() {
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let o = &mut *ptr;
        let mouse = || ((lparam & 0xFFFF) as u16 as i16 as i32, ((lparam >> 16) & 0xFFFF) as u16 as i16 as i32);

        match msg {
            WM_SETCURSOR => {
                SetCursor(LoadCursorW(null_mut(), IDC_CROSS));
                1
            }
            WM_ERASEBKGND => 1,
            WM_PAINT => {
                o.on_paint(hwnd);
                0
            }
            WM_MOUSEMOVE => {
                o.cursor = mouse();
                o.refresh(hwnd);
                0
            }
            WM_LBUTTONDOWN => {
                o.cursor = mouse();
                match o.kind {
                    PickKind::Point => o.finish(hwnd, PickResult::Point(o.screen(o.cursor).0, o.screen(o.cursor).1)),
                    PickKind::Area => {
                        o.drag_from = Some(o.cursor);
                        SetCapture(hwnd);
                        o.refresh(hwnd);
                    }
                    PickKind::Points => {
                        o.added.push(o.cursor);
                        InvalidateRect(hwnd, null(), 0);
                    }
                }
                0
            }
            WM_LBUTTONUP => {
                if let Some(from) = o.drag_from.take() {
                    ReleaseCapture();
                    let r = ScreenRect::from_corners(o.screen(from), o.screen(mouse()));
                    if r.w >= 2 && r.h >= 2 {
                        o.finish(hwnd, PickResult::Area(r));
                    } else {
                        o.refresh(hwnd);
                    }
                }
                0
            }
            WM_RBUTTONDOWN => {
                o.finish_or_cancel(hwnd);
                0
            }
            WM_KEYDOWN => {
                o.on_key(hwnd, wparam as u16);
                0
            }
            WM_DESTROY => {
                PostQuitMessage(0);
                0
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

impl Overlay {
    fn screen(&self, (x, y): (i32, i32)) -> (i32, i32) {
        (x + self.origin.0, y + self.origin.1)
    }

    fn px(&self, v: f32) -> i32 {
        (v * self.scale).round() as i32
    }

    unsafe fn finish(&mut self, hwnd: HWND, result: PickResult) {
        self.result = Some(result);
        unsafe { DestroyWindow(hwnd) };
    }

    unsafe fn finish_or_cancel(&mut self, hwnd: HWND) {
        let result = if self.kind == PickKind::Points && !self.added.is_empty() {
            PickResult::Points(self.added.iter().map(|&p| self.screen(p)).collect())
        } else {
            PickResult::Cancelled
        };
        unsafe { self.finish(hwnd, result) };
    }

    unsafe fn on_key(&mut self, hwnd: HWND, vk: u16) {
        use crate::keys::*;
        unsafe {
            match vk {
                VK_ESCAPE => self.finish(hwnd, PickResult::Cancelled),
                VK_RETURN | VK_SPACE => match self.kind {
                    PickKind::Point => {
                        let (x, y) = self.screen(self.cursor);
                        self.finish(hwnd, PickResult::Point(x, y));
                    }
                    PickKind::Points if vk == VK_SPACE => {
                        self.added.push(self.cursor);
                        InvalidateRect(hwnd, null(), 0);
                    }
                    PickKind::Points => self.finish_or_cancel(hwnd),
                    PickKind::Area => {}
                },
                VK_BACK if self.kind == PickKind::Points => {
                    self.added.pop();
                    InvalidateRect(hwnd, null(), 0);
                }
                VK_LEFT | VK_RIGHT | VK_UP | VK_DOWN => {
                    // Nudge the cursor for pixel-exact picking; Shift = 10 px.
                    let step = if super::win::is_key_down(VK_SHIFT) { 10 } else { 1 };
                    let (dx, dy) = match vk {
                        VK_LEFT => (-step, 0),
                        VK_RIGHT => (step, 0),
                        VK_UP => (0, -step),
                        _ => (0, step),
                    };
                    let (x, y) = cursor_pos();
                    set_cursor_pos(x + dx, y + dy);
                }
                _ => {}
            }
        }
    }

    /// Invalidate whatever moved since the last paint.
    unsafe fn refresh(&mut self, hwnd: HWND) {
        let new = self.current_parts();
        for (old, new) in self.parts.iter().zip(new.iter()) {
            if !same(old, new) {
                unsafe {
                    InvalidateRect(hwnd, old, 0);
                    InvalidateRect(hwnd, new, 0);
                }
            }
        }
        self.parts = new;
    }

    fn current_parts(&self) -> [RECT; 3] {
        let pad = self.px(6.0);
        let grow = |r: RECT| rect(r.left - pad, r.top - pad, r.right + pad, r.bottom + pad);
        let (loupe, label) = self.loupe_rects();
        let mut moving = rect(loupe.left, loupe.top, loupe.right.max(label.right), label.bottom);
        let arm = self.px(22.0);
        let (cx, cy) = self.cursor;
        moving = union(moving, rect(cx - arm, cy - arm, cx + arm + 1, cy + arm + 1));
        let selection = match (self.kind, self.drag_from) {
            (PickKind::Area, Some(from)) => {
                let r = sel_rect(from, self.cursor);
                union(r, self.size_label_rect(&r))
            }
            _ => EMPTY,
        };
        [grow(moving), grow(selection), grow(self.hint_rect())]
    }

    fn loupe_rects(&self) -> (RECT, RECT) {
        let cell = self.px(8.0).max(4);
        let size = cell * (LOUPE_RADIUS * 2 + 1);
        let label_h = self.px(26.0);
        let gap = self.px(22.0);
        let (cx, cy) = self.cursor;
        let mut x = cx + gap;
        let mut y = cy + gap;
        if x + size > self.size.0 {
            x = cx - gap - size;
        }
        if y + size + label_h + self.px(4.0) > self.size.1 {
            y = cy - gap - size - label_h - self.px(4.0);
        }
        let loupe = rect(x, y, x + size, y + size);
        let label = rect(x, y + size + self.px(4.0), x + size, y + size + self.px(4.0) + label_h);
        (loupe, label)
    }

    fn size_label_rect(&self, sel: &RECT) -> RECT {
        let h = self.px(24.0);
        let w = self.px(110.0);
        let top = if sel.top - h - self.px(6.0) >= 0 { sel.top - h - self.px(6.0) } else { sel.top + self.px(6.0) };
        rect(sel.left, top, sel.left + w, top + h)
    }

    fn hint_rect(&self) -> RECT {
        let mon = self.monitor_at_cursor();
        let text_w = unsafe { text_size(self.back.dc, self.font, &self.hint).0 };
        let w = text_w + self.px(36.0);
        let h = self.px(40.0);
        let left = (mon.left + mon.right) / 2 - w / 2;
        let top = mon.top + self.px(28.0);
        rect(left, top, left + w, top + h)
    }

    /// Monitor rectangle (client coordinates) under the cursor.
    fn monitor_at_cursor(&self) -> RECT {
        let (sx, sy) = self.screen(self.cursor);
        unsafe {
            let mon = MonitorFromPoint(POINT { x: sx, y: sy }, MONITOR_DEFAULTTONEAREST);
            let mut info: MONITORINFO = zeroed();
            info.cbSize = size_of::<MONITORINFO>() as u32;
            if GetMonitorInfoW(mon, &mut info) == 0 {
                return rect(0, 0, self.size.0, self.size.1);
            }
            let r = info.rcMonitor;
            rect(r.left - self.origin.0, r.top - self.origin.1, r.right - self.origin.0, r.bottom - self.origin.1)
        }
    }

    unsafe fn on_paint(&mut self, hwnd: HWND) {
        unsafe {
            let rgn = CreateRectRgn(0, 0, 0, 0);
            GetUpdateRgn(hwnd, rgn, 0);
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let r = ps.rcPaint;
            let (w, h) = (r.right - r.left, r.bottom - r.top);
            let dc = self.back.dc;

            // Everything is composed into the back buffer, clipped to the exact
            // update region, so only what changed is copied around.
            SelectClipRgn(dc, rgn);
            BitBlt(dc, r.left, r.top, w, h, self.dim.dc, r.left, r.top, SRCCOPY);
            self.draw_route(dc);
            self.draw_selection(dc);
            self.draw_crosshair(dc);
            self.draw_loupe(dc);
            self.draw_hint(dc);
            SelectClipRgn(dc, null_mut());

            BitBlt(hdc, r.left, r.top, w, h, dc, r.left, r.top, SRCCOPY);
            EndPaint(hwnd, &ps);
            DeleteObject(rgn);
        }
    }

    unsafe fn draw_route(&self, dc: HDC) {
        if self.kind != PickKind::Points {
            return;
        }
        unsafe {
            let all: Vec<((i32, i32), bool)> =
                self.existing.iter().map(|&p| (p, false)).chain(self.added.iter().map(|&p| (p, true))).collect();
            for pair in all.windows(2) {
                let color = if pair[1].1 { ACCENT } else { OLD_POINT };
                line(dc, pair[0].0, pair[1].0, self.px(2.0).max(1), color);
            }
            let r = self.px(11.0);
            for (i, &((x, y), new)) in all.iter().enumerate() {
                let fill = if new { ACCENT } else { OLD_POINT };
                with_brush_pen(dc, fill, WHITE, self.px(1.5).max(1), || {
                    Ellipse(dc, x - r, y - r, x + r + 1, y + r + 1);
                });
                let label = wide(&(i + 1).to_string());
                let (tw, th) = text_size(dc, self.font_bold, &label);
                text(dc, self.font_bold, x - tw / 2, y - th / 2, &label, WHITE);
            }
        }
    }

    unsafe fn draw_selection(&self, dc: HDC) {
        let Some(from) = self.drag_from else { return };
        if self.kind != PickKind::Area {
            return;
        }
        unsafe {
            let s = sel_rect(from, self.cursor);
            BitBlt(dc, s.left, s.top, s.right - s.left, s.bottom - s.top, self.bright.dc, s.left, s.top, SRCCOPY);
            let t = self.px(2.0).max(1);
            with_brush_pen_null(dc, ACCENT, t, || {
                Rectangle(dc, s.left - t / 2, s.top - t / 2, s.right + t / 2 + 1, s.bottom + t / 2 + 1);
            });
            let lr = self.size_label_rect(&s);
            panel(dc, lr, self.px(10.0), ACCENT, ACCENT);
            let label = wide(&format!("{} × {}", s.right - s.left, s.bottom - s.top));
            let (tw, th) = text_size(dc, self.font_bold, &label);
            text(
                dc,
                self.font_bold,
                lr.left + (lr.right - lr.left - tw) / 2,
                lr.top + (lr.bottom - lr.top - th) / 2,
                &label,
                WHITE,
            );
        }
    }

    unsafe fn draw_crosshair(&self, dc: HDC) {
        let (cx, cy) = self.cursor;
        let (inner, outer) = (self.px(6.0), self.px(20.0));
        let t = self.px(2.0).max(1);
        unsafe {
            for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let a = (cx + dx * inner, cy + dy * inner);
                let b = (cx + dx * outer, cy + dy * outer);
                line(dc, a, b, t + 2, BLACK);
                line(dc, a, b, t, WHITE);
            }
        }
    }

    unsafe fn draw_loupe(&self, dc: HDC) {
        let (l, label) = self.loupe_rects();
        let n = LOUPE_RADIUS * 2 + 1;
        let cell = (l.right - l.left) / n;
        let (cx, cy) = self.cursor;
        unsafe {
            with_brush_pen(dc, BLACK, BLACK, 1, || {
                Rectangle(dc, l.left, l.top, l.right, l.bottom);
            });
            SetStretchBltMode(dc, COLORONCOLOR);
            StretchBlt(
                dc,
                l.left,
                l.top,
                l.right - l.left,
                l.bottom - l.top,
                self.bright.dc,
                cx - LOUPE_RADIUS,
                cy - LOUPE_RADIUS,
                n,
                n,
                SRCCOPY,
            );
            // Highlight the exact pixel under the cursor.
            let px = l.left + LOUPE_RADIUS * cell;
            let py = l.top + LOUPE_RADIUS * cell;
            with_brush_pen_null(dc, BLACK, 3, || {
                Rectangle(dc, px - 1, py - 1, px + cell + 2, py + cell + 2);
            });
            with_brush_pen_null(dc, WHITE, 1, || {
                Rectangle(dc, px, py, px + cell + 1, py + cell + 1);
            });
            let t = self.px(2.0).max(1);
            with_brush_pen_null(dc, ACCENT, t, || {
                Rectangle(dc, l.left, l.top, l.right + 1, l.bottom + 1);
            });

            panel(dc, label, self.px(8.0), PANEL, PANEL_BORDER);
            let (sx, sy) = self.screen(self.cursor);
            let coords = wide(&format!("{sx}, {sy}"));
            let (tw, th) = text_size(dc, self.font_bold, &coords);
            let ty = label.top + (label.bottom - label.top - th) / 2;
            text(dc, self.font_bold, label.left + (label.right - label.left - tw) / 2, ty, &coords, TEXT);
        }
    }

    unsafe fn draw_hint(&self, dc: HDC) {
        let r = self.hint_rect();
        unsafe {
            panel(dc, r, self.px(14.0), PANEL, PANEL_BORDER);
            let (tw, th) = text_size(dc, self.font, &self.hint);
            text(
                dc,
                self.font,
                r.left + (r.right - r.left - tw) / 2,
                r.top + (r.bottom - r.top - th) / 2,
                &self.hint,
                TEXT,
            );
            if self.kind == PickKind::Points && !self.added.is_empty() {
                let count = wide(&format!("+{}", self.added.len()));
                let (cw, ch) = text_size(dc, self.font_bold, &count);
                let x = r.right + self.px(8.0);
                let pill = rect(x, r.top + self.px(6.0), x + cw + self.px(20.0), r.bottom - self.px(6.0));
                panel(dc, pill, self.px(12.0), ACCENT, ACCENT);
                text(
                    dc,
                    self.font_bold,
                    pill.left + self.px(10.0),
                    pill.top + (pill.bottom - pill.top - ch) / 2,
                    &count,
                    WHITE,
                );
            }
        }
    }
}

fn same(a: &RECT, b: &RECT) -> bool {
    (a.left, a.top, a.right, a.bottom) == (b.left, b.top, b.right, b.bottom)
}

fn sel_rect(a: (i32, i32), b: (i32, i32)) -> RECT {
    rect(a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1))
}

fn union(a: RECT, b: RECT) -> RECT {
    if same(&a, &EMPTY) {
        return b;
    }
    if same(&b, &EMPTY) {
        return a;
    }
    rect(a.left.min(b.left), a.top.min(b.top), a.right.max(b.right), a.bottom.max(b.bottom))
}

unsafe fn text_size(dc: HDC, font: HFONT, s: &[u16]) -> (i32, i32) {
    unsafe {
        let old = SelectObject(dc, font);
        let mut size = SIZE { cx: 0, cy: 0 };
        GetTextExtentPoint32W(dc, s.as_ptr(), s.len() as i32, &mut size);
        SelectObject(dc, old);
        (size.cx, size.cy)
    }
}

unsafe fn text(dc: HDC, font: HFONT, x: i32, y: i32, s: &[u16], color: u32) {
    unsafe {
        let old = SelectObject(dc, font);
        SetBkMode(dc, TRANSPARENT as i32);
        SetTextColor(dc, color);
        TextOutW(dc, x, y, s.as_ptr(), s.len() as i32);
        SelectObject(dc, old);
    }
}

unsafe fn with_brush_pen(dc: HDC, fill: u32, stroke: u32, width: i32, f: impl FnOnce()) {
    unsafe {
        let brush = CreateSolidBrush(fill);
        let pen = CreatePen(PS_SOLID, width, stroke);
        let ob = SelectObject(dc, brush);
        let op = SelectObject(dc, pen);
        f();
        SelectObject(dc, ob);
        SelectObject(dc, op);
        DeleteObject(brush);
        DeleteObject(pen);
    }
}

unsafe fn with_brush_pen_null(dc: HDC, stroke: u32, width: i32, f: impl FnOnce()) {
    unsafe {
        let pen = CreatePen(PS_SOLID, width, stroke);
        let ob = SelectObject(dc, GetStockObject(NULL_BRUSH));
        let op = SelectObject(dc, pen);
        f();
        SelectObject(dc, ob);
        SelectObject(dc, op);
        DeleteObject(pen);
    }
}

unsafe fn panel(dc: HDC, r: RECT, radius: i32, fill: u32, border: u32) {
    unsafe {
        with_brush_pen(dc, fill, border, 1, || {
            RoundRect(dc, r.left, r.top, r.right, r.bottom, radius, radius);
        });
    }
}

unsafe fn line(dc: HDC, a: (i32, i32), b: (i32, i32), width: i32, color: u32) {
    unsafe {
        let pen = CreatePen(PS_SOLID, width, color);
        let op = SelectObject(dc, pen);
        MoveToEx(dc, a.0, a.1, null_mut());
        LineTo(dc, b.0, b.1);
        SelectObject(dc, op);
        DeleteObject(pen);
    }
}

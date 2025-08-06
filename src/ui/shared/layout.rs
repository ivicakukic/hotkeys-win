use std::fmt::{self, Display, Formatter};

use serde::{Deserialize, Serialize};
use windows::{
    core::Result,
    Win32::{
        Foundation::{ HWND, RECT },
        UI::WindowsAndMessaging::{
                GetWindowLongW, SetWindowLongW, AdjustWindowRectEx,
                WS_EX_LAYERED, WS_OVERLAPPEDWINDOW, WINDOW_EX_STYLE, WINDOW_STYLE, WS_POPUP, WS_BORDER, WS_SIZEBOX, WS_EX_TOOLWINDOW, WS_EX_APPWINDOW, GWL_STYLE, GWL_EXSTYLE,
            },
    }
};


use crate::ui::shared::utils::set_window_pos;

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}


#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WindowLayout {
    pub style: WindowStyle,
    pub rect: Rect,
}

impl Default for WindowLayout {

    fn default() -> Self {
        fn get_screen_size() -> Result<(i32, i32)> {
            use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
            let screen_w = unsafe { GetSystemMetrics(SM_CXSCREEN) };
            let screen_h = unsafe { GetSystemMetrics(SM_CYSCREEN) };
            Ok((screen_w, screen_h))
        }

        let style = WindowStyle::Window;
        let (width, height) = (800, 600);
        let (scr_width, scr_height) = get_screen_size().unwrap_or((width, height));

        // center the window on the screen
        let left = (scr_width - width) / 2;
        let top = (scr_height - height) / 2;

        WindowLayout {
            style,
            rect: Rect { left, top, right: left + width, bottom: top + height },
        }
    }
}

impl WindowLayout {
    pub fn get_adjusted_rect(&self) -> Result<Rect> {
        let mut rect = RECT {
            left: self.rect.left,
            top: self.rect.top,
            right: self.rect.right,
            bottom: self.rect.bottom,
        };
        unsafe { AdjustWindowRectEx(&mut rect, self.style.style(), false, self.style.ex_style())?; }
        let rect = Rect {
            left: rect.left,
            top: rect.top,
            right: rect.right,
            bottom: rect.bottom,
        };
        Ok(rect)
    }

}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum WindowStyle {
    // Has title bar, borders, HAS taskbar icon (window, default)
    Window,
    // No title bar, no borders, NO taskbar icon
    Floating,
    // No title bar, no borders, HAS taskbar icon
    Taskbar,
}

impl WindowStyle {
    pub fn next(&self) -> WindowStyle {
        match *self {
            WindowStyle::Window => WindowStyle::Floating,
            WindowStyle::Floating => WindowStyle::Taskbar,
            WindowStyle::Taskbar => WindowStyle::Window,
        }
    }

    pub fn style(&self) -> WINDOW_STYLE {
        match *self {
            WindowStyle::Window => WS_OVERLAPPEDWINDOW | WS_SIZEBOX,
            WindowStyle::Floating => WS_POPUP | WS_BORDER,
            WindowStyle::Taskbar => WS_POPUP | WS_BORDER,
        }
    }

    pub fn ex_style(&self) -> WINDOW_EX_STYLE {
        match *self {
            WindowStyle::Window => WS_EX_LAYERED,
            WindowStyle::Floating => WS_EX_LAYERED | WS_EX_TOOLWINDOW,
            WindowStyle::Taskbar => WS_EX_LAYERED | WS_EX_APPWINDOW,
        }
    }

    pub fn apply(&self, hwnd: HWND, is_topmost: bool) {
        let my_style = self.style();
        let my_ex_style = self.ex_style();

        let other_style = match self {
            WindowStyle::Window => WindowStyle::Floating.style() | WindowStyle::Taskbar.style(),
            WindowStyle::Floating => WindowStyle::Window.style() | WindowStyle::Taskbar.style(),
            WindowStyle::Taskbar => WindowStyle::Window.style() | WindowStyle::Floating.style(),
        };
        let other_ex_style = match self {
            WindowStyle::Window => WindowStyle::Floating.ex_style() | WindowStyle::Taskbar.ex_style(),
            WindowStyle::Floating => WindowStyle::Window.ex_style() | WindowStyle::Taskbar.ex_style(),
            WindowStyle::Taskbar => WindowStyle::Window.ex_style() | WindowStyle::Floating.ex_style(),
        };

        let current_style = unsafe { WINDOW_STYLE(GetWindowLongW(hwnd, GWL_STYLE) as u32) };
        let current_ex_style = unsafe { WINDOW_EX_STYLE(GetWindowLongW(hwnd, GWL_EXSTYLE) as u32) };

        let new_style = (current_style & !other_style) | my_style;
        let new_ex_style = (current_ex_style & !other_ex_style) | my_ex_style;

        unsafe {
            SetWindowLongW(hwnd, GWL_STYLE, new_style.0 as i32);
            SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex_style.0 as i32);
            set_window_pos(hwnd, is_topmost);
        }
    }

    pub fn from_string(s: &str) -> Self {
        match s {
            "Floating" => WindowStyle::Floating,
            "Taskbar" => WindowStyle::Taskbar,
            "Window" => WindowStyle::Window,
            _ => WindowStyle::Window, // Fallback variant
        }
    }

}

impl Display for WindowStyle {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

use tray_item::{IconSource, TrayItem};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{SendMessageW, WM_CLOSE},
};

use crate::app::settings::{WM_OPEN_SETTINGS, WM_RELOAD_SETTINGS};

pub fn create(hwnd: isize) -> TrayItem {
    let mut tray = TrayItem::new("Hotkeys", IconSource::Resource("id")).unwrap();

    tray.add_menu_item("Settings", move || unsafe {
        SendMessageW(HWND(hwnd as *mut _), WM_OPEN_SETTINGS, None, None);
    })
    .unwrap();

    tray.add_menu_item("Reload", move || unsafe {
        SendMessageW(HWND(hwnd as *mut _), WM_RELOAD_SETTINGS, None, None);
    })
    .unwrap();

    tray.inner_mut().add_separator().unwrap();

    tray.add_menu_item("Quit", move || unsafe {
        SendMessageW(HWND(hwnd as *mut _), WM_CLOSE, None, None);
    })
    .unwrap();

    tray
}

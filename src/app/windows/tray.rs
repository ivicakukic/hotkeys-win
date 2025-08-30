use tray_item::{IconSource, TrayItem};
use windows::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{SendMessageW, WM_CLOSE, WM_USER},
};

pub const WM_RELOAD_SETTINGS:u32 = WM_USER + 10;
pub const WM_OPEN_SETTINGS:u32 = WM_USER + 11;
pub const WM_SAVE_SETTINGS:u32 = WM_USER + 12;
pub const WM_SHOW_APPLICATION:u32 = WM_USER + 13;

pub fn create(hwnd: isize) -> TrayItem {
    let mut tray = TrayItem::new("Hotkeys", IconSource::Resource("id")).unwrap();

    tray.add_menu_item("Open HotKeys", move || unsafe {
        SendMessageW(HWND(hwnd as *mut _), WM_SHOW_APPLICATION, None, None);
    })
    .unwrap();

    tray.inner_mut().add_separator().unwrap();

    tray.add_menu_item("Settings", move || unsafe {
        SendMessageW(HWND(hwnd as *mut _), WM_OPEN_SETTINGS, None, None);
    })
    .unwrap();

    tray.add_menu_item("Reload", move || unsafe {
        SendMessageW(HWND(hwnd as *mut _), WM_RELOAD_SETTINGS, None, None);
    })
    .unwrap();

    tray.add_menu_item("Save", move || unsafe {
        SendMessageW(HWND(hwnd as *mut _), WM_SAVE_SETTINGS, None, None);
    })
    .unwrap();

    tray.inner_mut().add_separator().unwrap();

    tray.add_menu_item("Quit", move || unsafe {
        SendMessageW(HWND(hwnd as *mut _), WM_CLOSE, None, None);
    })
    .unwrap();

    tray
}

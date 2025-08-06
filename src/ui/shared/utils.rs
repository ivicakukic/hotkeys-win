use windows::
    Win32::{
        Foundation::{ HWND },
        Graphics::Gdi::UpdateWindow,
        UI::WindowsAndMessaging::{SetWindowPos, HWND_TOP, HWND_TOPMOST, SWP_DRAWFRAME, SWP_NOMOVE, SWP_NOSIZE},
    }
;

pub unsafe fn set_window_pos(hwnd: HWND, is_topmost: bool) {
    let _ = SetWindowPos(hwnd, Some(if is_topmost { HWND_TOPMOST } else { HWND_TOP }), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_DRAWFRAME);
    let _ = UpdateWindow(hwnd);
}

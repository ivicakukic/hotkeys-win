use windows::
    Win32::{
        Foundation::{HWND, LPARAM, WPARAM},
        Graphics::Gdi::UpdateWindow,
        UI::WindowsAndMessaging::{SetWindowPos, PostMessageW, HWND_TOP, HWND_TOPMOST, SWP_DRAWFRAME, SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER},
    }
;

use crate::ui::shared::layout::Rect;

pub unsafe fn reset_window_pos(hwnd: HWND, is_topmost: bool) {
    let _ = SetWindowPos(hwnd, Some(if is_topmost { HWND_TOPMOST } else { HWND_TOP }), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_DRAWFRAME);
    let _ = UpdateWindow(hwnd);
}

pub unsafe fn set_window_rect(hwnd: HWND, rect: &Rect) {
    let x = rect.left;
    let y = rect.top;
    let width = rect.width();
    let height = rect.height();
    let _ = SetWindowPos(hwnd, None, x, y, width, height, SWP_NOZORDER | SWP_DRAWFRAME);
    let _ = UpdateWindow(hwnd);
}

/// Copy string into fixed-size array with null termination
pub fn copy_string_to_array(array: &mut [u8], s: &str) {
    let bytes = s.as_bytes();
    let len = bytes.len().min(array.len() - 1); // Leave room for null terminator
    array[..len].copy_from_slice(&bytes[..len]);
    array[len] = 0; // Null terminator
}

/// Extract string from fixed-size array (stops at null terminator)
pub fn get_string_from_array(array: &[u8]) -> &str {
    let end = array.iter().position(|&x| x == 0).unwrap_or(array.len());
    std::str::from_utf8(&array[..end]).unwrap_or("")
}

/// Generic function to send serializable data through Windows message queue
pub fn send_window_message<T>(hwnd: HWND, msg: u32, data: T) {
    let boxed = Box::new(data);
    let ptr = Box::into_raw(boxed);
    unsafe {
        let _ = PostMessageW(Some(hwnd), msg, WPARAM(ptr as usize), LPARAM(0));
    };
}

/// Generic function to receive and take ownership of data from Windows message queue
pub unsafe fn receive_window_message<T>(wparam: WPARAM) -> T {
    let ptr = wparam.0 as *mut T;
    *Box::from_raw(ptr)
}

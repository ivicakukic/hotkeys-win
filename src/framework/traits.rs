use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};

pub trait AppHandler {
    fn app_wnd_proc(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT;
}

pub trait Window {
    fn set_hwnd(&mut self, hwnd: HWND);
    fn handle_message(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> Option<LRESULT>;
}
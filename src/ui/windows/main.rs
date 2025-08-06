use std::sync::Once;

use windows::{
    core::{h, Result, HSTRING},
    Win32::{
        Foundation::{ HMODULE, HWND, LPARAM, LRESULT, WPARAM },
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
            CreateWindowExW, RegisterClassW, WM_CREATE, WNDCLASSW
        },
    }
};

use crate::{
    framework::{Window, wnd_proc_router},
    ui::shared::utils::set_window_pos,
    ui::shared::layout::WindowStyle::Floating
};


static REGISTER_WINDOW_CLASS: Once = Once::new();
static WINDOW_CLASS_NAME: &HSTRING = h!("HotKeys.MainWindow");

pub struct MainWindow {
    hwnd: HWND
}

impl MainWindow {
    fn register_window_class(hinstance: HMODULE) {
        REGISTER_WINDOW_CLASS.call_once(|| {
            let class = WNDCLASSW {
                hInstance: hinstance.into(),
                lpszClassName: windows::core::PCWSTR(WINDOW_CLASS_NAME.as_ptr()),
                lpfnWndProc: Some(wnd_proc_router::<Self>),
                ..Default::default()
            };
            assert_ne!(unsafe { RegisterClassW(&class) }, 0);
        });
    }

    pub fn new(
        title: &str, width: i32, height: i32,
    ) -> Result<Box<MainWindow>> {

        let hinstance = unsafe { GetModuleHandleW(None)? };
        Self::register_window_class(hinstance);
        let mut this = Box::new(Self {
            hwnd: HWND::default()
        });

        unsafe {
            CreateWindowExW(
                Floating.ex_style(),
                WINDOW_CLASS_NAME,
                &HSTRING::from(title),
                Floating.style(),
                0,
                0,
                width,
                height,
                None,
                None,
                Some(hinstance.into()),
                Some(&mut *this as *mut _ as _),
            )?
        };

        // we dont show this window at all, it's used for message routing only
        Ok(this)
    }

    pub fn hwnd(&self) -> isize {
        self.hwnd.0 as isize
    }

    unsafe fn on_create(&mut self) -> LRESULT {
        // no reason to apply the style here, it's already applied in the constructor
        set_window_pos(self.hwnd, true);
        LRESULT(0)
    }

}

impl Window for MainWindow {
    fn set_hwnd(&mut self, hwnd: HWND) {
        self.hwnd = hwnd;
    }
    fn handle_message(
        &mut self,
        _hwnd: HWND,
        msg: u32,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Option<LRESULT> {
        match msg {
            WM_CREATE => {
                Some(unsafe { self.on_create() })
            }
            _ => None,
        }
    }
}
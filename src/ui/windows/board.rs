use std::sync::Once;
use std::ffi::c_void;


use windows::{
    core::{h, Result, HSTRING},
    Win32::{
        Foundation::{ COLORREF, HMODULE, HWND, LPARAM, LRESULT, WPARAM },
        Graphics::Gdi::{InvalidateRect, HBRUSH},
        System::LibraryLoader::GetModuleHandleW,
        UI::WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, KillTimer, LoadCursorW, LoadIconW, PostMessageW, RegisterClassW, SetLayeredWindowAttributes, SetTimer, ShowWindow, IDC_ARROW, LWA_ALPHA, SW_SHOW, WM_CLOSE, WM_CREATE, WM_DESTROY, WM_KEYDOWN, WM_MOVE, WM_PAINT, WM_RBUTTONDOWN, WM_SIZE, WM_TIMER, WM_USER, WNDCLASSW
            },
    }
};


use crate::{
    app::settings::ColorScheme,
    framework::{Window, wnd_proc_router},
    ui::{
        components::assets::Assets,
        components::painter::BoardPainter,
        shared::layout::WindowLayout,
    },
    input::keys::vkey::VK_NUMPAD1,
    app::settings::Profile,
};

pub const WM_BOARD_COMMAND:u32 = WM_USER + 4;
pub const WM_BOARD_FINISHED:u32 = WM_USER + 5;

const ID_TIMER_TIMEOUT: usize = 1;
const ID_TIMER_FEEDBACK: usize = 2;

static REGISTER_WINDOW_CLASS: Once = Once::new();
static WINDOW_CLASS_NAME: &HSTRING = h!("HotKeys.Window");

pub struct BoardWindow {
    hwnd: HWND,
    pub layout: WindowLayout,
    colors: ColorScheme,
    pub profile: Profile,
    timeout: u32,
    feedback: f64,
    selected_pad: Option<u8>,
}

impl BoardWindow {

    fn register_window_class(hinstance: HMODULE) {
        REGISTER_WINDOW_CLASS.call_once(|| {
            let class = WNDCLASSW {
                hCursor: unsafe { LoadCursorW(None, IDC_ARROW).ok().unwrap() },
                hIcon: unsafe { LoadIconW(Some(hinstance.into()), h!("ID")).ok().unwrap() },
                hInstance: hinstance.into(),
                lpszClassName: windows::core::PCWSTR(WINDOW_CLASS_NAME.as_ptr()),
                lpfnWndProc: Some(wnd_proc_router::<Self>),
                hbrBackground: HBRUSH(16 as *mut _), // COLOR_WINDOW+1
                ..Default::default()
            };
            assert_ne!(unsafe { RegisterClassW(&class) }, 0);
        });
    }

    pub fn new(
        title: &str,
        layout: WindowLayout,
        colors: ColorScheme,
        profile: Profile,
        timeout: u32,
        feedback: f64,
    ) -> Result<Box<BoardWindow>> {

        let hinstance = unsafe { GetModuleHandleW(None)? };
        Self::register_window_class(hinstance);

        let style = layout.style.style();
        let ex_style = layout.style.ex_style();
        let rect = layout.get_adjusted_rect()?;

        let mut this = Box::new(Self {
            hwnd: HWND::default(),
            layout: layout,
            colors: colors,
            profile: profile,
            timeout: timeout,
            feedback: feedback,
            selected_pad: None,
        });


        let hwnd = unsafe {
            CreateWindowExW(
                ex_style,
                WINDOW_CLASS_NAME,
                &HSTRING::from(title),
                style,
                rect.left,
                rect.top,
                rect.right - rect.left,
                rect.bottom - rect.top,
                None,
                None,
                Some(hinstance.into()),
                Some(&mut *this as *mut _ as _),
            )?
        };

        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
        };

        Ok(this)
    }

    pub fn hide(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd).unwrap_or_default();
        }
    }

    fn on_create(&self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            let balpha = (self.colors.opacity() * 255.0) as u8;
            let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0x00), balpha, LWA_ALPHA);
            self.set_timer(hwnd, ID_TIMER_TIMEOUT, (self.timeout as f64).signum());
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }

    fn on_rotate_style(&mut self) -> LRESULT {
        self.layout.style = self.layout.style.next();
        self.layout.style.apply(self.hwnd, false);
        LRESULT(0)
    }

    fn on_paint(&self, hwnd: HWND) -> LRESULT {
        unsafe {
            BoardPainter {
                profile: &self.profile,
                timeout: self.timeout as u8,
                selected_pad: self.selected_pad,
                assets: &Assets::from(&self.colors),
            }.paint(hwnd)
        };
        LRESULT(0)
    }

    fn on_size(&mut self, hwnd: HWND, width: i32, height: i32) -> LRESULT {
        self.layout.rect.right = self.layout.rect.left + width;
        self.layout.rect.bottom = self.layout.rect.top + height;
        self.invalidate(hwnd)
    }

    fn on_move(&mut self, x: i32, y: i32) -> LRESULT {
        self.layout.rect.left = x;
        self.layout.rect.top = y;
        LRESULT(0)
    }

    fn on_keydown(&mut self, hwnd: HWND, wparam: WPARAM) -> LRESULT {
        let pad_id = wparam.0 as isize - VK_NUMPAD1.vkey as isize;

        if pad_id < 0 || pad_id >= 9 {
            self.post_board_finished_msg(hwnd);
            return LRESULT(0);
        }

        if self.feedback <= 0.0 {
            self.post_board_command_msg(hwnd, pad_id as u8);
            return LRESULT(0);
        }

        self.selected_pad = Some(pad_id as u8);
        self.invalidate(hwnd);
        self.set_timer(hwnd, ID_TIMER_FEEDBACK, self.feedback);

        LRESULT(0)
    }

    fn on_timer(&mut self, hwnd: HWND, wparam: WPARAM) -> LRESULT {
        match wparam.0 {
            ID_TIMER_TIMEOUT => {
                if self.timeout > 0 {
                    self.timeout -= 1;
                    self.invalidate(hwnd);

                    if self.timeout == 0 {
                        self.kill_timers(hwnd);
                        self.post_board_finished_msg(hwnd);
                    }
                }
            },
            ID_TIMER_FEEDBACK => {
                self.kill_timers(hwnd);
                if let Some(selected_pad) = self.selected_pad {
                    self.post_board_command_msg(hwnd, selected_pad);
                } else {
                    log::warn!("No pad selected for feedback timer");
                }
            },
            _ => log::warn!("Unknown timer event: {}", wparam.0),
        }
        LRESULT(0)
    }

    fn post_board_finished_msg(&self, hwnd: HWND) {
        let hwnd_val = hwnd.0 as usize;
        unsafe {
            PostMessageW(
                Some(HWND(hwnd_val as *mut c_void)),
                WM_BOARD_FINISHED,
                WPARAM(0),
                LPARAM(0)
            ).unwrap_or_default();
        }
    }

    fn post_board_command_msg(&self, hwnd: HWND, command: u8) {
        let hwnd_val = hwnd.0 as usize;
        unsafe {
            PostMessageW(
                Some(HWND(hwnd_val as *mut c_void)),
                WM_BOARD_COMMAND,
                WPARAM(command as usize),
                LPARAM(self.profile.id as isize)
            ).unwrap_or_default();
        }
    }


    fn set_timer(&self, hwnd: HWND, id: usize, seconds: f64) {
        if seconds > 0.0 {
            unsafe {
                SetTimer(Some(hwnd), id, (seconds * 1000.0) as u32, None);
            }
        }
    }

    fn kill_timers(&self, hwnd: HWND) -> LRESULT {
        unsafe { let _ = KillTimer(Some(hwnd), ID_TIMER_TIMEOUT); }
        unsafe { let _ = KillTimer(Some(hwnd), ID_TIMER_FEEDBACK); }
        LRESULT(0)
    }

    fn invalidate(&self, hwnd: HWND) -> LRESULT {
        unsafe {
            LRESULT(!InvalidateRect(Some(hwnd), None, true).as_bool() as isize)
        }
    }



}


impl Window for BoardWindow {
    fn set_hwnd(&mut self, hwnd: HWND) {
        self.hwnd = hwnd;
    }

    fn handle_message(
        &mut self,
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Option<LRESULT> {

        fn loword(lparam: LPARAM) -> i32 { (lparam.0 as usize & 0xffff) as i32 }
        fn hiword(lparam: LPARAM) -> i32 { ((lparam.0 as usize >> 16) & 0xffff) as i32 }


        match msg {
            WM_CREATE => {
                Some(self.on_create(hwnd, msg, wparam, lparam))
            },
            WM_PAINT => {
                Some(self.on_paint(hwnd))
            },
            WM_KEYDOWN => {
                Some(self.on_keydown(hwnd, wparam))
            },
            WM_RBUTTONDOWN => {
                Some(self.on_rotate_style())
            },
            WM_SIZE => {
                Some(self.on_size(hwnd, loword(lparam), hiword(lparam)))
            },
            WM_MOVE => {
                Some(self.on_move(loword(lparam), hiword(lparam)))
            },
            WM_TIMER => {
                Some(self.on_timer(hwnd, wparam))
            },
            WM_CLOSE => {
                self.kill_timers(hwnd); // kill the timer, let the app handle WM_CLOSE
                None
            },
            WM_DESTROY => {
                Some(self.kill_timers(hwnd))
            },
            _ => None,
        }
    }
}

use std::sync::Once;
use std::ffi::c_void;


use windows::{
    core::{h, Result, HSTRING},
    Win32::{
        Foundation::{ HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM },
        Graphics::Gdi::{InvalidateRect, HBRUSH},
        System::LibraryLoader::GetModuleHandleW,
        UI::{
            Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_ESCAPE, VK_NUMPAD0},
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, KillTimer, LoadCursorW, LoadIconW, PostMessageW, RegisterClassW, SetTimer, ShowWindow, IDC_ARROW, SW_SHOW, WM_CLOSE, WM_CREATE, WM_DESTROY, WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_MOVE, WM_PAINT, WM_RBUTTONDOWN, WM_SIZE, WM_SYSKEYDOWN, WM_SYSKEYUP, WM_TIMER, WM_USER, WNDCLASSW
            }
        },
    }
};


use crate::{
    input::{ModifierHandler, ModifierState},
    components::{BoardComponent, ChildWindowRequest, Direction, KeyboardEvent, MouseEvent, MouseEventTarget, SetWindowPosCommand, UiEvent, UiEventResult},
    framework::{wnd_proc_router, Window},
    model::{PadId},
    ui::components::painter,
    ui::shared::{ layout::WindowLayout, utils::{reset_window_pos, set_window_rect}}
};

pub const WM_BOARD_COMMAND:u32 = WM_USER + 20;
pub const WM_BOARD_FINISHED:u32 = WM_USER + 21;
pub const WM_UPDATE_LAYOUT:u32 = WM_USER + 22;
const WM_SHOW_CHILD_WINDOW:u32 = WM_USER + 23;

const ID_TIMER_TIMEOUT: usize = 1;
const ID_TIMER_FEEDBACK: usize = 2;

static REGISTER_WINDOW_CLASS: Once = Once::new();
static WINDOW_CLASS_NAME: &HSTRING = h!("HotKeys.Window");

pub struct BoardWindow {
    hwnd: HWND,
    layout: WindowLayout,
    board: Box<dyn BoardComponent>,
    timeout: u32,
    feedback: u64,
    selected_pad: Option<PadId>,
    modifier_state: ModifierState,
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
        board: Box<dyn BoardComponent>,
        timeout: u32,
        feedback: u64,
    ) -> Result<Box<BoardWindow>> {

        let hinstance = unsafe { GetModuleHandleW(None)? };
        Self::register_window_class(hinstance);

        let style = layout.style.style();
        let ex_style = layout.style.ex_style();
        let rect = layout.get_adjusted_rect()?;

        let mut this = Box::new(Self {
            hwnd: HWND::default(),
            layout: layout,
            board: board,
            timeout: timeout,
            feedback: feedback,
            selected_pad: None,
            modifier_state: ModifierState::default(),
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

    pub fn modifier_state(&self) -> &ModifierState {
        &self.modifier_state
    }

    pub fn board(&self) -> &dyn BoardComponent {
        self.board.as_ref()
    }

    pub fn layout(&self) -> &WindowLayout {
        &self.layout
    }

    fn on_create(&self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            // Don't call SetLayeredWindowAttributes - we use UpdateLayeredWindow instead
            // let balpha = (self.board.color_scheme().opacity() * 255.0) as u8;
            // let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0x00), balpha, LWA_ALPHA);
            self.set_timer(hwnd, ID_TIMER_TIMEOUT, (self.timeout as f64).signum());

            // Immediately render the window to make it visible
            self.update_layered_window(hwnd);

            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }

    #[allow(dead_code)]
    fn on_rotate_style(&mut self) -> LRESULT {
        self.layout.style = self.layout.style.next();
        self.layout.style.apply(self.hwnd, false);
        LRESULT(0)
    }

    fn on_paint(&self, hwnd: HWND) -> LRESULT {
        unsafe {
            // Need to call BeginPaint/EndPaint to satisfy Windows paint cycle
            let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
            let _hdc = windows::Win32::Graphics::Gdi::BeginPaint(hwnd, &mut ps);

            // Update layered window instead of drawing to HDC
            self.update_layered_window(hwnd);

            let _ = windows::Win32::Graphics::Gdi::EndPaint(hwnd, &ps);
        }
        LRESULT(0)
    }

    unsafe fn update_layered_window(&self, hwnd: HWND) {
        use windows::Win32::Graphics::Gdi::{
            CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
            BITMAPINFOHEADER, BITMAPINFO, DIB_RGB_COLORS, HGDIOBJ, BLENDFUNCTION
        };
        use windows::Win32::UI::WindowsAndMessaging::{UpdateLayeredWindow, ULW_ALPHA};
        use std::{mem, ptr};

        let mut rect = windows::Win32::Foundation::RECT::default();
        let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect);
        let rect: RECT = self.layout.get_adjusted_rect().map(Into::into).unwrap_or(rect);
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;

        // Get screen DC and create memory DC
        let screen_dc = GetDC(None);
        let mem_dc = CreateCompatibleDC(Some(screen_dc));

        // Create 32-bit RGBA bitmap
        let bmi = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // Negative for top-down bitmap
                biPlanes: 1,
                biBitCount: 32,
                biCompression: 0, // BI_RGB
                biSizeImage: 0,
                biXPelsPerMeter: 0,
                biYPelsPerMeter: 0,
                biClrUsed: 0,
                biClrImportant: 0,
            },
            bmiColors: [Default::default()],
        };

        let mut bits: *mut std::ffi::c_void = ptr::null_mut();
        let bitmap = CreateDIBSection(
            Some(mem_dc),
            &bmi,
            DIB_RGB_COLORS,
            &mut bits,
            None,
            0,
        ).unwrap();

        let old_bitmap = SelectObject(mem_dc, HGDIOBJ(bitmap.0));

        // Get pixel array for blending
        let pixel_count = (width * height) as usize;
        let pixels = std::slice::from_raw_parts_mut(bits as *mut painter::RGBA, pixel_count);

        // Initialize with transparent background based on board color scheme
        let board = self.board.as_ref().data();
        let (bg_r, bg_g, bg_b) = board.color_scheme().background().to_rgb();
        let bg_alpha = (board.color_scheme().opacity() * 255.0) as u8;

        for pixel in pixels.iter_mut() {
            *pixel = painter::RGBA {
                r: (bg_r as u16 * bg_alpha as u16 / 255) as u8,
                g: (bg_g as u16 * bg_alpha as u16 / 255) as u8,
                b: (bg_b as u16 * bg_alpha as u16 / 255) as u8,
                a: bg_alpha, // Use board's opacity setting
            };
        }

        // Call existing painter with memory DC and pixels for blending
        painter::BoardPainter {
            board: board,
            timeout: self.timeout as u8,
            selected_pad: self.selected_pad,
        }.paint(hwnd, mem_dc, pixels, width as usize, self.modifier_state.clone());

        // Update layered window
        let window_pos = windows::Win32::Foundation::POINT {
            x: self.layout.rect.left,
            y: self.layout.rect.top
        };
        let window_size = windows::Win32::Foundation::SIZE {
            cx: width,
            cy: height
        };
        let source_pos = windows::Win32::Foundation::POINT { x: 0, y: 0 };
        let blend = BLENDFUNCTION {
            BlendOp: 0, // AC_SRC_OVER
            BlendFlags: 0,
            SourceConstantAlpha: 255,
            AlphaFormat: 1, // AC_SRC_ALPHA
        };

        let _ = UpdateLayeredWindow(
            hwnd,
            Some(screen_dc),
            Some(&window_pos),
            Some(&window_size),
            Some(mem_dc),
            Some(&source_pos),
            windows::Win32::Foundation::COLORREF(0),
            Some(&blend),
            ULW_ALPHA,
        );

        // Cleanup
        SelectObject(mem_dc, old_bitmap);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(mem_dc);
        let _ = ReleaseDC(None, screen_dc);
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
        let vk_code = VIRTUAL_KEY(wparam.0 as u16);

        // Stop timeout timer and queue redraw on any key press
        self.stop_timeout_timer(hwnd);

        // Handle modifier keys first
        let old_state = self.modifier_state.clone();
        let mut modifier_handler = ModifierHandler::new(old_state.clone());
        let is_modifier = modifier_handler.handle_key_press(vk_code);
        let new_state = modifier_handler.state().clone();

        if is_modifier {
            if new_state != old_state {
                self.modifier_state = new_state;
                self.invalidate(hwnd);
            } else {
                return LRESULT(0); // No state change, ignore
            }
        }

        if let Some(handler) = self.board.as_mut().handler() {
            match handler.handle_ui_event(EventMapper::key_down(vk_code, new_state)) {
                UiEventResult::Handled => return LRESULT(0),
                UiEventResult::RequiresRedraw => {
                    self.invalidate(hwnd);
                    return LRESULT(0)
                },
                UiEventResult::RequestChildWindow(child_request) => {
                    self.post_child_window_message(hwnd, child_request);
                    return LRESULT(0)
                },
                UiEventResult::PadSelected(pad_id) => {
                    return self.on_pad_selected(pad_id, hwnd)
                }
                UiEventResult::SetWindowPos(command) => {
                    return self.move_or_size(hwnd, command)
                },
                UiEventResult::NotHandled => {},
                _ => {
                    log::warn!("UI layer received state machine operation - should be handled at higher level");
                }
            }
        }

        if is_modifier {
            return LRESULT(0); // Modifier key handled
        }

        // Handle Escape key
        if vk_code == VK_ESCAPE {
            self.post_board_finished_msg(hwnd);
            return LRESULT(0);
        }

        // Handle numeric pad keys
        let pad_id = wparam.0 as i32 - VK_NUMPAD0.0 as i32;
        if pad_id < 1 || pad_id > 9 {
            return LRESULT(0); // Ignore other keys
        }

        let pad_id = PadId::from_keypad_int(pad_id);
        self.on_pad_selected(pad_id, hwnd)
    }

    fn on_keyup(&mut self, hwnd: HWND, wparam: WPARAM) -> LRESULT {
        use windows::Win32::UI::Input::KeyboardAndMouse::*;

        let vk_code = VIRTUAL_KEY(wparam.0 as u16);

        // Handle modifier key releases
        let old_state = self.modifier_state.clone();
        let mut modifier_handler = ModifierHandler::new(old_state.clone());
        let is_modifier = modifier_handler.handle_key_release(vk_code);
        let new_state = modifier_handler.state().clone();

        if is_modifier {
            if new_state != old_state {
                self.modifier_state = new_state;
            }
        }

        if let Some(handler) = self.board.as_mut().handler() {
            match handler.handle_ui_event(EventMapper::key_up(vk_code, new_state)) {
                UiEventResult::RequiresRedraw => {
                    return self.invalidate(hwnd);
                },
                UiEventResult::Handled => return LRESULT(0),
                _ => {}
            }
        }

        if is_modifier {
            self.invalidate(hwnd);
        }

        LRESULT(0)
    }

    fn on_right_mouse_down(&mut self, hwnd: HWND, _wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        if let Some(handler) = self.board.as_mut().handler() {
            if let Some(target) = self.layout.hit_test(loword(lparam), hiword(lparam)) {
                let modifier_state = self.modifier_state.clone();
                match handler.handle_ui_event(EventMapper::right_mouse_down(target.clone(), modifier_state)) {
                    UiEventResult::Handled => return LRESULT(0),
                    UiEventResult::RequiresRedraw => {
                        self.invalidate(hwnd);
                        return LRESULT(0)
                    },
                    UiEventResult::PadSelected(pad_id) => {
                        return self.on_pad_selected(pad_id, hwnd)
                    }
                    UiEventResult::NotHandled => {
                        match target {
                            MouseEventTarget::Pad(pad_id) => {
                                return self.on_pad_selected(pad_id, hwnd)
                            },
                            _ => {}
                        }
                    },
                    _ => {
                        log::warn!("UI layer received state machine operation - should be handled at higher level");
                    }
                }
            }
        }
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

    fn post_board_command_msg(&self, hwnd: HWND, pad_id: PadId) {
        let hwnd_val = hwnd.0 as usize;
        unsafe {
            PostMessageW(
                Some(HWND(hwnd_val as *mut c_void)),
                WM_BOARD_COMMAND,
                WPARAM(pad_id.as_keypad_int() as usize),
                LPARAM(0)
            ).unwrap_or_default();
        }
    }

    fn post_layout_update_msg(&self, hwnd: HWND) {
        let hwnd_val = hwnd.0 as usize;
        unsafe {
            PostMessageW(
                Some(HWND(hwnd_val as *mut c_void)),
                WM_UPDATE_LAYOUT,
                WPARAM(0),
                LPARAM(0)
            ).unwrap_or_default();
        }
    }

    fn post_child_window_message(&self, hwnd: HWND, request: ChildWindowRequest) {
        let ptr = Box::into_raw(Box::new(request)) as usize;
        unsafe {
            PostMessageW(
                Some(hwnd),
                WM_SHOW_CHILD_WINDOW,
                WPARAM(ptr),
                LPARAM(0)
            ).unwrap_or_default();
        }
    }

    fn decode_child_window_message(wparam: WPARAM) -> ChildWindowRequest {
        let ptr = wparam.0 as *mut ChildWindowRequest;
        let request = unsafe { Box::from_raw(ptr) };
        *request
    }

    fn on_pad_selected(&mut self, pad_id: PadId, hwnd: HWND) -> LRESULT {
        let pad = self.board.as_ref().data().padset(Some(self.modifier_state)).pad(pad_id);
        if !pad.data.is_interactive() {
            return LRESULT(0);
        }

        if self.feedback == 0 {
            self.post_board_command_msg(hwnd, pad_id);
            return LRESULT(0);
        }

        self.selected_pad = Some(pad_id);
        self.invalidate(hwnd);
        self.set_timer(hwnd, ID_TIMER_FEEDBACK, self.feedback as f64 / 1000.0);
        LRESULT(0)
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

    fn stop_timeout_timer(&mut self, hwnd: HWND) {
        if self.timeout > 0 {
            unsafe { let _ = KillTimer(Some(hwnd), ID_TIMER_TIMEOUT); }
            self.timeout = 0;
            self.invalidate(hwnd);
        }
    }

    fn invalidate(&self, hwnd: HWND) -> LRESULT {
        unsafe {
            LRESULT(!InvalidateRect(Some(hwnd), None, true).as_bool() as isize)
        }
    }

    fn move_or_size(&mut self, hwnd: HWND, action: SetWindowPosCommand) -> LRESULT {
        let step = 10;
        match action {
            SetWindowPosCommand::Move(dir) => {
                let width = self.layout.rect.width();
                let height = self.layout.rect.height();
                match dir {
                    Direction::Left => self.layout.rect.left -= step,
                    Direction::Right => self.layout.rect.left += step,
                    Direction::Up => self.layout.rect.top -= step,
                    Direction::Down => self.layout.rect.top += step,
                }
                self.layout.rect.right = self.layout.rect.left + width;
                self.layout.rect.bottom = self.layout.rect.top + height;
            },
            SetWindowPosCommand::Size(dir) => {
                match dir {
                    Direction::Left => self.layout.rect.right -= step,
                    Direction::Right => self.layout.rect.right += step,
                    Direction::Up => self.layout.rect.bottom -= step,
                    Direction::Down => self.layout.rect.bottom += step,
                }
            }
        }
        if let Ok(rect) = self.layout.get_adjusted_rect() {
            unsafe { set_window_rect(hwnd, &rect); }
            self.post_layout_update_msg(hwnd);
        }
        LRESULT(0)
    }

    fn reset_window_pos(&self, hwnd: HWND, arg: bool)  {
        unsafe { reset_window_pos(hwnd, arg) };
    }

    pub fn redraw(&self) {
        self.invalidate(self.hwnd);
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

        match msg {
            WM_CREATE => {
                Some(self.on_create(hwnd, msg, wparam, lparam))
            },
            WM_PAINT => {
                Some(self.on_paint(hwnd))
            },
            WM_SYSKEYDOWN => {
                let vk_code = VIRTUAL_KEY(wparam.0 as u16);
                if ModifierHandler::is_modifier(vk_code) {
                    Some(self.on_keydown(hwnd, wparam))
                } else {
                    None // Let system handle non-ALT system keys
                }
            },
            WM_SYSKEYUP => {
                let vk_code = VIRTUAL_KEY(wparam.0 as u16);
                if ModifierHandler::is_modifier(vk_code) {
                    Some(self.on_keyup(hwnd, wparam))
                } else {
                    None
                }
            },
            WM_KEYDOWN => {
                Some(self.on_keydown(hwnd, wparam))
            },
            WM_KEYUP => {
                Some(self.on_keyup(hwnd, wparam))
            },
            WM_RBUTTONDOWN | WM_LBUTTONDOWN => {
                Some(self.on_right_mouse_down(hwnd, wparam, lparam))
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
            WM_SHOW_CHILD_WINDOW => {
                let child_request = Self::decode_child_window_message(wparam);
                if let Some(handler) = self.board.as_mut().handler() {
                    match handler.create_child_window(child_request, hwnd) {
                        UiEventResult::RequiresRedraw => {
                            self.invalidate(hwnd);
                        }
                        _ => {}
                    }
                }
                self.reset_window_pos(self.hwnd, false);
                Some(LRESULT(0))
            },
            _ => None,
        }
    }
}

fn loword(lparam: LPARAM) -> i32 { (lparam.0 as usize & 0xffff) as i32 }
fn hiword(lparam: LPARAM) -> i32 { ((lparam.0 as usize >> 16) & 0xffff) as i32 }


struct EventMapper;

impl EventMapper {
    fn key_down(vk_code: VIRTUAL_KEY, modifiers: ModifierState) -> UiEvent {
        UiEvent::KeyDown(KeyboardEvent {
            key: vk_code.0 as u32,
            modifiers,
        })
    }

    fn key_up(vk_code: VIRTUAL_KEY, modifiers: ModifierState) -> UiEvent {
        UiEvent::KeyUp(KeyboardEvent {
            key: vk_code.0 as u32,
            modifiers,
        })
    }

    fn right_mouse_down(target: MouseEventTarget, modifier_state: ModifierState) -> UiEvent {
        UiEvent::RightMouseDown(MouseEvent {
            target,
            modifiers: modifier_state,
        })
    }
}

trait MouseEventTargetable {
    fn hit_test(&self, x: i32, y: i32) -> Option<MouseEventTarget>;
}

impl MouseEventTargetable for WindowLayout {
    fn hit_test(&self, x: i32, y: i32) -> Option<MouseEventTarget> {
        let rect = self.get_adjusted_rect().ok()?;
        // X, Y are relative to the window (compare against width and height)

        if x > rect.width() && y > rect.height() {
            return None; // Outside window
        }

        // top 10% height is header
        // rest is 3x3 grid of pads (1,2,3 bottom row)

        let width = rect.width();
        let height = rect.height();
        let header_height = height / 10;
        if y < header_height {
            return Some(MouseEventTarget::Header);
        }
        let pad_height = (height - header_height) / 3;
        let pad_width = width / 3;

        // Check which pad was clicked
        let pad_x = x / pad_width;
        let pad_y = 2 - ((y - header_height) / pad_height);
        if pad_x >= 0 && pad_x < 3 && pad_y >= 0 && pad_y < 3 {
            return Some(MouseEventTarget::Pad(PadId::from_keypad_int(pad_x + pad_y * 3 + 1)));
        }

        None
    }
}
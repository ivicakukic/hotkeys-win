use std::mem;
use std::sync::Once;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::Graphics::Gdi::{InvalidateRect, UpdateWindow, COLOR_BTNFACE, HBRUSH};
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;


// Control IDs
const ID_DISPLAY_TEXT: u16 = 1001;
const ID_CLOSE_BUTTON: u16 = 1002;

// Window class registration protection
static REGISTER_CHORD_DIALOG_CLASS: Once = Once::new();
const CHORD_DIALOG_CLASS_NAME: &str = "ChordCaptureDialogClass";

use crate::input::{capture, ModifierHandler, ModifierState};

pub struct ShortcutCaptureDialog {
    hwnd: HWND,
    display_hwnd: HWND,
    is_closed: bool,
    is_stopped: bool,
    is_cancelled: bool,
    modifiers: ModifierState,
    capture: capture::KeyCombinationCapture,
}

impl ShortcutCaptureDialog {

    pub fn new() -> Self {
        Self {
            hwnd: HWND::default(),
            display_hwnd: HWND::default(),
            is_closed: false,
            is_stopped: false,
            is_cancelled: false,
            modifiers: ModifierState::default(),
            capture: capture::KeyCombinationCapture::new(),
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.is_cancelled || self.capture.last_record().is_none()
    }

    pub fn show_modal(&mut self, parent: Option<HWND>) {
        unsafe {
            let instance = GetModuleHandleW(None).unwrap();

            // Register window class
            Self::register_window_class(instance);

            // Calculate position (centered on screen)
            let dialog_width = 460;
            let dialog_height = 110;
            let screen_width = GetSystemMetrics(SM_CXSCREEN);
            let screen_height = GetSystemMetrics(SM_CYSCREEN);
            let x = (screen_width - dialog_width) / 2;
            let y = (screen_height - dialog_height) / 2;

            // Create dialog window
            let class_name = to_wide_string(CHORD_DIALOG_CLASS_NAME);
            self.hwnd = CreateWindowExW(
                WS_EX_DLGMODALFRAME | WS_EX_WINDOWEDGE,
                PCWSTR::from_raw(class_name.as_ptr()),
                w!("Recording...  press Esc to finish"),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                x,
                y,
                dialog_width,
                dialog_height,
                parent,
                None,
                Some(instance.into()),
                None,
            ).unwrap();

            // Store self pointer
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, self as *mut ShortcutCaptureDialog as isize);

            // Create controls
            self.create_controls();

            // Update initial display
            self.update_display();

            // Message loop
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() && !self.is_closed {
                let _ = TranslateMessage(&msg);
                let _ = DispatchMessageW(&msg);
            }
        }
    }

    fn register_window_class(instance: HMODULE) {
        REGISTER_CHORD_DIALOG_CLASS.call_once(|| {
            let class_name = to_wide_string(CHORD_DIALOG_CLASS_NAME);
            let wc = WNDCLASSEXW {
                cbSize: mem::size_of::<WNDCLASSEXW>() as u32,
                style: CS_HREDRAW | CS_VREDRAW,
                lpfnWndProc: Some(Self::window_proc),
                hInstance: instance.into(),
                hCursor: unsafe { LoadCursorW(None, IDC_ARROW).unwrap() },
                hbrBackground: HBRUSH((COLOR_BTNFACE.0 + 1) as *mut _),
                lpszClassName: PCWSTR::from_raw(class_name.as_ptr()),
                ..Default::default()
            };

            unsafe {
                let result = RegisterClassExW(&wc);
                assert!(result != 0, "Failed to register window class");
            }
        });
    }

    unsafe fn create_controls(&mut self) {
        let instance = GetModuleHandleW(None).unwrap();

        // Display area (large read-only edit control with custom font)
        self.display_hwnd = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            w!("EDIT"),
            w!("(empty)"),
            WS_CHILD | WS_CHILDWINDOW | WS_DISABLED | WS_VISIBLE | WINDOW_STYLE(ES_READONLY as _) | WINDOW_STYLE(ES_CENTER as _),
            15, 20, 300, 30, // Made height a bit larger
            Some(self.hwnd),
            Some(HMENU(ID_DISPLAY_TEXT as _)),
            Some(instance.into()),
            None,
        ).unwrap();

        // Create and set a larger font
        use windows::Win32::Graphics::Gdi::*;
        let hfont = CreateFontW(
            22,                             // Height (larger font)
            0,                              // Width (0 = default)
            0,                              // Escapement
            0,                              // Orientation
            600,                            // Weight (FW_NORMAL)
            0,                              // Italic
            0,                              // Underline
            0,                              // StrikeOut
            FONT_CHARSET(1),               // CharSet (DEFAULT_CHARSET)
            FONT_OUTPUT_PRECISION(0),      // OutPrecision
            FONT_CLIP_PRECISION(0),        // ClipPrecision
            FONT_QUALITY(0),               // Quality
            0,                              // PitchAndFamily
            w!("Segoe UI"),                // Font name
        );

        // CreateFontW returns HFONT directly, not Result
        use windows::Win32::UI::WindowsAndMessaging::*;
        let _ = SendMessageW(self.display_hwnd, WM_SETFONT, Some(WPARAM(hfont.0 as usize)), Some(LPARAM(1)));

        // Close button
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("Close"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as _),
            330, 20, 100, 30, // Adjusted Y position to center with taller text field
            Some(self.hwnd),
            Some(HMENU(ID_CLOSE_BUTTON as _)),
            Some(instance.into()),
            None,
        );
    }

    unsafe fn update_display(&self) {
        use capture::DisplayFormatable;
        let display_format = capture::DisplayFormats::InverseSpaced;
        let current_capture = self.capture.get_current_capture();
        let display_text = current_capture.display_format(&display_format.get_format());

        // let display_text = self.text_capture.text().unwrap_or("(empty)".to_string());
        // println!("Display text: {}", display_text);

        let wide_text = to_wide_string(&display_text);
        let _ = SetWindowTextW(self.display_hwnd, PCWSTR::from_raw(wide_text.as_ptr()));

        // force redraw
        let _ = InvalidateRect(Some(self.display_hwnd), None, true);

    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_CREATE => LRESULT(0),

            WM_COMMAND => {
                let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ShortcutCaptureDialog;
                if dialog.is_null() {
                    return LRESULT(0);
                }

                let id = (wparam.0 & 0xFFFF) as u16;
                match id {
                    ID_CLOSE_BUTTON => {
                        (*dialog).is_closed = true;
                        let _ = DestroyWindow(hwnd);
                    }
                    _ => {}
                }
                LRESULT(0)
            }

            WM_KEYDOWN => {
                let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ShortcutCaptureDialog;
                if !dialog.is_null() {
                    (*dialog).on_keydown(hwnd, wparam);
                }
                LRESULT(0)
            }

            WM_KEYUP => {
                let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ShortcutCaptureDialog;
                if !dialog.is_null() {
                    (*dialog).on_keyup(hwnd, wparam);
                }
                LRESULT(0)
            }

            WM_SYSKEYDOWN => {
                let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ShortcutCaptureDialog;
                if !dialog.is_null() {
                    let vk_code = VIRTUAL_KEY(wparam.0 as u16);
                    if capture::ModifierHandler::is_modifier(vk_code) {
                        (*dialog).on_keydown(hwnd, wparam);
                        return LRESULT(0);
                    }
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            },
            WM_SYSKEYUP => {
                let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ShortcutCaptureDialog;
                if !dialog.is_null() {
                    let vk_code = VIRTUAL_KEY(wparam.0 as u16);
                    if capture::ModifierHandler::is_modifier(vk_code) {
                        (*dialog).on_keyup(hwnd, wparam);
                        return LRESULT(0);
                    }
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            },

            WM_DESTROY => LRESULT(0),

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    fn on_keyup(&mut self, _hwnd: HWND, wparam: WPARAM) -> LRESULT {
        let vk_code = VIRTUAL_KEY(wparam.0 as u16);

        // Handle modifier keys first
        let mut modifier_handler = ModifierHandler::new(self.modifiers.clone());

        if modifier_handler.handle_key_release(vk_code) {
            let new_state = modifier_handler.state().clone();
            if new_state == self.modifiers {
                return LRESULT(0);
            }
            self.modifiers = new_state;
        }
        self.capture.on_keyup(wparam, self.modifiers.clone());
        LRESULT(0)
    }

    fn on_keydown(&mut self, _hwnd: HWND, wparam: WPARAM) -> LRESULT {
        if ! self.is_stopped {

            let vk_code = VIRTUAL_KEY(wparam.0 as u16);

            // Handle modifier keys first
            let mut modifier_handler = ModifierHandler::new(self.modifiers.clone());

            if modifier_handler.handle_key_press(vk_code) {
                let new_state = modifier_handler.state().clone();
                if new_state == self.modifiers {
                    return LRESULT(0);
                }
                self.modifiers = new_state;
            }

            self.capture.on_keydown(wparam, self.modifiers.clone());
            self.update_closed_state();
            unsafe { self.update_display() };
            LRESULT(0)
        } else {

            let vk_code = VIRTUAL_KEY(wparam.0 as u16);

            match vk_code {
                VK_ESCAPE  => {
                    self.is_closed = true;
                    self.is_cancelled = true;
                    unsafe { let _ = DestroyWindow(self.hwnd); }
                    LRESULT(0)
                },
                VK_RETURN | VK_SPACE => {
                    self.is_closed = true;
                    unsafe { let _ = DestroyWindow(self.hwnd); }
                    LRESULT(0)
                },
                _ => LRESULT(0)
            }

        }

    }

    fn update_closed_state(&mut self) {
        if self.is_closed {
            return;
        }

        let record = self.capture.last_record();
        if let Some(last) = record {
            if last.key == Some(VK_ESCAPE.0) && last.modifiers.is_none() {
                self.is_stopped = true;
                self.capture.remove_last_record();
                self.capture.deactivate_record();

                unsafe {
                    // change window title
                    let (new_title, new_button_text) = if self.capture.last_record().is_none() {
                        ("Canceled", "Close")
                    } else {
                        ("Done - press Enter to confirm or Esc to cancel", "Confirm")
                    };
                    let new_title = to_wide_string(new_title);
                    let _ = SetWindowTextW(self.hwnd, PCWSTR::from_raw(new_title.as_ptr()));
                    let _ = UpdateWindow(self.hwnd);

                    // change button text
                    let button_control = GetDlgItem(Some(self.hwnd), ID_CLOSE_BUTTON as i32);
                    if let Ok(control) = button_control {
                        let new_button_text = to_wide_string(new_button_text);
                        let _ = SetWindowTextW(control, PCWSTR::from_raw(new_button_text.as_ptr()));
                        let _ = UpdateWindow(control);
                    }
                }
            }
        }
    }

    pub fn get_current_capture(&self) -> Vec<capture::Combination> {
        if self.is_cancelled {
            vec![]
        } else {
            self.capture.get_current_capture()
        }
    }
}

// Helper functions
fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}


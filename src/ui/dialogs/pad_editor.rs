use std::mem;
use std::sync::Once;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::Graphics::Gdi::{HBRUSH, COLOR_BTNFACE};

use crate::input::capture::{self, DisplayFormatable};
use crate::core::integration::ActionType;
use crate::model::Pad;

// Control IDs
const ID_HEADER_EDIT: u16 = 1001;
const ID_TEXT_EDIT: u16 = 1002;
const ID_ACTIONS_LIST: u16 = 1003;
const ID_ACTION_TYPE_COMBO: u16 = 1004;
const ID_ACTION_VALUE_EDIT: u16 = 1005;
const ID_ADD_ACTION: u16 = 1006;
const ID_DELETE_ACTION: u16 = 1007;
const ID_UPDATE_ACTION: u16 = 1008;
const ID_CAPTURE_SHORTCUT: u16 = 1009;
const IDOK: u16 = 1;
const IDCANCEL: u16 = 2;

// Window class registration protection
static REGISTER_PAD_DIALOG_CLASS: Once = Once::new();
const PAD_DIALOG_CLASS_NAME: &str = "PadDialogClass";

// Custom message IDs
const WM_CAPTURE_SHORTCUT: u32 = WM_USER + 1;

struct PadEditor {
    hwnd: HWND,
    pad: Pad,
    actions: Vec<ActionType>, // Store actions separately for editing
    result: DialogResult,
    // Store final data after dialog closes
    final_header: String,
    final_text: String,
}

#[derive(Debug, Clone, PartialEq)]
enum DialogResult {
    Ok,
    Cancel,
    None,
}

impl PadEditor {
    fn new(pad: Pad) -> Self {
        let actions = pad.actions().clone();
        Self {
            hwnd: HWND::default(),
            pad,
            actions,
            result: DialogResult::None,
            final_header: String::new(),
            final_text: String::new(),
        }
    }

    /// Register window class once using `Once` to ensure one-time initialization
    fn register_window_class(instance: HMODULE) {
        REGISTER_PAD_DIALOG_CLASS.call_once(|| {
            let class_name = to_wide_string(PAD_DIALOG_CLASS_NAME);
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

    fn show_modal(&mut self, parent: Option<HWND>) -> DialogResult {
        unsafe {
            let instance = GetModuleHandleW(None).unwrap();

            // Register window class (protected by Once)
            Self::register_window_class(instance);

            // Calculate dialog position relative to parent
            let (dialog_x, dialog_y) = if let Some(parent_hwnd) = parent {
                let mut parent_rect = windows::Win32::Foundation::RECT::default();
                if windows::Win32::UI::WindowsAndMessaging::GetWindowRect(parent_hwnd, &mut parent_rect).is_ok() {
                    let parent_width = parent_rect.right - parent_rect.left;
                    let parent_height = parent_rect.bottom - parent_rect.top;
                    let dialog_width = 600;
                    let dialog_height = 445;

                    // Center dialog on parent
                    let x = parent_rect.left + (parent_width - dialog_width) / 2;
                    let y = parent_rect.top + (parent_height - dialog_height) / 2;
                    (x, y)
                } else {
                    (CW_USEDEFAULT, CW_USEDEFAULT)
                }
            } else {
                (CW_USEDEFAULT, CW_USEDEFAULT)
            };

            // Create dialog window
            let class_name = to_wide_string(PAD_DIALOG_CLASS_NAME);
            self.hwnd = CreateWindowExW(
                WS_EX_DLGMODALFRAME,
                PCWSTR::from_raw(class_name.as_ptr()),
                w!("Pad Editor"),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                dialog_x,
                dialog_y,
                600,
                445,
                parent,
                None,
                Some(instance.into()),
                None,
            ).unwrap();

            // Set up dialog pointer and create controls after window is created
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, self as *mut _ as _);
            self.create_controls();

            // Set initial focus to first control
            let first_control = GetNextDlgTabItem(self.hwnd, None, false);
            if let Ok(control) = first_control {
                let _ = SetFocus(Some(control));
            }

            // Message loop with dialog message processing
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                // Use IsDialogMessage to handle tab navigation automatically
                if !IsDialogMessageW(self.hwnd, &msg).as_bool() {
                    let _ = TranslateMessage(&msg);
                    let _ = DispatchMessageW(&msg);
                }

                // Check if dialog was closed
                if self.result != DialogResult::None {
                    break;
                }
            }

            self.result.clone()
        }
    }

    fn get_updated_pad(&self) -> Pad {
        // Create new core pad with updated data
        let core_pad = crate::core::data::Pad {
            header: Some(self.final_header.clone()),
            text: Some(self.final_text.clone()),
            icon: {
                let icon_str = self.pad.icon();
                if icon_str.is_empty() { None } else { Some(icon_str) }
            },
            actions: self.actions.clone(),
            board: self.pad.board(),
            board_params: self.pad.board_params().clone(),
            color_scheme: self.pad.color_scheme.as_ref().map(|cs| cs.name.clone()),
            text_style: self.pad.text_style.as_ref().map(|ts| ts.name.clone()),
        };

        // Create new model pad
        Pad::new(
            self.pad.pad_id(),
            core_pad,
            self.pad.color_scheme.clone(),
            self.pad.text_style.clone(),
            self.pad.tags().clone(),
        )
    }

    unsafe fn create_controls(&mut self) {
        let instance = GetModuleHandleW(None).unwrap();

        // Header label and edit
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Header:"),
            WS_CHILD | WS_VISIBLE,
            10, 10, 80, 20,
            Some(self.hwnd),
            None,
            Some(instance.into()),
            None,
        );

        let _ = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            w!("EDIT"),
            PCWSTR::from_raw(to_wide_string(&self.pad.header()).as_ptr()),
            WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as _),
            100, 10, 475, 25,
            Some(self.hwnd),
            Some(HMENU(ID_HEADER_EDIT as _)),
            Some(instance.into()),
            None,
        );

        // Text label and edit (multiline)
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Text:"),
            WS_CHILD | WS_VISIBLE,
            10, 45, 80, 20,
            Some(self.hwnd),
            None,
            Some(instance.into()),
            None,
        );

        let _ = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            w!("EDIT"),
            PCWSTR::from_raw(to_wide_string(&newline_to_backslash_n(&self.pad.text())).as_ptr()),
            WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as _),
            100, 45, 475, 25,
            Some(self.hwnd),
            Some(HMENU(ID_TEXT_EDIT as _)),
            Some(instance.into()),
            None,
        );

        // Actions label
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Actions:"),
            WS_CHILD | WS_VISIBLE,
            10, 85, 80, 20,
            Some(self.hwnd),
            None,
            Some(instance.into()),
            None,
        );

        // Actions listbox
        let _ = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            w!("LISTBOX"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(LBS_NOTIFY as _),
            100, 85, 475, 120,
            Some(self.hwnd),
            Some(HMENU(ID_ACTIONS_LIST as _)),
            Some(instance.into()),
            None,
        );

        // Action type combo
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Action Type:"),
            WS_CHILD | WS_VISIBLE,
            10, 220, 80, 20,
            Some(self.hwnd),
            None,
            Some(instance.into()),
            None,
        );

        let combo = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("COMBOBOX"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(CBS_DROPDOWNLIST as _),
            100, 215, 100, 200,
            Some(self.hwnd),
            Some(HMENU(ID_ACTION_TYPE_COMBO as _)),
            Some(instance.into()),
            None,
        ).unwrap();

        // Add action types to combo
        for action_type in ["Shortcut", "Text", "Line", "Paste", "PasteEnter", "Pause", "OpenUrl"] {
            let wide = to_wide_string(action_type);
            SendMessageW(combo, CB_ADDSTRING, Some(WPARAM(0)), Some(LPARAM(wide.as_ptr() as _)));
        }
        SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(0)), Some(LPARAM(0)));

        // Action value edit
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("STATIC"),
            w!("Value:"),
            WS_CHILD | WS_VISIBLE,
            210, 220, 50, 20,
            Some(self.hwnd),
            None,
            Some(instance.into()),
            None,
        );

        let _ = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            w!("EDIT"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP | WINDOW_STYLE(ES_AUTOHSCROLL as _),
            270, 215, 305, 25,
            Some(self.hwnd),
            Some(HMENU(ID_ACTION_VALUE_EDIT as _)),
            Some(instance.into()),
            None,
        );

        // Action buttons
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("Add"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as _),
            100, 255, 80, 30,
            Some(self.hwnd),
            Some(HMENU(ID_ADD_ACTION as _)),
            Some(instance.into()),
            None,
        );

        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("Update"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as _),
            190, 255, 80, 30,
            Some(self.hwnd),
            Some(HMENU(ID_UPDATE_ACTION as _)),
            Some(instance.into()),
            None,
        );

        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("Delete"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as _),
            280, 255, 80, 30,
            Some(self.hwnd),
            Some(HMENU(ID_DELETE_ACTION as _)),
            Some(instance.into()),
            None,
        );

        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("Capture Shortcut"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as _),
            370, 255, 120, 30,
            Some(self.hwnd),
            Some(HMENU(ID_CAPTURE_SHORTCUT as _)),
            Some(instance.into()),
            None,
        );

        // OK/Cancel buttons
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("OK"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as _),
            210, 365, 80, 30,
            Some(self.hwnd),
            Some(HMENU(IDOK as _)),
            Some(instance.into()),
            None,
        );

        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("Cancel"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as _),
            310, 365, 80, 30,
            Some(self.hwnd),
            Some(HMENU(IDCANCEL as _)),
            Some(instance.into()),
            None,
        );

        // Populate actions list
        self.refresh_actions_list();
    }

    unsafe fn refresh_actions_list(&self) {
        let list = GetDlgItem(Some(self.hwnd), ID_ACTIONS_LIST as _).unwrap();
        SendMessageW(list, LB_RESETCONTENT, Some(WPARAM(0)), Some(LPARAM(0)));

        for action in &self.actions {
            let text = format_action_type(action);
            let wide_text = to_wide_string(&text);
            SendMessageW(list, LB_ADDSTRING, Some(WPARAM(0)), Some(LPARAM(wide_text.as_ptr() as _)));
        }
    }

    unsafe fn add_action(&mut self) {
        let combo = GetDlgItem(Some(self.hwnd), ID_ACTION_TYPE_COMBO as _).unwrap();
        let edit = GetDlgItem(Some(self.hwnd), ID_ACTION_VALUE_EDIT as _).unwrap();

        let sel = SendMessageW(combo, CB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))).0;
        let value = get_window_text(edit);

        let action = match sel {
            0 => ActionType::Shortcut(value),
            1 => ActionType::Text(backslash_n_to_newline(&value)),
            2 => ActionType::Line(backslash_n_to_newline(&value)),
            3 => ActionType::Paste(backslash_n_to_newline(&value)),
            4 => ActionType::PasteEnter(backslash_n_to_newline(&value)),
            5 => ActionType::Pause(value.parse().unwrap_or(1000)),
            6 => ActionType::OpenUrl(value), // Using OpenUrl instead of Board for now
            _ => return,
        };

        self.actions.push(action);
        self.refresh_actions_list();
        let _ = SetWindowTextW(edit, w!(""));
    }

    unsafe fn delete_action(&mut self) {
        let list = GetDlgItem(Some(self.hwnd), ID_ACTIONS_LIST as _).unwrap();
        let sel = SendMessageW(list, LB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))).0 as usize;

        if sel != LB_ERR as usize && sel < self.actions.len() {
            self.actions.remove(sel);
            self.refresh_actions_list();
        }
    }

    unsafe fn update_action(&mut self) {
        let list = GetDlgItem(Some(self.hwnd), ID_ACTIONS_LIST as _).unwrap();
        let combo = GetDlgItem(Some(self.hwnd), ID_ACTION_TYPE_COMBO as _).unwrap();
        let edit = GetDlgItem(Some(self.hwnd), ID_ACTION_VALUE_EDIT as _).unwrap();

        let sel = SendMessageW(list, LB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))).0 as usize;
        if sel == LB_ERR as usize || sel >= self.actions.len() {
            return;
        }

        let type_sel = SendMessageW(combo, CB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))).0;
        let value = get_window_text(edit);

        let action = match type_sel {
            0 => ActionType::Shortcut(value),
            1 => ActionType::Text(backslash_n_to_newline(&value)),
            2 => ActionType::Line(backslash_n_to_newline(&value)),
            3 => ActionType::Paste(backslash_n_to_newline(&value)),
            4 => ActionType::PasteEnter(backslash_n_to_newline(&value)),
            5 => ActionType::Pause(value.parse().unwrap_or(1000)),
            6 => ActionType::OpenUrl(value), // Using OpenUrl instead of Board for now
            _ => return,
        };

        self.actions[sel] = action;
        self.refresh_actions_list();
    }

    unsafe fn load_selected_action(&self) {
        let list = GetDlgItem(Some(self.hwnd), ID_ACTIONS_LIST as _).unwrap();
        let combo = GetDlgItem(Some(self.hwnd), ID_ACTION_TYPE_COMBO as _).unwrap();
        let edit = GetDlgItem(Some(self.hwnd), ID_ACTION_VALUE_EDIT as _).unwrap();

        let sel = SendMessageW(list, LB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))).0 as usize;
        if sel == LB_ERR as usize || sel >= self.actions.len() {
            return;
        }

        let action = &self.actions[sel];

        // Set the combo box selection and edit text based on action type
        match action {
            ActionType::Shortcut(keys) => {
                SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(0)), Some(LPARAM(0)));
                let _ = SetWindowTextW(edit, PCWSTR::from_raw(to_wide_string(keys).as_ptr()));
            }
            ActionType::Text(content) => {
                SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(1)), Some(LPARAM(0)));
                let _ = SetWindowTextW(edit, PCWSTR::from_raw(to_wide_string(&newline_to_backslash_n(content)).as_ptr()));
            }
            ActionType::Line(content) => {
                SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(2)), Some(LPARAM(0)));
                let _ = SetWindowTextW(edit, PCWSTR::from_raw(to_wide_string(&newline_to_backslash_n(content)).as_ptr()));
            }
            ActionType::Paste(content) => {
                SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(3)), Some(LPARAM(0)));
                let _ = SetWindowTextW(edit, PCWSTR::from_raw(to_wide_string(&newline_to_backslash_n(content)).as_ptr()));
            }
            ActionType::PasteEnter(content) => {
                SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(4)), Some(LPARAM(0)));
                let _ = SetWindowTextW(edit, PCWSTR::from_raw(to_wide_string(&newline_to_backslash_n(content)).as_ptr()));
            }
            ActionType::Pause(duration) => {
                SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(5)), Some(LPARAM(0)));
                let _ = SetWindowTextW(edit, PCWSTR::from_raw(to_wide_string(&duration.to_string()).as_ptr()));
            }
            ActionType::OpenUrl(url) => {
                SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(6)), Some(LPARAM(0)));
                let _ = SetWindowTextW(edit, PCWSTR::from_raw(to_wide_string(url).as_ptr()));
            }
            ActionType::Custom(_) => {
                // Handle custom action type if needed
            }
        }
    }

    unsafe fn save_data(&mut self) {
        // Save data while window is still valid
        self.final_header = get_window_text(GetDlgItem(Some(self.hwnd), ID_HEADER_EDIT as _).unwrap());
        self.final_text = backslash_n_to_newline(&get_window_text(GetDlgItem(Some(self.hwnd), ID_TEXT_EDIT as _).unwrap()));
    }

    unsafe fn set_current_capture(&self, capture: Vec<capture::Combination>) {
        let capture = capture.display_format(&capture::DisplayFormats::InverseSpaced.get_format());
        let edit = GetDlgItem(Some(self.hwnd), ID_ACTION_VALUE_EDIT as _).unwrap();
        let _ = SetWindowTextW(edit, PCWSTR::from_raw(to_wide_string(&capture).as_ptr()));
        // Set action type to Shortcut
        let combo = GetDlgItem(Some(self.hwnd), ID_ACTION_TYPE_COMBO as _).unwrap();
        SendMessageW(combo, CB_SETCURSEL, Some(WPARAM(0)), Some(LPARAM(0)));
    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match msg {
            WM_CREATE => {
                LRESULT(0)
            }
            WM_COMMAND => {
                let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PadEditor;
                if dialog.is_null() {
                    return LRESULT(0);
                }

                let id = (wparam.0 & 0xFFFF) as u16;
                let notification = ((wparam.0 >> 16) & 0xFFFF) as u16;

                match id {
                    IDOK => {
                        (*dialog).save_data();
                        (*dialog).result = DialogResult::Ok;
                        let _ = DestroyWindow(hwnd);
                    }
                    IDCANCEL => {
                        (*dialog).result = DialogResult::Cancel;
                        let _ = DestroyWindow(hwnd);
                    }
                    ID_ADD_ACTION => (*dialog).add_action(),
                    ID_DELETE_ACTION => (*dialog).delete_action(),
                    ID_UPDATE_ACTION => (*dialog).update_action(),
                    ID_ACTIONS_LIST => {
                        if notification == LBN_DBLCLK as u16 {
                            (*dialog).load_selected_action();
                        }
                    }
                    ID_CAPTURE_SHORTCUT => {
                        // Open shortcut capture dialog
                        let _ = PostMessageW(Some(hwnd), WM_CAPTURE_SHORTCUT, WPARAM(0), LPARAM(0));
                    }
                    _ => {}
                }
                LRESULT(0)
            }
            WM_KEYDOWN => {
                let vk_code = wparam.0 as u16;

                if vk_code == VK_ESCAPE.0 {
                    let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PadEditor;
                    if !dialog.is_null() {
                        (*dialog).result = DialogResult::Cancel;
                        let _ = DestroyWindow(hwnd);
                    }
                }
                // Tab navigation is now handled by IsDialogMessageW in the message loop
                LRESULT(0)
            }
            WM_DESTROY => {
                // Don't call PostQuitMessage(0) here - we only want to exit the dialog's message loop,
                // not quit the entire application. The message loop will exit when self.result != DialogResult::None
                LRESULT(0)
            }
            WM_CAPTURE_SHORTCUT => {
                let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PadEditor;
                if dialog.is_null() {
                    return LRESULT(0);
                }

                // Open the shortcut capture dialog
                let mut capture_dialog = crate::ui::dialogs::capture_dialog::ShortcutCaptureDialog::new();
                capture_dialog.show_modal(None);

                if !capture_dialog.is_cancelled() {
                    let capture = capture_dialog.get_current_capture();
                    (*dialog).set_current_capture(capture);
                }
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

}


pub fn open_pad_editor(pad: Pad, parent: Option<HWND>) -> Option<Pad> {
    let mut editor = PadEditor::new(pad);
    let result = editor.show_modal(parent);
    if result == DialogResult::Ok {
        Some(editor.get_updated_pad())
    } else {
        None
    }
}

// Helper functions
fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn newline_to_backslash_n(text: &str) -> String {
    text.replace('\n', "\\n")
}

fn backslash_n_to_newline(text: &str) -> String {
    text.replace("\\n", "\n")
}

unsafe fn get_window_text(hwnd: HWND) -> String {
    let len = GetWindowTextLengthW(hwnd) + 1;
    let mut buffer = vec![0u16; len as usize];
    GetWindowTextW(hwnd, &mut buffer);
    String::from_utf16_lossy(&buffer).trim_end_matches('\0').to_string()
}

fn format_action_type(action: &ActionType) -> String {
    match action {
        ActionType::Shortcut(keys) => format!("Shortcut: {}", keys),
        ActionType::Text(content) => format!("Text: {}", content),
        ActionType::Line(content) => format!("Line: {}", content),
        ActionType::Pause(duration) => format!("Pause: {}ms", duration),
        ActionType::OpenUrl(url) => format!("OpenUrl: {}", url),
        ActionType::Paste(text) => format!("Paste: {}", text),
        ActionType::PasteEnter(text) => format!("PasteEnter: {}", text),
        ActionType::Custom(params) => format!("Custom: {}", params.action_type),
    }
}
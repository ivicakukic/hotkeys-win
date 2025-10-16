use std::mem;
use std::sync::Once;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;
use windows::Win32::Graphics::Gdi::{HBRUSH, COLOR_BTNFACE};

// Control IDs
const ID_BOARDS_LIST: u16 = 1000;
const ID_BOARDS_COMBO: u16 = 1001;
const ID_ADD_BOARD: u16 = 1002;
const ID_DELETE_BOARD: u16 = 1003;
const IDOK: u16 = 1;
const IDCANCEL: u16 = 2;

// Window class registration protection
static REGISTER_CHAIN_DIALOG_CLASS: Once = Once::new();
const CHAIN_DIALOG_CLASS_NAME: &str = "ChainDialogClass";

struct ChainEditor {
    hwnd: HWND,
    chain_boards: Vec<String>,
    initial_board: String,
    all_boards: Vec<String>,
    result: DialogResult,
}

#[derive(Debug, Clone, PartialEq)]
enum DialogResult {
    Ok,
    Cancel,
    None,
}

impl ChainEditor {
    fn new(chain_boards: Vec<String>, initial_board: Option<String>, all_boards: Vec<String>) -> Self {
        Self {
            hwnd: HWND::default(),
            chain_boards,
            initial_board: initial_board.unwrap_or_default(),
            all_boards,
            result: DialogResult::None,
        }
    }

    /// Register window class once using `Once` to ensure one-time initialization
    fn register_window_class(instance: HMODULE) {
        REGISTER_CHAIN_DIALOG_CLASS.call_once(|| {
            let class_name = to_wide_string(CHAIN_DIALOG_CLASS_NAME);
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
            let class_name = to_wide_string(CHAIN_DIALOG_CLASS_NAME);
            self.hwnd = CreateWindowExW(
                WS_EX_DLGMODALFRAME,
                PCWSTR::from_raw(class_name.as_ptr()),
                w!("Collection Editor"),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                dialog_x,
                dialog_y,
                500,
                280,
                parent,
                None,
                Some(instance.into()),
                None,
            ).unwrap();

            // Set up dialog pointer and create controls after window is created
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, self as *mut _ as _);
            self.create_controls();

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

    unsafe fn create_controls(&mut self) {
        let instance = GetModuleHandleW(None).unwrap();

        // Boards listbox
        let _ = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            w!("LISTBOX"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_BORDER | WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(LBS_NOTIFY as _),
            10, 10, 465, 120,
            Some(self.hwnd),
            Some(HMENU(ID_BOARDS_LIST as _)),
            Some(instance.into()),
            None,
        );

        // Boards combo
        let combo = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("COMBOBOX"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(CBS_DROPDOWNLIST as _),
            10, 140, 290, 200,
            Some(self.hwnd),
            Some(HMENU(ID_BOARDS_COMBO as _)),
            Some(instance.into()),
            None,
        ).unwrap();

        // Add action types to combo
        for board in self.all_boards.iter() {
            let wide = to_wide_string(board.as_str());
            SendMessageW(combo, CB_ADDSTRING, Some(WPARAM(0)), Some(LPARAM(wide.as_ptr() as _)));
        }

        // Add/Delete buttons
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("Add"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as _),
            308, 138, 80, 30,
            Some(self.hwnd),
            Some(HMENU(ID_ADD_BOARD as _)),
            Some(instance.into()),
            None,
        );

        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("Delete"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as _),
            395, 138, 80, 30,
            Some(self.hwnd),
            Some(HMENU(ID_DELETE_BOARD as _)),
            Some(instance.into()),
            None,
        );




        // OK/Cancel buttons
        let _ = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("OK"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as _),
            170, 200, 80, 30,
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
            260, 200, 80, 30,
            Some(self.hwnd),
            Some(HMENU(IDCANCEL as _)),
            Some(instance.into()),
            None,
        );

        // Populate chain boards list
        self.ensure_initial_board_exists();
        self.refresh_chain_boards_list();
    }

    unsafe fn refresh_chain_boards_list(&mut self) {
        let list = GetDlgItem(Some(self.hwnd), ID_BOARDS_LIST as _).unwrap();
        SendMessageW(list, LB_RESETCONTENT, Some(WPARAM(0)), Some(LPARAM(0)));

        let mut initial_board_found = false;
        for board in &self.chain_boards {
            let mut text = board.clone();
            if !self.all_boards.contains(board) {
                text = format!("{} (missing)", text);
            }
            if board == &self.initial_board && !initial_board_found {
                text = format!("{} [initial]", text);
                initial_board_found = true;
            }
            let wide_text = to_wide_string(text.as_str());
            SendMessageW(list, LB_ADDSTRING, Some(WPARAM(0)), Some(LPARAM(wide_text.as_ptr() as _)));
        }
    }

    unsafe fn add_board(&mut self) {
        let combo = GetDlgItem(Some(self.hwnd), ID_BOARDS_COMBO as _).unwrap();
        let sel = SendMessageW(combo, CB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))).0 as usize;
        if sel == CB_ERR as usize {
            return;
        }

        if sel < self.all_boards.len() {
            let new_board = self.all_boards[sel].clone();
            if self.chain_boards.contains(&new_board) {
                // Prevent adding duplicate boards
                MessageBoxW(Some(self.hwnd), w!("This board is already in the list."), w!("Cannot Add Board"), MB_OK | MB_ICONWARNING);
                return;
            }
            self.chain_boards.push(new_board);
            self.refresh_chain_boards_list();
        }
    }

    unsafe fn delete_board(&mut self) {
        if self.chain_boards.len() == 1 {
            // Prevent deleting the last board
            MessageBoxW(Some(self.hwnd), w!("At least one board must remain in the list."), w!("Cannot Delete Board"), MB_OK | MB_ICONWARNING);
            return;
        }

        let list = GetDlgItem(Some(self.hwnd), ID_BOARDS_LIST as _).unwrap();
        let sel = SendMessageW(list, LB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))).0 as usize;

        if sel != LB_ERR as usize && sel < self.chain_boards.len() {
            self.chain_boards.remove(sel);
            self.ensure_initial_board_exists();
            self.refresh_chain_boards_list();
        }
    }

    unsafe fn select_initial_board(&mut self) {
        let list = GetDlgItem(Some(self.hwnd), ID_BOARDS_LIST as _).unwrap();

        let sel = SendMessageW(list, LB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))).0 as usize;
        if sel == LB_ERR as usize || sel >= self.chain_boards.len() {
            return;
        }

        self.initial_board = self.chain_boards[sel].clone();
        self.refresh_chain_boards_list();
    }

    unsafe fn ensure_initial_board_exists(&mut self) {
        if !self.chain_boards.contains(&self.initial_board) {
            if let Some(first_board) = self.chain_boards.first() {
                self.initial_board = first_board.clone();
            } else {
                self.initial_board.clear();
            }
        }
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
                let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ChainEditor;
                if dialog.is_null() {
                    return LRESULT(0);
                }

                let id = (wparam.0 & 0xFFFF) as u16;
                let notification = ((wparam.0 >> 16) & 0xFFFF) as u16;

                match id {
                    IDOK => {
                        (*dialog).result = DialogResult::Ok;
                        let _ = DestroyWindow(hwnd);
                    }
                    IDCANCEL => {
                        (*dialog).result = DialogResult::Cancel;
                        let _ = DestroyWindow(hwnd);
                    }
                    ID_BOARDS_LIST => {
                        if notification == LBN_DBLCLK as u16 {
                            (*dialog).select_initial_board();
                        }
                    }
                    ID_ADD_BOARD => (*dialog).add_board(),
                    ID_DELETE_BOARD => (*dialog).delete_board(),
                    _ => {}
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ChainEditor;
                if !dialog.is_null() {
                    (*dialog).result = DialogResult::Cancel;
                }
                let _ = DestroyWindow(hwnd);
                LRESULT(0)
            }
            WM_KEYDOWN => {
                let vk_code = wparam.0 as u16;

                if vk_code == VK_ESCAPE.0 {
                    let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ChainEditor;
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
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

}


pub fn open_chain_editor(chain_boards: Vec<String>,  initial_board: Option<String>, all_boards: Vec<String>, parent: Option<HWND>) -> Option<(Vec<String>, String)> {
    let mut editor = ChainEditor::new(chain_boards, initial_board, all_boards);
    let result = editor.show_modal(parent);
    if result == DialogResult::Ok {
        Some((editor.chain_boards, editor.initial_board.clone()))
    } else {
        None
    }
}

// Helper functions
fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
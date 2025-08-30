use std::mem;
use std::sync::Once;
use windows::core::*;
use windows::Win32::Foundation::*;
use windows::Win32::Graphics::Gdi::*;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;

// Control IDs
const ID_FONT_COMBO: u16 = 1001;
const ID_SIZE_COMBO: u16 = 1002;
const ID_BOLD_CHECK: u16 = 1003;
const ID_ITALIC_CHECK: u16 = 1004;
const ID_PREVIEW_PANEL: u16 = 1005;
const IDOK: u16 = 1;
const IDCANCEL: u16 = 2;

// Window class registration protection
static REGISTER_FONT_DIALOG_CLASS: Once = Once::new();
const FONT_DIALOG_CLASS_NAME: &str = "FontSelectionDialogClass";

#[derive(Debug, Clone, PartialEq)]
enum DialogResult {
    Ok,
    Cancel,
    None,
}

struct FontSelectionDialog {
    hwnd: HWND,
    selected_font: String,
    selected_size: i32,
    is_bold: bool,
    is_italic: bool,
    preview_font: HFONT,
    result: DialogResult,
    final_font_string: String,
}

impl FontSelectionDialog {
    fn new(initial_font: &str) -> Self {
        let (face, bold, italic, size) = parse_font(initial_font);
        Self {
            hwnd: HWND::default(),
            selected_font: face,
            selected_size: size,
            is_bold: bold,
            is_italic: italic,
            preview_font: HFONT::default(),
            result: DialogResult::None,
            final_font_string: String::new(),
        }
    }

    /// Register window class once using `Once` to ensure one-time initialization
    fn register_window_class(instance: HMODULE) {
        REGISTER_FONT_DIALOG_CLASS.call_once(|| {
            let class_name = to_wide_string(FONT_DIALOG_CLASS_NAME);
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

            // If parent is None, center on screen, else center on parent
            let (x, y) = if let Some(parent_hwnd) = parent {
                let mut rect = RECT::default();
                let _ = GetWindowRect(parent_hwnd, &mut rect);
                let parent_width = rect.right - rect.left;
                let parent_height = rect.bottom - rect.top;
                let x = rect.left + (parent_width - 470) / 2;
                let y = rect.top + (parent_height - 320) / 2;
                (x, y)
            } else {
                let screen_width = GetSystemMetrics(SM_CXSCREEN);
                let screen_height = GetSystemMetrics(SM_CYSCREEN);
                let x = (screen_width - 470) / 2;
                let y = (screen_height - 320) / 2;
                (x, y)
            };

            // Create dialog window
            let class_name = to_wide_string(FONT_DIALOG_CLASS_NAME);
            self.hwnd = CreateWindowExW(
                WS_EX_DLGMODALFRAME,
                PCWSTR::from_raw(class_name.as_ptr()),
                w!("Select Font"),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                x,
                y,
                470,
                320,
                parent,
                None,
                Some(instance.into()),
                None,
            ).unwrap();

            // Set up dialog pointer and create controls after window is created
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, self as *mut _ as _);
            self.update_preview_font();
            self.create_controls();

            // Message loop
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                // Handle tab navigation and enter key
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

    fn get_selected_font(&self) -> String {
        self.final_font_string.clone()
    }

    unsafe fn create_controls(&mut self) {
        let instance = GetModuleHandleW(None).unwrap();
        let default_font = GetStockObject(DEFAULT_GUI_FONT);

        // Font combo
        let font_combo = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("COMBOBOX"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL | WINDOW_STYLE(CBS_DROPDOWNLIST as u32 | CBS_HASSTRINGS as u32 | CBS_DISABLENOSCROLL as u32),
            20, 20, 250, 200,
            Some(self.hwnd),
            Some(HMENU(ID_FONT_COMBO as _)),
            Some(instance.into()),
            None,
        ).unwrap();
        SendMessageW(font_combo, WM_SETFONT, Some(WPARAM(default_font.0 as usize)), Some(LPARAM(1)));

        // Size combo
        let size_combo = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("COMBOBOX"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(CBS_DROPDOWN as u32 | CBS_HASSTRINGS as u32),
            280, 20, 80, 200,
            Some(self.hwnd),
            Some(HMENU(ID_SIZE_COMBO as _)),
            Some(instance.into()),
            None,
        ).unwrap();
        SendMessageW(size_combo, WM_SETFONT, Some(WPARAM(default_font.0 as usize)), Some(LPARAM(1)));

        // Bold button (checkbox style)
        let bold_check = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("B"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32 | BS_PUSHLIKE as u32),
            370, 19, 30, 24,
            Some(self.hwnd),
            Some(HMENU(ID_BOLD_CHECK as _)),
            Some(instance.into()),
            None,
        ).unwrap();
        let bold_font = CreateFontW(
            14, 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0,
            DEFAULT_CHARSET, FONT_OUTPUT_PRECISION(OUT_TT_PRECIS.0 as u8), FONT_CLIP_PRECISION(CLIP_DEFAULT_PRECIS.0 as u8),
            CLEARTYPE_QUALITY, 0, w!("Arial")
        );
        SendMessageW(bold_check, WM_SETFONT, Some(WPARAM(bold_font.0 as usize)), Some(LPARAM(1)));

        // Italic button (checkbox style)
        let italic_check = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("I"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_AUTOCHECKBOX as u32 | BS_PUSHLIKE as u32),
            410, 19, 30, 24,
            Some(self.hwnd),
            Some(HMENU(ID_ITALIC_CHECK as _)),
            Some(instance.into()),
            None,
        ).unwrap();
        let italic_font = CreateFontW(
            14, 0, 0, 0, FW_NORMAL.0 as i32, 1, 0, 0,
            DEFAULT_CHARSET, FONT_OUTPUT_PRECISION(OUT_TT_PRECIS.0 as u8), FONT_CLIP_PRECISION(CLIP_DEFAULT_PRECIS.0 as u8),
            CLEARTYPE_QUALITY, 0, w!("Arial")
        );
        SendMessageW(italic_check, WM_SETFONT, Some(WPARAM(italic_font.0 as usize)), Some(LPARAM(1)));

        // Preview panel (owner-drawn)
        let _preview = CreateWindowExW(
            WS_EX_CLIENTEDGE,
            w!("STATIC"),
            w!(""),
            WS_CHILD | WS_VISIBLE | WINDOW_STYLE(0x0000000D), // SS_OWNERDRAW
            20, 60, 420, 160,
            Some(self.hwnd),
            Some(HMENU(ID_PREVIEW_PANEL as _)),
            Some(instance.into()),
            None,
        ).unwrap();

        // OK button
        let ok_button = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("OK"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_DEFPUSHBUTTON as _),
            240, 235, 90, 30,
            Some(self.hwnd),
            Some(HMENU(IDOK as _)),
            Some(instance.into()),
            None,
        ).unwrap();
        SendMessageW(ok_button, WM_SETFONT, Some(WPARAM(default_font.0 as usize)), Some(LPARAM(1)));

        // Cancel button
        let cancel_button = CreateWindowExW(
            WINDOW_EX_STYLE::default(),
            w!("BUTTON"),
            w!("Cancel"),
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WINDOW_STYLE(BS_PUSHBUTTON as _),
            350, 235, 90, 30,
            Some(self.hwnd),
            Some(HMENU(IDCANCEL as _)),
            Some(instance.into()),
            None,
        ).unwrap();
        SendMessageW(cancel_button, WM_SETFONT, Some(WPARAM(default_font.0 as usize)), Some(LPARAM(1)));

        // Initialize controls
        self.populate_font_combo();
        self.populate_size_combo();
        self.set_initial_values();
    }

    unsafe fn populate_font_combo(&self) {
        let font_combo = GetDlgItem(Some(self.hwnd), ID_FONT_COMBO as _).unwrap();

        for font in get_system_fonts() {
            let font_hstring = HSTRING::from(&font);
            SendMessageW(font_combo, CB_ADDSTRING, Some(WPARAM(0)), Some(LPARAM(font_hstring.as_ptr() as isize)));
        }
    }

    unsafe fn populate_size_combo(&self) {
        let size_combo = GetDlgItem(Some(self.hwnd), ID_SIZE_COMBO as _).unwrap();

        for size in get_font_sizes() {
            let size_str = HSTRING::from(size.to_string());
            SendMessageW(size_combo, CB_ADDSTRING, Some(WPARAM(0)), Some(LPARAM(size_str.as_ptr() as isize)));
        }
    }

    unsafe fn set_initial_values(&self) {
        let font_combo = GetDlgItem(Some(self.hwnd), ID_FONT_COMBO as _).unwrap();
        let size_combo = GetDlgItem(Some(self.hwnd), ID_SIZE_COMBO as _).unwrap();
        let bold_check = GetDlgItem(Some(self.hwnd), ID_BOLD_CHECK as _).unwrap();
        let italic_check = GetDlgItem(Some(self.hwnd), ID_ITALIC_CHECK as _).unwrap();

        // Set font selection
        let fonts = get_system_fonts();
        let font_index = fonts.iter().position(|f| f == &self.selected_font).unwrap_or(0);
        SendMessageW(font_combo, CB_SETCURSEL, Some(WPARAM(font_index)), Some(LPARAM(0)));

        // Set size
        let size_str = HSTRING::from(self.selected_size.to_string());
        let _ = SetWindowTextW(size_combo, &size_str);

        // Set bold/italic
        SendMessageW(bold_check, BM_SETCHECK, Some(WPARAM(if self.is_bold { 1 } else { 0 })), Some(LPARAM(0)));
        SendMessageW(italic_check, BM_SETCHECK, Some(WPARAM(if self.is_italic { 1 } else { 0 })), Some(LPARAM(0)));

        // Trigger initial preview paint
        self.refresh_preview();
    }

    unsafe fn update_preview_font(&mut self) {
        // Delete old font if exists
        if !self.preview_font.is_invalid() {
            let _ = DeleteObject(self.preview_font.into());
        }

        let weight = if self.is_bold { FW_BOLD.0 } else { FW_NORMAL.0 };
        let italic = if self.is_italic { 1 } else { 0 };

        self.preview_font = CreateFontW(
            self.selected_size,
            0, 0, 0,
            weight as i32,
            italic,
            0, 0,
            DEFAULT_CHARSET,
            FONT_OUTPUT_PRECISION(OUT_TT_PRECIS.0 as u8),
            FONT_CLIP_PRECISION(CLIP_DEFAULT_PRECIS.0 as u8),
            CLEARTYPE_QUALITY,
            0,
            &HSTRING::from(&self.selected_font),
        );
    }

    unsafe fn refresh_preview(&self) {
        let preview_panel = GetDlgItem(Some(self.hwnd), ID_PREVIEW_PANEL as _).unwrap();
        let _ = RedrawWindow(Some(preview_panel), None, None, RDW_INVALIDATE | RDW_UPDATENOW);
    }

    unsafe fn paint_preview_with_rect(&self, hdc: HDC, rect: &RECT) {
        // Clear the background with white
        let white_brush = GetStockObject(WHITE_BRUSH);
        FillRect(hdc, rect, HBRUSH(white_brush.0));

        if !self.preview_font.is_invalid() {
            let old_font = SelectObject(hdc, self.preview_font.into());
            SetBkMode(hdc, TRANSPARENT);

            let preview_text = "Text Preview";
            let mut preview_wide: Vec<u16> = preview_text.encode_utf16().chain(Some(0)).collect();

            let mut rect_copy = *rect;
            DrawTextW(
                hdc,
                &mut preview_wide,
                &mut rect_copy,
                DT_CENTER | DT_VCENTER | DT_SINGLELINE,
            );

            SelectObject(hdc, old_font);
        }
    }

    unsafe fn handle_font_change(&mut self) {
        let font_combo = GetDlgItem(Some(self.hwnd), ID_FONT_COMBO as _).unwrap();
        let index = SendMessageW(font_combo, CB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))).0 as i32;

        if index != CB_ERR {
            let fonts = get_system_fonts();
            if let Some(font_name) = fonts.get(index as usize) {
                self.selected_font = font_name.clone();
                self.update_preview_font();
                self.refresh_preview();
            }
        }
    }

    unsafe fn handle_size_selchange(&mut self) {
        // Handle selection from dropdown list
        let size_combo = GetDlgItem(Some(self.hwnd), ID_SIZE_COMBO as _).unwrap();
        let index = SendMessageW(size_combo, CB_GETCURSEL, Some(WPARAM(0)), Some(LPARAM(0))).0 as i32;

        if index != CB_ERR {
            let sizes = get_font_sizes();
            if let Some(&size) = sizes.get(index as usize) {
                self.selected_size = size;
                self.update_preview_font();
                self.refresh_preview();
            }
        }
    }

    unsafe fn handle_size_editchange(&mut self) {
        // Handle manual text entry
        let size_combo = GetDlgItem(Some(self.hwnd), ID_SIZE_COMBO as _).unwrap();
        let mut text = [0u16; 10];
        let len = GetWindowTextW(size_combo, &mut text);

        if len > 0 {
            let size_str = String::from_utf16_lossy(&text[..len as usize]);
            if let Ok(size) = size_str.parse::<i32>() {
                self.selected_size = size;
                self.update_preview_font();
                self.refresh_preview();
            }
        }
    }

    unsafe fn handle_bold_change(&mut self) {
        let bold_check = GetDlgItem(Some(self.hwnd), ID_BOLD_CHECK as _).unwrap();
        let checked = SendMessageW(bold_check, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0))).0;

        self.is_bold = checked == 1;
        self.update_preview_font();
        self.refresh_preview();
    }

    unsafe fn handle_italic_change(&mut self) {
        let italic_check = GetDlgItem(Some(self.hwnd), ID_ITALIC_CHECK as _).unwrap();
        let checked = SendMessageW(italic_check, BM_GETCHECK, Some(WPARAM(0)), Some(LPARAM(0))).0;

        self.is_italic = checked == 1;
        self.update_preview_font();
        self.refresh_preview();
    }

    fn get_font_string(&self) -> String {
        let mut parts = vec![self.selected_font.clone()];
        if self.is_bold {
            parts.push("Bold".to_string());
        }
        if self.is_italic {
            parts.push("Italic".to_string());
        }
        parts.push(self.selected_size.to_string());
        parts.join(" ")
    }

    unsafe fn save_data(&mut self) {
        self.final_font_string = self.get_font_string();
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
                let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut FontSelectionDialog;
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
                    ID_FONT_COMBO if notification == CBN_SELCHANGE as u16 => {
                        (*dialog).handle_font_change();
                    }
                    ID_SIZE_COMBO if notification == CBN_SELCHANGE as u16 => {
                        (*dialog).handle_size_selchange();
                    }
                    ID_SIZE_COMBO if notification == CBN_EDITCHANGE as u16 => {
                        (*dialog).handle_size_editchange();
                    }
                    ID_BOLD_CHECK if notification == BN_CLICKED as u16 => {
                        (*dialog).handle_bold_change();
                    }
                    ID_ITALIC_CHECK if notification == BN_CLICKED as u16 => {
                        (*dialog).handle_italic_change();
                    }
                    _ => {}
                }
                LRESULT(0)
            }
            WM_DRAWITEM => {
                let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut FontSelectionDialog;
                if dialog.is_null() {
                    return LRESULT(0);
                }

                // Correctly handle owner-draw static control
                #[repr(C)]
                #[allow(non_snake_case)]
                struct DRAWITEMSTRUCT {
                    CtlType: u32,
                    CtlID: u32,
                    itemID: u32,
                    itemAction: u32,
                    itemState: u32,
                    hwndItem: HWND,
                    hDC: HDC,
                    rcItem: RECT,
                    itemData: usize,
                }

                let draw_item = &*(lparam.0 as *const DRAWITEMSTRUCT);
                if draw_item.CtlID == ID_PREVIEW_PANEL as u32 {
                    (*dialog).paint_preview_with_rect(draw_item.hDC, &draw_item.rcItem);
                    return LRESULT(1);
                }
                LRESULT(0)
            }
            WM_CLOSE => {
                let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut FontSelectionDialog;
                if !dialog.is_null() {
                    (*dialog).result = DialogResult::Cancel;
                    let _ = DestroyWindow(hwnd);
                }
                LRESULT(0)
            }
            WM_KEYDOWN => {
                let vk_code = wparam.0 as u16;
                if vk_code == VK_ESCAPE.0 {
                    let dialog = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut FontSelectionDialog;
                    if !dialog.is_null() {
                        (*dialog).result = DialogResult::Cancel;
                        let _ = DestroyWindow(hwnd);
                    }
                }
                LRESULT(0)
            }
            WM_DESTROY => {
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

impl Drop for FontSelectionDialog {
    fn drop(&mut self) {
        if !self.preview_font.is_invalid() {
            unsafe { let _ = DeleteObject(self.preview_font.into()); }
        }
    }
}

// Helper functions
fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn parse_font(font_str: &str) -> (String, bool, bool, i32) {
    let parts: Vec<&str> = font_str.split_whitespace().collect();
    if parts.is_empty() {
        return ("Arial".to_string(), false, false, 12);
    }

    let size = parts
        .last()
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(12);

    let mut face_parts = Vec::new();
    let mut bold = false;
    let mut italic = false;

    for part in &parts[..parts.len().saturating_sub(1)] {
        match part.to_lowercase().as_str() {
            "bold" => bold = true,
            "italic" => italic = true,
            _ => face_parts.push(*part),
        }
    }

    let face = if face_parts.is_empty() {
        "Arial".to_string()
    } else {
        face_parts.join(" ")
    };

    (face, bold, italic, size)
}

fn get_system_fonts() -> Vec<String> {
    unsafe {
        let mut fonts = Vec::new();

        // Get screen DC
        let hdc = GetDC(None);

        // Set up LOGFONTW to enumerate all fonts
        let mut logfont = LOGFONTW::default();
        logfont.lfCharSet = DEFAULT_CHARSET;

        // Callback to collect font names
        unsafe extern "system" fn enum_font_proc(
            lpelfe: *const LOGFONTW,
            _lpntme: *const TEXTMETRICW,
            _font_type: u32,
            lparam: LPARAM,
        ) -> i32 {
            let fonts = &mut *(lparam.0 as *mut Vec<String>);
            let logfont = &*lpelfe;

            // Convert font name from wide string
            let len = logfont.lfFaceName.iter().position(|&c| c == 0).unwrap_or(logfont.lfFaceName.len());
            let font_name = String::from_utf16_lossy(&logfont.lfFaceName[..len]);

            // Skip fonts that start with @ (vertical fonts)
            if !font_name.starts_with('@') && !font_name.is_empty() {
                // Avoid duplicates
                if !fonts.contains(&font_name) {
                    fonts.push(font_name);
                }
            }

            1 // Continue enumeration
        }

        // Enumerate fonts
        EnumFontFamiliesExW(
            hdc,
            &logfont,
            Some(enum_font_proc),
            LPARAM(&mut fonts as *mut _ as isize),
            0,
        );

        let _ = ReleaseDC(None, hdc);

        // Sort alphabetically
        fonts.sort();

        fonts
    }
}

fn get_font_sizes() -> Vec<i32> {
    vec![8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 36, 48, 72]
}

/// Convenience function to show font picker and return the result
/// Returns Some(font_string) if user selected a font, None if cancelled
pub fn open_font_editor(initial_font: &str, parent: Option<HWND>) -> Option<String> {
    let mut dialog = FontSelectionDialog::new(initial_font);
    match dialog.show_modal(parent) {
        DialogResult::Ok => Some(dialog.get_selected_font()),
        DialogResult::Cancel | DialogResult::None => None,
    }
}
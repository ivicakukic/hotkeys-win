use super::keys::{ModifierState, keyboard_api};
use windows::Win32::Foundation::*;
use windows::Win32::UI::Input::KeyboardAndMouse::*;


pub struct ModifierHandler {
    state: ModifierState,
}

impl ModifierHandler {
    /// Create new handler with given initial state
    pub fn new(initial_state: ModifierState) -> Self {
        Self {
            state: initial_state,
        }
    }

    /// Get current state (for comparison or external use)
    pub fn state(&self) -> &ModifierState {
        &self.state
    }

    /// Handle key press event (WM_KEYDOWN)
    /// Returns true if this was a modifier key we handle, false otherwise
    pub fn handle_key_press(&mut self, vk_code: VIRTUAL_KEY) -> bool {
        match vk_code {
            VK_LCONTROL | VK_RCONTROL | VK_CONTROL => {
                self.state.ctrl = true;
                true
            },
            VK_LSHIFT | VK_RSHIFT | VK_SHIFT => {
                self.state.shift = true;
                true
            },
            VK_LMENU | VK_RMENU | VK_MENU => { // Alt key
                self.state.alt = true;
                true
            },
            // VK_LWIN | VK_RWIN => { // Windows key (Super)
            //     self.state.super_key = true;
            //     true
            // },
            _ => false,
        }
    }

    /// Handle key release event (WM_KEYUP)
    /// Returns true if this was a modifier key we handle, false otherwise
    pub fn handle_key_release(&mut self, vk_code: VIRTUAL_KEY) -> bool {
        match vk_code {
            VK_LCONTROL | VK_RCONTROL | VK_CONTROL => {
                self.state.ctrl = false;
                true
            },
            VK_LSHIFT | VK_RSHIFT | VK_SHIFT => {
                self.state.shift = false;
                true
            },
            VK_LMENU | VK_RMENU | VK_MENU => { // Alt key
                self.state.alt = false;
                true
            },
            // VK_LWIN | VK_RWIN => { // Windows key (Super)
            //     self.state.super_key = false;
            //     true
            // },
            _ => false,
        }
    }

    pub fn is_modifier(vk: VIRTUAL_KEY) -> bool {
        matches!(vk, VK_LMENU | VK_RMENU | VK_MENU
                | VK_LCONTROL | VK_RCONTROL | VK_CONTROL
                | VK_LSHIFT | VK_RSHIFT | VK_SHIFT
            //  | VK_LWIN | VK_RWIN
         )
    }

}


pub struct TextCapture {
    allow_newline: bool,
    text: String,
    // modifiers: ModifierState,
}

impl TextCapture {
    pub fn new(text: Option<String>, allow_newline: bool) -> Self {
        Self {
            text: text.unwrap_or_default(),
            allow_newline,
        }
    }

    pub fn text(&self) -> Option<String> {
        if self.text.is_empty() {
            None
        } else {
            Some(self.text.clone())
        }
    }

    pub fn on_keydown(&mut self, wparam: WPARAM, modifiers: ModifierState) -> LRESULT {
        self.append_input(VIRTUAL_KEY(wparam.0 as u16), modifiers);
        LRESULT(0)
    }

    pub fn on_keyup(&mut self, _wparam: WPARAM, _modifiers: ModifierState) -> LRESULT {
        LRESULT(0)
    }

    fn append_input(&mut self, vkey: VIRTUAL_KEY, modifiers: ModifierState) {

        if vkey == VK_BACK && (modifiers.ctrl || modifiers.shift) {
            self.text.clear();
            return;
        }

        if vkey == VK_BACK {
            self.text.pop();
            return;
        }

        if vkey == VK_SPACE {
            self.text.push(' ');
            return;
        }

        if vkey == VK_ESCAPE {
            return;
        }

        if (modifiers.ctrl && vkey == VK_V) || (modifiers.shift && vkey == VK_INSERT) {
            use clipboard_win::{get_clipboard, formats::Unicode};
            if let Ok(result) = get_clipboard::<String, Unicode>(Unicode) {
                if !result.is_empty() {
                    self.text.push_str(&result);
                }
            }
        }

        if self.allow_newline && vkey == VK_RETURN && modifiers.shift {
            self.text.push('\n');
            return;
        }

        if vkey == VK_RETURN {
            return;
        }

        let modifiers = ModifierState { shift: modifiers.shift, ..Default::default() };
        // TODO: sort-out the dependencies here
        if let Some(ch) = crate::input::keys::keyboard_api::vkey_to_string(vkey.0, &modifiers, false) {
            self.text.push_str(&ch);
            return;
        }
    }
}



/// A keyboard key combination (e.g. "Ctrl+K")
#[derive(Debug, Clone)]
pub struct Combination {
    pub modifiers: ModifierState,
    pub key: Option<u16>
}

pub struct KeyCombinationCapture {
    records: Vec<Combination>,
    has_active_record: bool,
    modifiers: ModifierState,
}

impl KeyCombinationCapture {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            has_active_record: false,
            modifiers: ModifierState::default(),
        }
    }

    fn activate_record(&mut self) {
        if !self.has_active_record {
            self.has_active_record = true;
        }
    }

    fn finalize_record(&mut self, vk_code: VIRTUAL_KEY) {
        self.has_active_record = false;
        log::debug!(target:"input_api", "Captured: ({:#x}, {})", vk_code.0, self.modifiers);
        self.records.push(Combination {
            modifiers: self.modifiers.clone(),
            key: Some(vk_code.0),
        });
    }

    pub fn on_keydown(&mut self, wparam: WPARAM, modifiers: ModifierState) -> LRESULT {
        let vk_code = VIRTUAL_KEY(wparam.0 as u16);

        // Handle modifier keys first
        let old_state = self.modifiers.clone();
        let is_modifier = old_state != modifiers;

        if is_modifier {
            self.modifiers = modifiers.clone();
            if !self.has_active_record {
                self.activate_record();
            }
        } else {
            self.finalize_record(vk_code);
        }

        LRESULT(0)
    }

    pub fn on_keyup(&mut self, _wparam: WPARAM, modifiers: ModifierState) -> LRESULT {
        self.modifiers = modifiers.clone();
        LRESULT(0)
    }

    fn has_captured_anything(&self) -> bool {
        ! ( self.records.is_empty() && !self.has_active_record && self.modifiers.is_none() )
    }

    pub fn get_current_capture(&self) -> Vec<Combination> {

        if ! self.has_captured_anything() {
            return vec![]
        }

        let mut parts = Vec::new();

        // Determine how many records to show based on parity
        let total_records = self.records.len() + if self.has_active_record { 1 } else { 0 };
        if total_records > 0 {
            let is_odd = total_records % 2 == 1;
            // eprintln!("- act:{}, rec:{}, tot:{}, odd:{}", self.has_active_record, self.records.len(), total_records, is_odd );
            if is_odd {
                if self.has_active_record {
                    parts.push(Combination{
                        modifiers: self.modifiers.clone(),
                        key: None,
                    });
                } else {
                    if let Some(last) = self.records.last() {
                        parts.push(last.clone());
                    }
                }
            } else {
                if ! self.has_active_record {
                    // add last two terms
                    if self.records.len() as i32 - 2 >= 0 {
                        if let Some(second_last) = self.records.get(self.records.len() - 2) {
                            parts.push(second_last.clone());
                        }
                    } else {
                        log::error!("No second last record, but expected one");
                    }
                    if let Some(last) = self.records.last() {
                        parts.push(last.clone());
                    }
                } else {
                    if let Some(last) = self.records.last() {
                        parts.push(last.clone());
                    }
                    parts.push(Combination{
                        modifiers: self.modifiers.clone(),
                        key: None,
                    });
                }
            }
        }

        parts

    }

    pub fn last_record(&self) -> Option<&Combination> {
        self.records.last()
    }

    pub fn remove_last_record(&mut self) {
        self.records.pop();
    }

    pub fn deactivate_record(&mut self) {
        self.has_active_record = false;
    }

}

pub enum DisplaySeparator {
    Space,
    Plus,
    SpacePlusSpace
}

impl DisplaySeparator {
    fn to_str(&self) -> &'static str {
        match self {
            DisplaySeparator::Space => " ",
            DisplaySeparator::Plus => "+",
            DisplaySeparator::SpacePlusSpace => " + ",
        }
    }
}

pub enum DisplayCase {
    #[allow(dead_code)]
    Upper,
    #[allow(dead_code)]
    Lower,
    Title,
}

impl DisplayCase {
    fn apply(&self, s: String) -> String {
        match self {
            DisplayCase::Upper => s.to_uppercase(),
            DisplayCase::Lower => s.to_lowercase(),
            DisplayCase::Title => s.to_ascii_lowercase().chars().enumerate().map(|(i, c)| if i == 0 { c.to_ascii_uppercase() } else { c }).collect(),
        }
    }
}

pub struct DisplayFormat {
    key_separator: DisplaySeparator,
    combination_separator: DisplaySeparator,
    display_case: DisplayCase,
}

static STANDARD_FORMAT: DisplayFormat = DisplayFormat { key_separator: DisplaySeparator::Plus, combination_separator: DisplaySeparator::Space, display_case: DisplayCase::Title };
static INVERSE_FORMAT: DisplayFormat = DisplayFormat { key_separator: DisplaySeparator::Space, combination_separator: DisplaySeparator::Plus, display_case: DisplayCase::Title };
static INVERSE_FORMAT_SPACED: DisplayFormat = DisplayFormat { key_separator: DisplaySeparator::Space, combination_separator: DisplaySeparator::SpacePlusSpace, display_case: DisplayCase::Title };

pub trait DisplayFormatable {
    fn display_format(&self, fmt: &DisplayFormat) -> String;
}

impl DisplayFormatable for Combination {
    fn display_format(&self, fmt: &DisplayFormat) -> String {
        let mut parts = Vec::new();

        if ! self.modifiers.is_none() {
            parts.extend(self.modifiers.display_state().iter().map(|s| s.to_string()));
        }

        if let Some(key) = self.key {
            // let modifiers = ModifierState { shift: self.modifiers.shift, ..Default::default() };
            let modifiers = ModifierState::default();
            if let Some(ch) = keyboard_api::vkey_to_string(key, &modifiers, true) {
                if ch == "+" {
                    parts.push("'+'".to_string());
                } else {
                    parts.push(ch);
                }
            }
        }

        let parts: Vec<String> = parts.into_iter().map(|s| fmt.display_case.apply(s)).collect();
        parts.join(fmt.key_separator.to_str())
    }
}

impl DisplayFormatable for Vec<Combination> {
    fn display_format(&self, fmt: &DisplayFormat) -> String {
        if self.is_empty() {
            "(empty)".to_string()
        } else {
            let parts: Vec<String> = self.iter().map(|c| c.display_format(fmt)).collect();
            parts.join(fmt.combination_separator.to_str())
        }
    }
}

pub enum DisplayFormats {
    #[allow(dead_code)]
    Standard,
    #[allow(dead_code)]
    Inverse,
    InverseSpaced,
}

impl DisplayFormats {
    pub fn get_format(&self) -> &DisplayFormat {
        match self {
            DisplayFormats::Standard => &STANDARD_FORMAT,
            DisplayFormats::Inverse => &INVERSE_FORMAT,
            DisplayFormats::InverseSpaced => &INVERSE_FORMAT_SPACED,
        }
    }
}
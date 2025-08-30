use serde::{Deserialize, Serialize};

pub mod vkey;

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Default)]
pub struct ModifierState {
    #[serde(default)]
    pub ctrl: bool,
    #[serde(default)]
    pub shift: bool,
    #[serde(default)]
    pub alt: bool,
    #[serde(default, rename = "super")]
    pub super_key: bool,
}

impl std::fmt::Display for ModifierState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let parts = self.display_state();
        write!(f, "{}", parts.join("+"))
    }
}

impl ModifierState {
    pub fn is_none(&self) -> bool {
        !self.ctrl && !self.shift && !self.alt && !self.super_key
    }

    pub fn is_any(&self) -> bool {
        self.ctrl || self.shift || self.alt || self.super_key
    }

    pub fn display_state(&self) -> Vec<&'static str> {
        let mut parts = Vec::new();
        if self.ctrl { parts.push("Ctrl"); }
        if self.shift { parts.push("Shift"); }
        if self.alt { parts.push("Alt"); }
        if self.super_key { parts.push("Super"); }
        parts
    }
}

pub mod keyboard_api {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    use crate::input::keys::vkey;

    use super::ModifierState;

    /// Convert character to virtual key + shift state using Windows API
    pub fn char_to_vkey(ch: char) -> Option<(u16, ModifierState)> {
        unsafe {
            let layout = GetKeyboardLayout(0); // Current layout
            let result = VkKeyScanExW(ch as u16, layout);
            if result == -1 {
                None
            } else {
                let vk_code = (result & 0xFF) as u16;
                let shift_state = ((result >> 8) & 0xFF) as u8;

                let mut modifiers = ModifierState::default();

                // Parse shift state flags from VkKeyScanEx result
                if shift_state & 1 != 0 { modifiers.shift = true; }
                if shift_state & 2 != 0 { modifiers.ctrl = true; }
                if shift_state & 4 != 0 { modifiers.alt = true; }

                Some((vk_code, modifiers))
            }
        }
    }

    /// Convert virtual key + modifiers to string using Windows API
    /// We use String here because of non-printable keys, mapped explicitly in vkey: VK_F1, VK_NUMLOCK, etc.
    pub fn vkey_to_string(vk_code: u16, modifiers: &ModifierState, add_non_printable: bool) -> Option<String> {
        unsafe {
            let layout = GetKeyboardLayout(0); // Current layout

            // Build keyboard state array (256 bytes)
            let mut keyboard_state = [0u8; 256];

            // Set modifier states (0x80 = key is pressed)
            if modifiers.ctrl {
                keyboard_state[VK_CONTROL.0 as usize] = 0x80;
            }
            if modifiers.alt {
                keyboard_state[VK_MENU.0 as usize] = 0x80;
            }
            if modifiers.shift {
                keyboard_state[VK_SHIFT.0 as usize] = 0x80;
            }

            if add_non_printable {
                // Check if it's a non-printable key (like F1, Esc, etc.)
                if let Some(vk) = vkey::find_vkey_by_code(vk_code) {
                    return Some(vk.title.to_string());
                }
            }

            // Get scan code for the virtual key
            let scan_code = MapVirtualKeyExW(vk_code as u32, MAPVK_VK_TO_VSC, Some(layout));

            // Convert to Unicode
            let mut buffer = [0u16; 16];
            let result = ToUnicodeEx(
                vk_code as u32,
                scan_code,
                &keyboard_state,
                &mut buffer,
                0, // flags
                Some(layout)
            );

            if result > 0 {
                // Convert UTF-16 to String
                let chars: Vec<u16> = buffer[..result as usize].to_vec();
                String::from_utf16(&chars).ok()
            } else {
                None
            }
        }
    }
}

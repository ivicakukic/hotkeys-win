use std::fmt::Display;

use windows::Win32::UI::Input::KeyboardAndMouse::{SendInput, INPUT, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, INPUT_KEYBOARD, INPUT_0, KEYBD_EVENT_FLAGS};

pub struct KeyboardInput {
    pub vk_code: u16,
    pub key_down: bool
}

pub fn send_input (input: KeyboardInput) {
    unsafe {
        log::trace!(target:"input_api", "Input: {}", input);
        let pinputs = create_input(input.vk_code, input.key_down);
        SendInput(&[pinputs], std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn send_inputs (inputs: Vec<KeyboardInput>) {
    unsafe {
        let pinputs = inputs.iter().map(|input| {
            create_input(input.vk_code, input.key_down)
        }).collect::<Vec<INPUT>>();

        log::trace!(target:"input_api", "Inputs: {}", KeyboardInputs { vec: inputs });

        SendInput(pinputs.as_slice(), std::mem::size_of::<INPUT>() as i32);
    }
}

fn create_input(vk_code: u16, key_down: bool) -> INPUT {
    unsafe {
        let mut input_u: INPUT_0 = std::mem::zeroed();
        *(& mut input_u.ki) = KEYBDINPUT {
            wVk: VIRTUAL_KEY(vk_code),
            dwFlags: if key_down { KEYBD_EVENT_FLAGS(0) } else { KEYEVENTF_KEYUP },
            dwExtraInfo: 1,
            wScan: 0,
            time: 0,
        };

        INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: input_u
        }
    }
}

impl Display for KeyboardInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{{{:#x},{}}}",
                        self.vk_code,
                        if self.key_down { "down" } else { "up" })
    }
}

struct KeyboardInputs { pub vec: Vec<KeyboardInput> }

impl Display for KeyboardInputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[")
        .and_then(|_| write!(f, "{}", self.vec.iter().map(|el| format!("{}", el)).collect::<Vec<String>>().join(",")))
        .and_then(|_| write!(f, "]"))
    }
}
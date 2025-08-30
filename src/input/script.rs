use super::{
    steps::*,
    keys::{
        vkey::{VK_SHIFT, VK_ENTER, VK_ALT, VK_CTRL, find_vkey_by_text},
        ModifierState
    }
};

pub struct InputScript {
    pub steps: Vec<Box<dyn InputStep>>
}

impl InputScript {
    pub fn play(&self) {
        self.steps.iter().for_each(|step| step.play());
    }
}

enum Token {
    PLUS,
    CHAR(String),
    QUOTED(String),
    WORD(String),
}

struct KeyCombination {
    keys: Vec<u16>,
}

impl Default for KeyCombination {
    fn default() -> Self {
        Self { keys: Default::default() }
    }
}

use Token::*;

fn scan_shortcut_expression(text: &str) -> Vec<Token> {
    let txt = text.to_owned()
                .replace("'+'", "_PLUS_")
                .replace("+", " + ")
                .replace("_PLUS_", "'+'");

    let splits: Vec<&str> = txt.split(" ").collect();
    let tokens: Vec<Token> = splits
        .iter()
        .filter(|val| { "".ne(**val) })
        .map(|val| {
            let low = val.to_lowercase();
            let chars = low.chars().collect::<Vec<char>>();
            let len = chars.len();
            let is_quoted = (len == 3) && (chars[0] == '\'') && (chars[2] == '\'');
            let is_letter = len == 1;

            let letter = if is_letter { chars[0] }
                else if is_quoted { chars[1] }
                else { ' ' };

            let is_plus = is_letter && letter == '+';

            if is_quoted { QUOTED(letter.to_string()) }
            else if is_plus { PLUS }
            else if is_letter { CHAR(letter.to_string()) }
            else { WORD(low) }
        })
        .collect();
    tokens
}

fn parse_shortcut_expression(text: &str) -> Vec<KeyCombination> {
    scan_shortcut_expression(text.to_lowercase().as_str())
    .into_iter()
    .fold(Vec::new(), |mut acc, token| {
        match token {
            WORD(text) => {
                if acc.is_empty() {
                    acc.push(KeyCombination::default());
                }
                if let Some(vk) = find_vkey_by_text(text.clone()) {
                    acc.last_mut().unwrap().keys.push(vk.vkey);
                } else {
                    log::error!(target:"input_api", "Unsupported key or modifier in shortcut expression: '{}'", text);
                }
            },
            CHAR(text) | QUOTED(text) => {
                assert!(text.chars().count() == 1);

                if acc.is_empty() {
                    acc.push(KeyCombination::default());
                }

                let char = text.chars().next().unwrap();
                let (vk_code, modifiers) = super::keys::keyboard_api::char_to_vkey(char).unwrap_or_default();

                if vk_code > 0 && modifiers.is_none() {
                    acc.last_mut().unwrap().keys.push(vk_code);
                } else {
                    log::error!(target:"input_api", "Unsupported key or modifier in shortcut expression: '{}'", text);
                }
            },
            PLUS => acc.push(KeyCombination::default())
        }
        acc
    })
}

pub fn for_shortcut(text: String) -> InputScript {
    log::debug!(target:"input_api", "Shortcut: {}",  text);

    let mut steps = vec![];
    for cmb in parse_shortcut_expression(text.as_str()) {
        steps.append(&mut cmb.keys.iter().map(
            |key| Box::new(map_vk_code(*key, true)) as Box<dyn InputStep>).collect());
        steps.append(&mut cmb.keys.iter().rev().map(
            |key| Box::new(map_vk_code(*key, false)) as Box<dyn InputStep>).collect());
    }

    InputScript { steps }
}

pub fn for_pause(pause: u64) -> InputScript {
    log::debug!(target:"input_api", "Pause: {}ms",  pause);
    InputScript { steps: vec![
        Box::new( NoInput { pause } )
    ] }
}

pub fn for_text(text: String) -> InputScript {
    log::debug!(target:"input_api", "Text: {}",  text);
    for_text_or_line(text, false)
}

pub fn for_line(text: String) -> InputScript {
    log::debug!(target:"input_api", "Line: {}",  text);
    for_text_or_line(text, true)
}

fn for_text_or_line(text: String, new_line: bool) -> InputScript {
    let mut steps = vec![];

    for ch in text.chars() {
        if let Some((vk_code, modifiers)) = super::keys::keyboard_api::char_to_vkey(ch) {
            let inputs = map_character_key(vk_code, &modifiers);
            steps.push(Box::new(KeyInputs { inputs }) as Box<dyn InputStep>);
        }
    }

    if new_line {
        let enter_inputs = map_character_key(VK_ENTER.vkey, &ModifierState::default());
        steps.push(Box::new(KeyInputs { inputs: enter_inputs }) as Box<dyn InputStep>);
    }

    InputScript { steps }
}

fn map_vk_code(vk_code: u16, key_down: bool) -> KeyInput {
    KeyInput { vk_code, key_down }
}

fn map_character_key(vk_code: u16, modifiers: &ModifierState) -> Vec<KeyInput> {
    let mut inputs = vec![];

    if modifiers.ctrl {
        inputs.push(KeyInput { vk_code: VK_CTRL.vkey, key_down: true });
    }
    if modifiers.shift {
        inputs.push(KeyInput { vk_code: VK_SHIFT.vkey, key_down: true });
    }
    if modifiers.alt {
        inputs.push(KeyInput { vk_code: VK_ALT.vkey, key_down: true });
    }

    inputs.push(KeyInput { vk_code, key_down: true });
    inputs.push(KeyInput { vk_code, key_down: false });

    if modifiers.alt {
        inputs.push(KeyInput { vk_code: VK_ALT.vkey, key_down: false });
    }

    if modifiers.shift {
        inputs.push(KeyInput { vk_code: VK_SHIFT.vkey, key_down: false });
    }

    if modifiers.ctrl {
        inputs.push(KeyInput { vk_code: VK_CTRL.vkey, key_down: false });
    }

    inputs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::keys::vkey::{VK_SHIFT};
    use windows::Win32::UI::Input::KeyboardAndMouse::{*};

    #[test]
    fn test_shortcut() {
        let script = for_shortcut("Ctrl Shift A".to_string());

        assert_eq!(script.steps.len(), 6);

        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_CTRL.vkey, key_down: true });
        assert_eq!(script.steps[1].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_SHIFT.vkey, key_down: true });
        assert_eq!(script.steps[2].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_A.0, key_down: true });

        assert_eq!(script.steps[3].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_A.0, key_down: false });
        assert_eq!(script.steps[4].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_SHIFT.vkey, key_down: false });
        assert_eq!(script.steps[5].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_CTRL.vkey, key_down: false });
    }

    #[test]
    fn test_shortcurt_chord() {
        let script = for_shortcut("Ctrl K + Ctrl B".to_string());

        assert_eq!(script.steps.len(), 8);

        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_CTRL.vkey, key_down: true });
        assert_eq!(script.steps[1].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_K.0, key_down: true });
        assert_eq!(script.steps[2].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_K.0, key_down: false });
        assert_eq!(script.steps[3].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_CTRL.vkey, key_down: false });

        assert_eq!(script.steps[4].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_CTRL.vkey, key_down: true });
        assert_eq!(script.steps[5].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_B.0, key_down: true });
        assert_eq!(script.steps[6].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_B.0, key_down: false });
        assert_eq!(script.steps[7].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_CTRL.vkey, key_down: false });
    }

    #[test]
    fn test_text() {
        let script = for_text("abK".to_string());

        assert_eq!(script.steps.len(), 3);
        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInputs>().unwrap().inputs.len(), 2);
        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[0], KeyInput { vk_code: VK_A.0, key_down: true });
        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[1], KeyInput { vk_code: VK_A.0, key_down: false });

        assert_eq!(script.steps[1].as_any().downcast_ref::<KeyInputs>().unwrap().inputs.len(), 2);
        assert_eq!(script.steps[1].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[0], KeyInput { vk_code: VK_B.0, key_down: true });
        assert_eq!(script.steps[1].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[1], KeyInput { vk_code: VK_B.0, key_down: false });

        assert_eq!(script.steps[2].as_any().downcast_ref::<KeyInputs>().unwrap().inputs.len(), 4);
        assert_eq!(script.steps[2].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[0], KeyInput { vk_code: VK_SHIFT.vkey, key_down: true });
        assert_eq!(script.steps[2].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[1], KeyInput { vk_code: VK_K.0, key_down: true });
        assert_eq!(script.steps[2].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[2], KeyInput { vk_code: VK_K.0, key_down: false });
        assert_eq!(script.steps[2].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[3], KeyInput { vk_code: VK_SHIFT.vkey, key_down: false });
    }

    #[test]
    fn test_scan_basic_tokens() {
        let tokens = scan_shortcut_expression("ctrl a");
        assert_eq!(tokens.len(), 2);

        match &tokens[0] {
            Token::WORD(w) => assert_eq!(w, "ctrl"),
            _ => panic!("Expected WORD token"),
        }

        match &tokens[1] {
            Token::CHAR(c) => assert_eq!(c, "a"),
            _ => panic!("Expected CHAR token"),
        }
    }

    #[test]
    fn test_scan_plus_token() {
        let tokens = scan_shortcut_expression("ctrl + shift");
        assert_eq!(tokens.len(), 3);

        match &tokens[0] {
            Token::WORD(w) => assert_eq!(w, "ctrl"),
            _ => panic!("Expected WORD token"),
        }

        match &tokens[1] {
            Token::PLUS => {},
            _ => panic!("Expected PLUS token"),
        }

        match &tokens[2] {
            Token::WORD(w) => assert_eq!(w, "shift"),
            _ => panic!("Expected WORD token"),
        }
    }

    #[test]
    fn test_scan_quoted_plus() {
        let tokens = scan_shortcut_expression("'+'");
        assert_eq!(tokens.len(), 1);

        match &tokens[0] {
            Token::QUOTED(q) => assert_eq!(q, "+"),
            _ => panic!("Expected QUOTED token"),
        }
    }

    #[test]
    fn test_scan_mixed_tokens() {
        let tokens = scan_shortcut_expression("Ctrl Shift 'a' + F1");
        assert_eq!(tokens.len(), 5);

        match &tokens[0] {
            Token::WORD(w) => assert_eq!(w, "ctrl"),
            _ => panic!("Expected WORD token for Ctrl"),
        }

        match &tokens[1] {
            Token::WORD(w) => assert_eq!(w, "shift"),
            _ => panic!("Expected WORD token for Shift"),
        }

        match &tokens[2] {
            Token::QUOTED(q) => assert_eq!(q, "a"),
            _ => panic!("Expected QUOTED token for 'a'"),
        }

        match &tokens[3] {
            Token::PLUS => {},
            _ => panic!("Expected PLUS token"),
        }

        match &tokens[4] {
            Token::WORD(w) => assert_eq!(w, "f1"),
            _ => panic!("Expected WORD token for F1"),
        }
    }

    #[test]
    fn test_scan_empty_string() {
        let tokens = scan_shortcut_expression("");
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_scan_spaces_only() {
        let tokens = scan_shortcut_expression("   ");
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_parse_single_combination() {
        let combinations = parse_shortcut_expression("ctrl a");
        assert_eq!(combinations.len(), 1);
        assert_eq!(combinations[0].keys.len(), 2);
        assert_eq!(combinations[0].keys[0], VK_CTRL.vkey);
        assert_eq!(combinations[0].keys[1], VK_A.0);
    }

    #[test]
    fn test_parse_chord_combination() {
        let combinations = parse_shortcut_expression("ctrl k + ctrl b");
        assert_eq!(combinations.len(), 2);

        // First combination: Ctrl+K
        assert_eq!(combinations[0].keys.len(), 2);
        assert_eq!(combinations[0].keys[0], VK_CTRL.vkey);
        assert_eq!(combinations[0].keys[1], VK_K.0);

        // Second combination: Ctrl+B
        assert_eq!(combinations[1].keys.len(), 2);
        assert_eq!(combinations[1].keys[0], VK_CTRL.vkey);
        assert_eq!(combinations[1].keys[1], VK_B.0);
    }

    #[test]
    fn test_parse_multiple_modifiers() {
        let combinations = parse_shortcut_expression("ctrl shift alt a");
        assert_eq!(combinations.len(), 1);
        assert_eq!(combinations[0].keys.len(), 4);
        assert_eq!(combinations[0].keys[0], VK_CTRL.vkey);
        assert_eq!(combinations[0].keys[1], VK_SHIFT.vkey);
        assert_eq!(combinations[0].keys[2], VK_ALT.vkey);
        assert_eq!(combinations[0].keys[3], VK_A.0);
    }

    #[test]
    fn test_parse_empty_string() {
        let combinations = parse_shortcut_expression("");
        assert_eq!(combinations.len(), 0);
    }

    #[test]
    fn test_parse_quoted_characters() {
        let combinations = parse_shortcut_expression("'a' + 'b'");
        assert_eq!(combinations.len(), 2);
        assert_eq!(combinations[0].keys.len(), 1);
        assert_eq!(combinations[0].keys[0], VK_A.0);
        assert_eq!(combinations[1].keys.len(), 1);
        assert_eq!(combinations[1].keys[0], VK_B.0);
    }

    #[test]
    fn test_ctrl_singlequote() {
        let combinations = parse_shortcut_expression("Ctrl '");
        assert_eq!(combinations.len(), 1);
        assert_eq!(combinations[0].keys.len(), 2);
        assert_eq!(combinations[0].keys[0], VK_CTRL.vkey);
        assert_eq!(combinations[0].keys[1], VK_OEM_2.0); // VK_OEM_2 is the virtual key code for the single quote (') character
    }


    #[test]
    fn test_map_character_key_with_shift() {
        let inputs = map_character_key(VK_A.0, &ModifierState { shift: true, ..Default::default() });

        assert_eq!(inputs.len(), 4);
        assert_eq!(inputs[0], KeyInput { vk_code: VK_SHIFT.vkey, key_down: true });
        assert_eq!(inputs[1], KeyInput { vk_code: VK_A.0, key_down: true });
        assert_eq!(inputs[2], KeyInput { vk_code: VK_A.0, key_down: false });
        assert_eq!(inputs[3], KeyInput { vk_code: VK_SHIFT.vkey, key_down: false });
    }

    #[test]
    fn test_map_character_key_shift_sequence() {
        // Test mapping '!' which requires Shift+1
        let inputs = map_character_key(VK_1.0, &ModifierState { shift: true, ..Default::default() });

        assert_eq!(inputs.len(), 4);
        // Shift down, key down, key up, shift up
        assert_eq!(inputs[0], KeyInput { vk_code: VK_SHIFT.vkey, key_down: true });
        assert_eq!(inputs[1], KeyInput { vk_code: VK_1.0, key_down: true });
        assert_eq!(inputs[2], KeyInput { vk_code: VK_1.0, key_down: false });
        assert_eq!(inputs[3], KeyInput { vk_code: VK_SHIFT.vkey, key_down: false });
    }

}
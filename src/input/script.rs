use crate::app::settings::KeyboardLayout;

use super::{steps::*, keys::{vkey::{self, VK_SHIFT, VK_ENTER}, ckey::{self, CharacterKey}}};

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

struct KeyCombination<'a> {
    keys: Vec<vkey::VirtualKey<'a>>,
}

impl<'a> Default for KeyCombination<'a> {
    fn default() -> Self {
        Self { keys: Default::default() }
    }
}

use Token::*;

fn scan(text: &str) -> Vec<Token> {
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
            let len = low.len();
            let chars = low.chars().collect::<Vec<char>>();
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

fn parse<'a>(text: &'a str) -> Vec<KeyCombination<'a>> {
    scan(text.to_lowercase().as_str())
    .into_iter()
    .fold(Vec::new(), |mut acc, token| {
        match token {
            CHAR(text) | QUOTED(text) | WORD(text) => {
                if acc.is_empty() {
                    acc.push(KeyCombination::default());
                }
                acc.last_mut().unwrap().keys.extend(vkey::find_vkey(text));
            },
            PLUS => acc.push(KeyCombination::default())
        }
        acc
    })
}

pub fn for_shortcut(text: String) -> InputScript {
    log::trace!("Shortcut: {}",  text);

    let mut steps = vec![];
    for cmb in parse(text.as_str()) {
        steps.append(&mut cmb.keys.iter().map(
            |key| map_virtual_key(key.vkey, true)).collect());
        steps.append(&mut cmb.keys.iter().rev().map(
            |key| map_virtual_key(key.vkey, false)).collect());
    }

    InputScript { steps }
}

pub fn for_pause(pause: u16) -> InputScript {
    log::trace!("Pause: {}ms",  pause);
    InputScript { steps: vec![
        Box::new( NoInput { pause } )
    ] }
}

pub fn for_text(text: String, layout: KeyboardLayout) -> InputScript {
    log::trace!("Text: {}",  text);
    for_text_or_line(text, false, layout)
}

pub fn for_line(text: String, layout: KeyboardLayout) -> InputScript {
    log::trace!("Line: {}",  text);
    for_text_or_line(text, true, layout)
}

fn for_text_or_line(text: String, new_line: bool, layout: KeyboardLayout) -> InputScript {
    let ckey = ckey::with_layout(layout.mappings());

    InputScript { steps : vec![
        Box::new(KeyInputs{
            inputs : text.chars()
                    .map(|ch| ckey.find_ckey(ch))
                    .flatten()
                    .chain( new_line.then_some(CharacterKey::new(VK_ENTER)) )
                    .flat_map(|ck| map_character_key(ck) )
                    .collect()
        }) as Box<dyn InputStep>
    ] }
}

fn map_virtual_key(vk_code: u16, key_down: bool) -> Box<dyn InputStep> {
    Box::new( KeyInput { vk_code, key_down } )
}

fn map_character_key(ck: CharacterKey) -> Vec<KeyInput> {
    vec![
        ck.shift.then_some(KeyInput {vk_code:VK_SHIFT.vkey, key_down: true}),
        Some(KeyInput {vk_code:ck.vkey.vkey, key_down: true}),
        Some(KeyInput {vk_code:ck.vkey.vkey, key_down: false}),
        ck.shift.then_some(KeyInput {vk_code:VK_SHIFT.vkey, key_down: false}),
    ]
    .into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::keys::vkey::{VK_CTRL, VK_SHIFT, VK_A, VK_K, VK_B};

    #[test]
    fn test_shortcut() {
        let script = for_shortcut("Ctrl Shift A".to_string());

        assert_eq!(script.steps.len(), 6);

        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_CTRL.vkey, key_down: true });
        assert_eq!(script.steps[1].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_SHIFT.vkey, key_down: true });
        assert_eq!(script.steps[2].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_A.vkey, key_down: true });

        assert_eq!(script.steps[3].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_A.vkey, key_down: false });
        assert_eq!(script.steps[4].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_SHIFT.vkey, key_down: false });
        assert_eq!(script.steps[5].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_CTRL.vkey, key_down: false });
    }

    #[test]
    fn test_shortcurt_chord() {
        let script = for_shortcut("Ctrl K + Ctrl B".to_string());

        assert_eq!(script.steps.len(), 8);

        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_CTRL.vkey, key_down: true });
        assert_eq!(script.steps[1].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_K.vkey, key_down: true });
        assert_eq!(script.steps[2].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_K.vkey, key_down: false });
        assert_eq!(script.steps[3].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_CTRL.vkey, key_down: false });

        assert_eq!(script.steps[4].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_CTRL.vkey, key_down: true });
        assert_eq!(script.steps[5].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_B.vkey, key_down: true });
        assert_eq!(script.steps[6].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_B.vkey, key_down: false });
        assert_eq!(script.steps[7].as_any().downcast_ref::<KeyInput>().unwrap(), &KeyInput { vk_code: VK_CTRL.vkey, key_down: false });
    }

    #[test]
    fn test_text() {
        let script = for_text("abK".to_string(), KeyboardLayout::default());

        assert_eq!(script.steps.len(), 1);
        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInputs>().unwrap().inputs.len(), 8);

        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[0], KeyInput { vk_code: VK_A.vkey, key_down: true });
        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[1], KeyInput { vk_code: VK_A.vkey, key_down: false });

        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[2], KeyInput { vk_code: VK_B.vkey, key_down: true });
        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[3], KeyInput { vk_code: VK_B.vkey, key_down: false });

        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[4], KeyInput { vk_code: VK_SHIFT.vkey, key_down: true });
        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[5], KeyInput { vk_code: VK_K.vkey, key_down: true });
        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[6], KeyInput { vk_code: VK_K.vkey, key_down: false });
        assert_eq!(script.steps[0].as_any().downcast_ref::<KeyInputs>().unwrap().inputs[7], KeyInput { vk_code: VK_SHIFT.vkey, key_down: false });
    }

    #[test]
    fn test_scan_basic_tokens() {
        let tokens = scan("ctrl a");
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
        let tokens = scan("ctrl + shift");
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
        let tokens = scan("'+'");
        assert_eq!(tokens.len(), 1);

        match &tokens[0] {
            Token::QUOTED(q) => assert_eq!(q, "+"),
            _ => panic!("Expected QUOTED token"),
        }
    }

    #[test]
    fn test_scan_mixed_tokens() {
        let tokens = scan("Ctrl Shift 'a' + F1");
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
        let tokens = scan("");
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_scan_spaces_only() {
        let tokens = scan("   ");
        assert_eq!(tokens.len(), 0);
    }

    #[test]
    fn test_parse_single_combination() {
        let combinations = parse("ctrl a");
        assert_eq!(combinations.len(), 1);
        assert_eq!(combinations[0].keys.len(), 2);
        assert_eq!(combinations[0].keys[0].title, "ctrl");
        assert_eq!(combinations[0].keys[1].title, "a");
    }

    #[test]
    fn test_parse_chord_combination() {
        let combinations = parse("ctrl k + ctrl b");
        assert_eq!(combinations.len(), 2);

        // First combination: Ctrl+K
        assert_eq!(combinations[0].keys.len(), 2);
        assert_eq!(combinations[0].keys[0].title, "ctrl");
        assert_eq!(combinations[0].keys[1].title, "k");

        // Second combination: Ctrl+B
        assert_eq!(combinations[1].keys.len(), 2);
        assert_eq!(combinations[1].keys[0].title, "ctrl");
        assert_eq!(combinations[1].keys[1].title, "b");
    }

    #[test]
    fn test_parse_multiple_modifiers() {
        let combinations = parse("ctrl shift alt a");
        assert_eq!(combinations.len(), 1);
        assert_eq!(combinations[0].keys.len(), 4);
        assert_eq!(combinations[0].keys[0].title, "ctrl");
        assert_eq!(combinations[0].keys[1].title, "shift");
        assert_eq!(combinations[0].keys[2].title, "alt");
        assert_eq!(combinations[0].keys[3].title, "a");
    }

    #[test]
    fn test_parse_empty_string() {
        let combinations = parse("");
        assert_eq!(combinations.len(), 0);
    }

    #[test]
    fn test_parse_quoted_characters() {
        let combinations = parse("'a' + 'b'");
        assert_eq!(combinations.len(), 2);
        assert_eq!(combinations[0].keys.len(), 1);
        assert_eq!(combinations[0].keys[0].title, "a");
        assert_eq!(combinations[1].keys.len(), 1);
        assert_eq!(combinations[1].keys[0].title, "b");
    }

    #[test]
    fn test_map_character_key_no_shift() {
        use crate::input::keys::ckey::CharacterKey;
        use crate::input::keys::vkey::VK_A;

        let ckey = CharacterKey::new(VK_A);
        let inputs = map_character_key(ckey);

        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs[0], KeyInput { vk_code: VK_A.vkey, key_down: true });
        assert_eq!(inputs[1], KeyInput { vk_code: VK_A.vkey, key_down: false });
    }

    #[test]
    fn test_map_character_key_with_shift() {
        use crate::input::keys::ckey::CharacterKey;
        use crate::input::keys::vkey::{VK_A, VK_SHIFT};

        let ckey = CharacterKey::new_sh(VK_A);
        let inputs = map_character_key(ckey);

        assert_eq!(inputs.len(), 4);
        assert_eq!(inputs[0], KeyInput { vk_code: VK_SHIFT.vkey, key_down: true });
        assert_eq!(inputs[1], KeyInput { vk_code: VK_A.vkey, key_down: true });
        assert_eq!(inputs[2], KeyInput { vk_code: VK_A.vkey, key_down: false });
        assert_eq!(inputs[3], KeyInput { vk_code: VK_SHIFT.vkey, key_down: false });
    }

    #[test]
    fn test_map_character_key_shift_sequence() {
        use crate::input::keys::ckey::CharacterKey;
        use crate::input::keys::vkey::{VK_1, VK_SHIFT};

        // Test mapping '!' which requires Shift+1
        let ckey = CharacterKey::new_sh(VK_1);
        let inputs = map_character_key(ckey);

        assert_eq!(inputs.len(), 4);
        // Shift down, key down, key up, shift up
        assert_eq!(inputs[0], KeyInput { vk_code: VK_SHIFT.vkey, key_down: true });
        assert_eq!(inputs[1], KeyInput { vk_code: VK_1.vkey, key_down: true });
        assert_eq!(inputs[2], KeyInput { vk_code: VK_1.vkey, key_down: false });
        assert_eq!(inputs[3], KeyInput { vk_code: VK_SHIFT.vkey, key_down: false });
    }

}
use std::collections::HashMap;

use super::vkey::{self, VirtualKey, find_vkey, VK_A, VK_Z, VK_SPACE};

#[derive(Debug, Clone)]
pub struct CharacterKey<'a> {
    pub vkey: VirtualKey<'a>,
    pub shift: bool
}

impl<'a> CharacterKey<'a> {
    pub fn new(vkey: VirtualKey<'a>) -> Self {
        Self { vkey, shift: false }
    }

    pub fn new_sh(vkey: VirtualKey<'a>) -> Self {
        Self { vkey, shift: true }
    }
}

fn default_map<'a>() -> HashMap::<String, CharacterKey<'a>> {    
    let mut map = HashMap::<String, CharacterKey>::new();

    vkey::ALL_KEYS.iter()    
        .filter(|vk| VK_A.vkey <= vk.vkey && vk.vkey <= VK_Z.vkey)
        .for_each(|vk| {
            let ch = vk.title;
            map.insert(ch.to_owned(), CharacterKey::new((*vk).clone()));
            map.insert(ch.to_uppercase(), CharacterKey::new_sh((*vk).clone()));
        });

    let mut fnx = | key: &str, shkey: &str| {
        let vk = find_vkey(key.to_owned()).unwrap();
        map.insert(key.to_owned(), CharacterKey::new(vk.clone()));
        map.insert(shkey.to_owned(), CharacterKey::new_sh(vk.clone()));
    };

    fnx("1", "!");
    fnx("2", "@");
    fnx("3", "#");
    fnx("4", "$");
    fnx("5", "%");
    fnx("6", "^");
    fnx("7", "&");
    fnx("8", "*");
    fnx("9", "(");
    fnx("0", ")");

    fnx(";", ":");
    fnx("=", "+");
    fnx("-", "_");
    fnx(",", "<");
    fnx(".", ">");
    fnx("/", "?");
    fnx("`", "~");
    fnx("[", "{");
    fnx("]", "}");
    fnx("'", "\"");
    fnx("\\", "|");

    map.insert(" ".to_owned(), CharacterKey::new(VK_SPACE.clone()));
    
    map
}

pub struct WithLayout {
    mapping: HashMap<String, String>
}

impl WithLayout {
    pub fn find_ckey<'a>(&self, ch: char) -> Option<CharacterKey<'a>> {
        find_mapped_ckey(ch, &self.mapping)
    }
}

#[allow(dead_code)]
pub fn find_ckey<'a>(ch: char) -> Option<CharacterKey<'a>> {
    default_map().get(&ch.to_string()).cloned()
}

pub fn find_mapped_ckey<'a>(ch: char, mapping: &HashMap<String, String>) -> Option<CharacterKey<'a>> {
    let text = ch.to_string();
    default_map().get(
        mapping.get(&text).unwrap_or(&text)
    ).cloned()
}

pub fn with_layout(mapping:HashMap<String, String>) -> WithLayout {
    WithLayout { mapping }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::keys::vkey::{VK_A, VK_Z, VK_SPACE, VK_1, VK_SEMICOL, VK_S, VK_C, VK_D};

    #[test]
    fn test_character_key_new() {
        let ckey = CharacterKey::new(VK_A);
        assert_eq!(ckey.vkey, VK_A);
        assert_eq!(ckey.shift, false);
    }

    #[test]
    fn test_character_key_new_sh() {
        let ckey = CharacterKey::new_sh(VK_A);
        assert_eq!(ckey.vkey, VK_A);
        assert_eq!(ckey.shift, true);
    }

    #[test]
    fn test_find_ckey_lowercase_letters() {
        let ckey = find_ckey('a').unwrap();
        assert_eq!(ckey.vkey, VK_A);
        assert_eq!(ckey.shift, false);

        let ckey = find_ckey('z').unwrap();
        assert_eq!(ckey.vkey, VK_Z);
        assert_eq!(ckey.shift, false);
    }

    #[test]
    fn test_find_ckey_uppercase_letters() {
        let ckey = find_ckey('A').unwrap();
        assert_eq!(ckey.vkey, VK_A);
        assert_eq!(ckey.shift, true);

        let ckey = find_ckey('Z').unwrap();
        assert_eq!(ckey.vkey, VK_Z);
        assert_eq!(ckey.shift, true);
    }

    #[test]
    fn test_find_ckey_numbers() {
        let ckey = find_ckey('1').unwrap();
        assert_eq!(ckey.vkey, VK_1);
        assert_eq!(ckey.shift, false);
    }

    #[test]
    fn test_find_ckey_symbols() {
        let ckey = find_ckey('!').unwrap();
        assert_eq!(ckey.vkey, VK_1);
        assert_eq!(ckey.shift, true);

        let ckey = find_ckey(';').unwrap();
        assert_eq!(ckey.vkey, VK_SEMICOL);
        assert_eq!(ckey.shift, false);

        let ckey = find_ckey(':').unwrap();
        assert_eq!(ckey.vkey, VK_SEMICOL);
        assert_eq!(ckey.shift, true);
    }

    #[test]
    fn test_find_ckey_space() {
        let ckey = find_ckey(' ').unwrap();
        assert_eq!(ckey.vkey, VK_SPACE);
        assert_eq!(ckey.shift, false);
    }

    #[test]
    fn test_find_ckey_invalid_char() {
        let result = find_ckey('€');
        assert!(result.is_none());
    }

    #[test]
    fn test_default_map_contains_letters() {
        let map = default_map();
        
        assert!(map.contains_key("a"));
        assert!(map.contains_key("A"));
        assert!(map.contains_key("z"));
        assert!(map.contains_key("Z"));
        
        let lowercase_a = map.get("a").unwrap();
        assert_eq!(lowercase_a.vkey, VK_A);
        assert_eq!(lowercase_a.shift, false);
        
        let uppercase_a = map.get("A").unwrap();
        assert_eq!(uppercase_a.vkey, VK_A);
        assert_eq!(uppercase_a.shift, true);
    }

    #[test]
    fn test_default_map_contains_numbers_and_symbols() {
        let map = default_map();
        
        assert!(map.contains_key("1"));
        assert!(map.contains_key("!"));
        assert!(map.contains_key("0"));
        assert!(map.contains_key(")"));
        
        let one = map.get("1").unwrap();
        assert_eq!(one.vkey, VK_1);
        assert_eq!(one.shift, false);
        
        let exclamation = map.get("!").unwrap();
        assert_eq!(exclamation.vkey, VK_1);
        assert_eq!(exclamation.shift, true);
    }

    #[test]
    fn test_default_map_contains_punctuation() {
        let map = default_map();
        
        assert!(map.contains_key(";"));
        assert!(map.contains_key(":"));
        assert!(map.contains_key(","));
        assert!(map.contains_key("<"));
        assert!(map.contains_key(" "));
        
        let space = map.get(" ").unwrap();
        assert_eq!(space.vkey, VK_SPACE);
        assert_eq!(space.shift, false);
    }

    #[test]
    fn test_find_mapped_ckey_no_mapping() {
        let mapping = HashMap::new();
        
        let ckey = find_mapped_ckey('a', &mapping).unwrap();
        assert_eq!(ckey.vkey, VK_A);
        assert_eq!(ckey.shift, false);
    }

    #[test]
    fn test_find_mapped_ckey_with_mapping() {
        let mut mapping = HashMap::new();
        mapping.insert("š".to_owned(), "s".to_owned());
        mapping.insert("Š".to_owned(), "S".to_owned());
        
        let ckey = find_mapped_ckey('š', &mapping).unwrap();
        assert_eq!(ckey.vkey, VK_S);
        assert_eq!(ckey.shift, false);
        
        let ckey = find_mapped_ckey('Š', &mapping).unwrap();
        assert_eq!(ckey.vkey, VK_S);
        assert_eq!(ckey.shift, true);
    }

    #[test]
    fn test_find_mapped_ckey_unmapped_char() {
        let mut mapping = HashMap::new();
        mapping.insert("š".to_owned(), "s".to_owned());
        
        let result = find_mapped_ckey('€', &mapping);
        assert!(result.is_none());
    }

    #[test]
    fn test_with_layout_find_ckey() {
        let mut mapping = HashMap::new();
        mapping.insert("č".to_owned(), "c".to_owned());
        mapping.insert("Č".to_owned(), "C".to_owned());
        mapping.insert("đ".to_owned(), "d".to_owned());
        
        let layout = with_layout(mapping);
        
        let ckey = layout.find_ckey('č').unwrap();
        assert_eq!(ckey.vkey, VK_C);
        assert_eq!(ckey.shift, false);
        
        let ckey = layout.find_ckey('Č').unwrap();
        assert_eq!(ckey.vkey, VK_C);
        assert_eq!(ckey.shift, true);
        
        let ckey = layout.find_ckey('đ').unwrap();
        assert_eq!(ckey.vkey, VK_D);
        assert_eq!(ckey.shift, false);
    }

    #[test]
    fn test_with_layout_fallback_to_default() {
        let mapping = HashMap::new();
        let layout = with_layout(mapping);
        
        let ckey = layout.find_ckey('a').unwrap();
        assert_eq!(ckey.vkey, VK_A);
        assert_eq!(ckey.shift, false);
    }

    #[test]
    fn test_with_layout_invalid_char() {
        let mapping = HashMap::new();
        let layout = with_layout(mapping);
        
        let result = layout.find_ckey('€');
        assert!(result.is_none());
    }
}
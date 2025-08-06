use paste::paste;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualKey<'a> {
    pub vkey: u16,
    pub title: &'a str,
}

impl<'a> VirtualKey<'a> {
    const fn new(vkey: u16, title: &'a str) -> Self {
        Self { vkey, title }
    }
    fn matches(&self, text: &str) -> bool {
        self.title.eq(text)
    }
}

macro_rules! virtual_keys {
    ($($name:tt, $vkey:tt, $text:tt;)*) => {
        $(
            paste! {
                pub const [<VK_ $name:upper>]: VirtualKey = VirtualKey::new($vkey, $text);
            }
        )*
        pub const ALL_KEYS: &'static [&'static VirtualKey] = &[$(
            &paste!{[<VK_ $name:upper>]}
        ),*];
    }
}

virtual_keys! {
    "back",      0x08,   "back";
    "tab",       0x09,   "tab";
    "clear",     0x0C,   "clear";
    "enter",     0x0D,   "enter";
    "shift",     0x10,   "shift";
    "ctrl",      0x11,   "ctrl";
    "alt",       0x12,   "alt";
    "pause",     0x13,   "pause";
    "capslock",  0x14,   "capslock";
    "esc",       0x1B,   "esc";
    "space",     0x20,   "space";
    "pgup",      0x21,   "pgup";
    "pgdown",    0x22,   "pgdown";
    "end",       0x23,   "end";
    "home",      0x24,   "home";
    "larrow",    0x25,   "larrow";
    "uarrow",    0x26,   "uarrow";
    "rarrow",    0x27,   "rarrow";
    "darrow",    0x28,   "darrow";
    "select",    0x29,   "select";
    "print",     0x2A,   "print";
    "execute",   0x2B,   "execute";
    "prtscrn",   0x2C,   "prtscrn";
    "ins",       0x2D,   "ins";
    "del",       0x2E,   "del";
    "help",      0x2F,   "help";
    "0",         0x30,   "0";
    "1",         0x31,   "1";
    "2",         0x32,   "2";
    "3",         0x33,   "3";
    "4",         0x34,   "4";
    "5",         0x35,   "5";
    "6",         0x36,   "6";
    "7",         0x37,   "7";
    "8",         0x38,   "8";
    "9",         0x39,   "9";
    "a",         0x41,   "a";
    "b",         0x42,   "b";
    "c",         0x43,   "c";
    "d",         0x44,   "d";
    "e",         0x45,   "e";
    "f",         0x46,   "f";
    "g",         0x47,   "g";
    "h",         0x48,   "h";
    "i",         0x49,   "i";
    "j",         0x4A,   "j";
    "k",         0x4B,   "k";
    "l",         0x4C,   "l";
    "m",         0x4D,   "m";
    "n",         0x4E,   "n";
    "o",         0x4F,   "o";
    "p",         0x50,   "p";
    "q",         0x51,   "q";
    "r",         0x52,   "r";
    "s",         0x53,   "s";
    "t",         0x54,   "t";
    "u",         0x55,   "u";
    "v",         0x56,   "v";
    "w",         0x57,   "w";
    "x",         0x58,   "x";
    "y",         0x59,   "y";
    "z",         0x5A,   "z";
    "lwin",      0x5B,   "lwin";
    "rwin",      0x5C,   "rwin";
    "numpad0",   0x60,   "numpad0";
    "numpad1",   0x61,   "numpad1";
    "numpad2",   0x62,   "numpad2";
    "numpad3",   0x63,   "numpad3";
    "numpad4",   0x64,   "numpad4";
    "numpad5",   0x65,   "numpad5";
    "numpad6",   0x66,   "numpad6";
    "numpad7",   0x67,   "numpad7";
    "numpad8",   0x68,   "numpad8";
    "numpad9",   0x69,   "numpad9";
    "multiply",  0x6A,   "multiply";
    "add",       0x6B,   "add";
    "subtract",  0x6D,   "subtract";
    "decimal",   0x6E,   "decimal";
    "divide",    0x6F,   "divide";
    "f1",        0x70,   "f1";
    "f2",        0x71,   "f2";
    "f3",        0x72,   "f3";
    "f4",        0x73,   "f4";
    "f5",        0x74,   "f5";
    "f6",        0x75,   "f6";
    "f7",        0x76,   "f7";
    "f8",        0x77,   "f8";
    "f9",        0x78,   "f9";
    "f10",       0x79,   "f10";
    "f11",       0x7A,   "f11";
    "f12",       0x7B,   "f12";
    "f13",       0x7C,   "f13";
    "f14",       0x7D,   "f14";
    "f15",       0x7E,   "f15";
    "f16",       0x7F,   "f16";
    "f17",       0x80,   "f17";
    "f18",       0x81,   "f18";
    "f19",       0x82,   "f19";
    "f20",       0x83,   "f20";
    "f21",       0x84,   "f21";
    "f22",       0x85,   "f22";
    "f23",       0x86,   "f23";
    "f24",       0x87,   "f24";
    "numlock",   0x90,   "numlock";
    "scrllock",  0x91,   "scrllock";
    "lshift",    0xA0,   "lshift";
    "rshift",    0xA1,   "rshift";
    "lctrl",     0xA2,   "lctrl";
    "rctrl",     0xA3,   "rctrl";
    "lalt",      0xA4,   "lalt";
    "ralt",      0xA5,   "ralt";
    "semicol",   0xBA,   ";";
    "plus",      0xBB,   "=";
    "comma",     0xBC,   ",";
    "minus",     0xBD,   "-";
    "dot",       0xBE,   ".";
    "slash",     0xBF,   "/";
    "tick",      0xC0,   "`";
    "lsbrck",    0xDB,   "[";
    "backslash", 0xDC,   "\\";
    "rsbrck",    0xDD,   "]";
    "sqote" ,    0xDE,   "'";
}


pub fn find_vkey<'a>(text: String) -> Option<VirtualKey<'a>> {
    ALL_KEYS.iter()
    .find(|vk| vk.matches(text.as_str()))
    .cloned()
    .cloned()
}

#[cfg(test)]
mod tests {
    use crate::input::keys::vkey::*;

    #[test]
    fn test_virtual_keys() {
        assert_eq!(VK_F1, find_vkey("f1".to_owned()).unwrap());
        assert_eq!(VK_NUMLOCK, find_vkey("numlock".to_owned()).unwrap());
        assert_eq!(VK_P, find_vkey("p".to_owned()).unwrap());
    }
}
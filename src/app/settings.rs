use std::{fs, collections::HashMap};
use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::COLORREF;
use windows::Win32::UI::WindowsAndMessaging::WM_USER;

const DEFAULT_SCHEME: &str = "Blue";
const DEFAULT_TIMEOUT : f64 = 4.;
const DEFAULT_FEEDBACK : f64 = 0.;
const DEFAULT_KEYBOARD_LAYOUT: &str = "default";
const DEFAULT_OPACITY: f64 = 0.75;
const DEFAULT_BACKGROUND: &str = "#00007f";
const DEFAULT_FOREGROUND1: &str = "#5454a9";
const DEFAULT_FOREGROUND2: &str = "#dbdbec";
const DEFAULT_EDITOR: &str = "notepad.exe";

pub const WM_RELOAD_SETTINGS:u32 = WM_USER + 1;
pub const WM_OPEN_SETTINGS:u32 = WM_USER + 2;
pub const WM_HOOK_TRIGGER:u32 = WM_USER + 3;

#[derive(Serialize, Deserialize, Clone)]
pub struct Color {
    pub a: u8,
    pub r: u8,
    pub g: u8,
    pub b: u8
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ColorScheme {
    name: String,
    opacity: f64,
    background: String,
    foreground1: String, // lines
    foreground2: String, // text
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct KeyboardLayout {
    name: String,
    mappings: HashMap<String, String>
}


#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Profile {
    pub name: String,
    pub keyword: String,
    pub color_scheme: String,
    pub pads: Vec<Pad>,
    pub id: isize,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Pad {
    pub title : String,
    pub description: String,
    pub actions: Vec<Action>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Action {
    Shortcut(String),
    Text(String),
    Line(String),
    Pause(u16),
    Board(String)
}

#[derive(Debug, Clone)]
pub enum ProfileMatch {
    NameEquals(String),
    NameContains(String),
    IdEquals(isize),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LayoutSettings {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub window_style: String, // "Window" | "Floating" | "Taskbar"
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    timeout: f64,
    feedback: f64,
    editor: String,
    color_schemes: Vec<ColorScheme>,
    keyboard_layout: String,
    keyboard_layouts: Vec<KeyboardLayout>,
    profiles: Vec<Profile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    layout: Option<LayoutSettings>,
    #[serde(default, skip_serializing)]
    file_path: String,
}


impl Default for Color {
    fn default() -> Self {
        Self { a: 0, r: 0, g: 0, b: 0 }
    }
}

impl Color {
    pub fn from_hex(hex: &str) -> Option<Self> {
        let mut hex = hex.to_lowercase();
        if hex.starts_with("0x") { hex = hex[2..].to_string(); }
        if hex.starts_with("#") { hex = hex[1..].to_string(); }

        match hex.len() {
            6 => Some(Self {
                a : 0,
                r : u8::from_str_radix(&hex[0..2], 16).unwrap_or_default(),
                g : u8::from_str_radix(&hex[2..4], 16).unwrap_or_default(),
                b : u8::from_str_radix(&hex[4..6], 16).unwrap_or_default(),
            }),
            _ => None
        }
    }

    pub fn from_hex_or(hex: &str, optb: &str) -> Option<Self> {
        Self::from_hex(hex).or(Self::from_hex(optb))
    }

    pub fn to_colorref(self) -> COLORREF {
        COLORREF(
            ((self.a as u32) << 24) + ((self.b as u32) << 16) + ((self.g as u32) << 8) + (self.r as u32)
        )
    }
}

impl Default for ColorScheme {
    fn default() -> Self {
        Self {
            name: DEFAULT_SCHEME.to_owned(),
            opacity: DEFAULT_OPACITY,
            background: DEFAULT_BACKGROUND.to_owned(),
            foreground1: DEFAULT_FOREGROUND1.to_owned(),
            foreground2: DEFAULT_FOREGROUND2.to_owned(),
        }
    }
}

impl ColorScheme {
    pub fn opacity(&self) -> f64 {
        self.opacity
    }
    pub fn background(&self) -> Color {
        self.to_color(&self.background, DEFAULT_BACKGROUND)
    }
    pub fn foreground1(&self) -> Color {
        self.to_color(&self.foreground1, DEFAULT_FOREGROUND1)
    }
    pub fn foreground2(&self) -> Color {
        self.to_color(&self.foreground2, DEFAULT_FOREGROUND2)
    }

    fn to_color(&self, value: &String, default: &str) -> Color {
        Color::from_hex_or(value.as_str(), default).unwrap()
    }

}

impl Default for KeyboardLayout {
    fn default() -> Self {
        Self {
            name: DEFAULT_KEYBOARD_LAYOUT.to_owned(),
            mappings: HashMap::new()
        }
    }
}

impl KeyboardLayout {
    pub fn mappings(self) -> HashMap<String, String> {
        self.mappings
    }
}


impl Pad {
    pub fn new(title: &str, description: &str, actions: Vec<Action>) -> Self {
        Self { title: title.to_owned(), description: description.to_owned(), actions }
    }
    pub fn default() -> Self {
        Self { title: "".to_string(), description: "".to_string(), actions: vec![] }
    }
}

impl Profile {
    pub fn new(name: &str, keyword: &str, color_scheme:&str, shortcuts: Vec<Pad>, id: isize) -> Self {
        Self { name: name.to_owned(), keyword: keyword.to_owned(), color_scheme: color_scheme.to_owned(), pads: shortcuts, id }
    }

    pub fn is_match(&self, matcher: &ProfileMatch) -> bool {
        match matcher {
            ProfileMatch::NameEquals(text) => text.to_lowercase().eq(&self.keyword),
            ProfileMatch::NameContains(text) => text.to_lowercase().contains(&self.keyword),
            ProfileMatch::IdEquals(id) => *id == self.id,
        }
    }

    pub fn get_or_default(&self, index: usize) -> Pad {
        if index >= self.pads.len() {
            Pad::default()
        } else {
            self.pads[index].clone()
        }
    }
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            feedback: DEFAULT_FEEDBACK,
            editor: DEFAULT_EDITOR.to_owned(),
            color_schemes: vec![ColorScheme::default()],
            keyboard_layout: DEFAULT_KEYBOARD_LAYOUT.to_owned(),
            keyboard_layouts: vec![KeyboardLayout::default()],
            profiles: vec![],
            layout: None,
            file_path: "".to_owned()
        }
    }
}

impl Drop for Settings {
    fn drop(&mut self) {
        log::trace!("Settings dropped");
    }
}

impl Settings {
    pub fn editor(&self) -> &str {
        &self.editor
    }

    pub fn file_path(&self) -> &str {
        &self.file_path
    }

    pub fn timeout(&self) -> f64 {
        self.timeout
    }

    pub fn keyboard_layout(&self) -> KeyboardLayout {
        self.keyboard_layouts
            .iter()
            .find(|kl| kl.name == self.keyboard_layout)
            .map(|kl| kl.clone())
            .unwrap_or_default()
    }

    pub fn find_scheme(&self, name: &String) -> Option<ColorScheme> {
        self.color_schemes
            .iter()
            .find(|cs| cs.name == *name)
            .map(|cs| cs.clone())
    }

    pub fn feedback(&self) -> f64 {
        self.feedback
    }

    pub fn with_file_path(mut self, fp: &str) -> Self {
        self.file_path = fp.to_owned();
        self
    }

    pub fn reload(&mut self) {
        *self = load(self.file_path.as_str());
    }

    #[allow(dead_code)]
    pub fn save(&self) {
        self.save_to(&self.file_path)
    }

    #[allow(dead_code)]
    pub fn save_to(&self, file_path: &str) {
        let serialized = serde_json::to_string_pretty(self).unwrap();
        fs::write(file_path, serialized).unwrap();
    }

    pub fn try_match(&self, matcher: &ProfileMatch) -> Option<&Profile> {
        self.profiles.iter().find(|p| { p.is_match(matcher) })
    }

    pub fn layout(&self) -> Option<&LayoutSettings> {
        self.layout.as_ref()
    }

    pub fn set_layout(&mut self, layout: LayoutSettings) {
        self.layout = Some(layout);
    }

}

pub fn load(file_path: &str) -> Settings {
    let text = fs::read_to_string(file_path).unwrap();
    serde_json::from_str::<Settings>(&text)
            .unwrap()
            .with_file_path(file_path)
}


use serde::{Deserialize, Serialize};
use windows::Win32::Foundation::{COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{CreateFontW, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DRAW_TEXT_FORMAT, DT_CENTER, DT_LEFT, DT_RIGHT, FW_BOLD, FW_NORMAL, HFONT, OUT_DEVICE_PRECIS};

pub use crate::core::data::{ColorScheme, TextStyle};
pub use crate::input::ModifierState;

use crate::core;
use crate::ui::components::assets::Assets;


#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
pub enum PadId {
    One = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
}

impl PadId {
    pub fn from_keypad_int(value: i32) -> Self {
        match value {
            1 => PadId::One,
            2 => PadId::Two,
            3 => PadId::Three,
            4 => PadId::Four,
            5 => PadId::Five,
            6 => PadId::Six,
            7 => PadId::Seven,
            8 => PadId::Eight,
            9 => PadId::Nine,
            _ => panic!("Invalid keypad number: {}. Must be 1-9.", value),
        }
    }

    pub fn as_keypad_int(&self) -> i32 {
        *self as i32
    }

    pub fn row(&self) -> i32 {
        (self.as_keypad_int() - 1) / 3
    }

    pub fn col(&self) -> i32 {
        (self.as_keypad_int() - 1) % 3
    }


    pub fn up_to(&self) -> Vec<PadId> {
        match self {
            PadId::One => vec![PadId::One],
            PadId::Two => vec![PadId::One, PadId::Two],
            PadId::Three => vec![PadId::One, PadId::Two, PadId::Three],
            PadId::Four => vec![PadId::One, PadId::Two, PadId::Three, PadId::Four],
            PadId::Five => vec![PadId::One, PadId::Two, PadId::Three, PadId::Four, PadId::Five],
            PadId::Six => vec![PadId::One, PadId::Two, PadId::Three, PadId::Four, PadId::Five, PadId::Six],
            PadId::Seven => vec![PadId::One, PadId::Two, PadId::Three, PadId::Four, PadId::Five, PadId::Six, PadId::Seven],
            PadId::Eight => vec![PadId::One, PadId::Two, PadId::Three, PadId::Four, PadId::Five, PadId::Six, PadId::Seven, PadId::Eight],
            PadId::Nine => vec![PadId::One, PadId::Two, PadId::Three, PadId::Four, PadId::Five, PadId::Six, PadId::Seven, PadId::Eight, PadId::Nine],
        }
    }

    pub fn all() -> Vec<PadId> {
        vec![
            PadId::One, PadId::Two, PadId::Three,
            PadId::Four, PadId::Five, PadId::Six,
            PadId::Seven, PadId::Eight, PadId::Nine,
        ]
    }

    pub fn with_data(&self, pad: core::data::Pad) -> Pad {
        (*self, pad).into()
    }

    pub fn with(self, fun: impl FnOnce(&mut Pad)) -> Pad {
        let mut pad = Pad::from(self);
        fun(&mut pad);
        pad
    }

    pub fn to_string(&self) -> String {
        self.as_keypad_int().to_string()
    }
}


// New anchoring logic experiment below

#[derive(Debug, Clone, Copy)]
pub enum AnchorPin {
    NW, N, NE,
    W,  C,  E,
    SW, S, SE
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Anchor {
    NW,   NNW,  N,  NNE,   NE,
    WNW,        CN,        ENE,
    W,          C,         E,
    WSW,        CS,        ESE,
    SW,   SSW,  S,  SSE,   SE,
    Rel(f32, f32),
    Abs(i32, i32),
    Abs2(i32, i32)
}

impl AnchorPin {
    pub fn to_dt_flags(self) -> DRAW_TEXT_FORMAT {
        match self {
            AnchorPin::W | AnchorPin::NW | AnchorPin::SW => DT_LEFT,
            AnchorPin::C | AnchorPin::N | AnchorPin::S => DT_CENTER,
            AnchorPin::E | AnchorPin::NE | AnchorPin::SE => DT_RIGHT,
        }
    }

    pub fn default_for_anchor_point(anchor_point: &Anchor) -> Self {
        match anchor_point {
            Anchor::C => AnchorPin::C,
            Anchor::N => AnchorPin::N,
            Anchor::E => AnchorPin::E,
            Anchor::S => AnchorPin::S,
            Anchor::W => AnchorPin::W,
            Anchor::CN => AnchorPin::C,
            Anchor::CS => AnchorPin::C,
            Anchor::NE => AnchorPin::NE,
            Anchor::NW => AnchorPin::NW,
            Anchor::SE => AnchorPin::SE,
            Anchor::SW => AnchorPin::SW,
            Anchor::NNW => AnchorPin::N,
            Anchor::NNE => AnchorPin::N,
            Anchor::ESE => AnchorPin::E,
            Anchor::ENE => AnchorPin::E,
            Anchor::SSE => AnchorPin::S,
            Anchor::SSW => AnchorPin::S,
            Anchor::WSW => AnchorPin::W,
            Anchor::WNW => AnchorPin::W,
            Anchor::Rel(_, _) => AnchorPin::C,
            Anchor::Abs(_, _) => AnchorPin::C,
            Anchor::Abs2(_, _) => AnchorPin::C,
        }
    }
}

impl Anchor {
    pub fn to_coords(&self, rect: &RECT) -> (f32, f32) {
        let w = (rect.right - rect.left) as f32;
        let h = (rect.bottom - rect.top) as f32;
        let left = rect.left as f32;
        let top = rect.top as f32;

        let (rel_x, rel_y) = match *self {
            Anchor::NW => (0.0, 0.0),
            Anchor::NNW => (0.25, 0.0),
            Anchor::N => (0.5, 0.0),
            Anchor::NNE => (0.75, 0.0),
            Anchor::NE => (1.0, 0.0),
            Anchor::WNW => (0.0, 0.25),
            Anchor::ENE => (1.0, 0.25),
            Anchor::W => (0.0, 0.5),
            Anchor::C => (0.5, 0.5),
            Anchor::E => (1.0, 0.5),
            Anchor::WSW => (0.0, 0.75),
            Anchor::ESE => (1.0, 0.75),
            Anchor::SW => (0.0, 1.0),
            Anchor::SSW => (0.25, 1.0),
            Anchor::S => (0.5, 1.0),
            Anchor::SSE => (0.75, 1.0),
            Anchor::SE => (1.0, 1.0),
            Anchor::CN => (0.5, 0.25),
            Anchor::CS => (0.5, 0.75),
            Anchor::Rel(x, y) => (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)),
            Anchor::Abs(x, y) => return (x.clamp(rect.left, rect.right) as f32, y.clamp(rect.top, rect.bottom) as f32),
            Anchor::Abs2(x, y) => {
                // X - distance from left if positive, from right if negative
                let abs_x = if x >= 0 { (left + x as f32).clamp(left, left + w) } else { (rect.right as f32 + x as f32).clamp(left, left + w) };
                let abs_y = if y >= 0 { (top + y as f32).clamp(top, top + h) } else { (rect.bottom as f32 + y as f32).clamp(top, top + h) };
                return (abs_x, abs_y);
            },
        };

        (left + rel_x * w, top + rel_y * h)
    }

}

#[derive(Debug, Clone)]
pub struct Tag {
    pub text: String,
    pub anchor: Anchor,
    pub pin: Option<AnchorPin>,
    pub color_idx: Option<usize>,
    pub font_idx: Option<usize>,
}

impl Tag {
    pub fn get_font(&self, assets: &Assets) -> HFONT {
        if let Some(index) = self.font_idx {
            if let Some(font) = assets.palette_font(index) {
                return font;
            }
        }
        assets.tag_font()
    }

    pub fn get_color(&self, assets: &Assets) -> COLORREF {
        if let Some(index) = self.color_idx {
            if let Some(color) = assets.palette_color(index) {
                return color;
            }
        }
        assets.tag_color()
    }

    pub fn get_effective_handle(&self) -> AnchorPin {
        self.pin.unwrap_or_else(|| {
            AnchorPin::default_for_anchor_point(&self.anchor)
        })
    }
}

impl Default for Tag {
    fn default() -> Self {
        Tag {
            text: String::new(),
            anchor: Anchor::NW,
            pin: None,
            color_idx: None,
            font_idx: None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pad {
    pub pad_id: PadId,
    pub data: core::data::Pad,
    pub color_scheme: Option<ColorScheme>,
    pub text_style: Option<TextStyle>,
    pub tags: Vec<Tag>,
}

impl Default for Pad {
    fn default() -> Self {
        Pad {
            pad_id: PadId::One,
            data: core::data::Pad::default(),
            color_scheme: None,
            text_style: None,
            tags: Vec::new(),
        }
    }
}

impl From<PadId> for Pad {
    fn from(pad_id: PadId) -> Self {
        Pad {
            pad_id,
            data: core::data::Pad::default(),
            color_scheme: None,
            text_style: None,
            tags: Vec::new(),
        }
    }
}

impl Pad {
    pub fn new(pad_id: PadId, pad: core::data::Pad, color_scheme: Option<ColorScheme>, text_style: Option<TextStyle>, tags: Vec<Tag>) -> Self {
        Pad {
            pad_id,
            data: pad,
            color_scheme,
            text_style,
            tags,
        }
    }


    pub fn pad_id(&self) -> PadId {
        self.pad_id
    }
    pub fn header(&self) -> String {
        self.data.header.clone().unwrap_or_default()
    }
    pub fn text(&self) -> String {
        self.data.text.clone().unwrap_or_default()
    }
    pub fn icon(&self) -> String {
        self.data.icon.clone().unwrap_or_default()
    }
    pub fn actions(&self) -> &Vec<core::integration::ActionType> {
        &self.data.actions
    }
    pub fn board(&self) -> Option<String> {
        self.data.board.clone()
    }

    pub fn board_params(&self) -> &Vec<core::integration::Param> {
        &self.data.board_params
    }

    pub fn tags(&self) -> &Vec<Tag> {
        &self.tags
    }

    pub fn as_data(&self) -> core::data::Pad {
        let mut pad = self.data.clone();
        pad.color_scheme = self.color_scheme.as_ref().map(|cs| cs.name.clone());
        pad.text_style = self.text_style.as_ref().map(|ts| ts.name.clone());
        pad
    }

    pub fn with_data(mut self, pad: core::data::Pad) -> Self {
        self.data = pad;
        self
    }

    pub fn with_color_scheme(mut self, color_scheme: ColorScheme) -> Self {
        self.color_scheme = Some(color_scheme);
        self
    }

    pub fn with_text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = Some(text_style);
        self
    }


    pub fn with_tags(mut self, tags: Vec<Tag>) -> Self {
        self.tags.extend(tags);
        self
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8
}

impl Color {
    pub fn from_hex(hex: &str) -> Option<Self> {
        let mut hex = hex.to_lowercase();
        if hex.starts_with("0x") { hex = hex[2..].to_string(); }
        if hex.starts_with("#") { hex = hex[1..].to_string(); }

        match hex.len() {
            6 => Some(Self {
                r: u8::from_str_radix(&hex[0..2], 16).unwrap_or_default(),
                g: u8::from_str_radix(&hex[2..4], 16).unwrap_or_default(),
                b: u8::from_str_radix(&hex[4..6], 16).unwrap_or_default(),
            }),
            _ => None
        }
    }

    pub fn from_hex_or(hex: &str, optb: &str) -> Option<Self> {
        Self::from_hex(hex).or(Self::from_hex(optb))
    }

    pub fn to_hex(&self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    }

    pub fn to_rgb(&self) -> (u8, u8, u8) {
        (self.r, self.g, self.b)
    }

    pub fn to_colorref(self) -> COLORREF {
        COLORREF(
            ((self.b as u32) << 16) + ((self.g as u32) << 8) + (self.r as u32)
        )
    }

    pub fn from_colorref(color: COLORREF) -> Self {
        Self {
            r: (color.0 & 0x000000ff) as u8,
            g: ((color.0 & 0x0000ff00) >> 8) as u8,
            b: ((color.0 & 0x00ff0000) >> 16) as u8,
        }
    }

    pub fn inverted(self) -> Self {
        Self {
            r: 255 - self.r,
            g: 255 - self.g,
            b: 255 - self.b,
        }
    }
}

impl ColorScheme {
    pub fn opacity(&self) -> f64 {
        self.opacity
    }

    pub fn background(&self) -> Color {
        self.to_color(&self.background, "#00007f")
    }

    pub fn foreground1(&self) -> Color {
        self.to_color(&self.foreground1, "#5454a9")
    }

    pub fn foreground2(&self) -> Color {
        self.to_color(&self.foreground2, "#dbdbec")
    }

    pub fn tag_foreground(&self) -> Color {
        self.to_color(&self.tag_foreground, "#ff0000")
    }

    pub fn inverted(&self) -> ColorScheme {
        ColorScheme {
            name: format!("{} (inverted)", self.name),
            opacity: self.opacity,
            background: self.background().inverted().to_hex(),
            foreground1: self.foreground1().inverted().to_hex(),
            foreground2: self.foreground2().inverted().to_hex(),
            tag_foreground: self.tag_foreground().inverted().to_hex(),
            palette: self.palette.clone().into_iter().map(|c| {
                let color = self.to_color(&c, "#ff0000");
                color.inverted().to_hex()
            }).collect(),
        }
    }

    pub fn to_color(&self, value: &String, default: &str) -> Color {
        Color::from_hex_or(value.as_str(), default).unwrap()
    }

    pub fn palette(&self) -> &Vec<String> {
        &self.palette
    }

    pub fn palette_color(&self, index: usize) -> Option<Color> {
        if index < self.palette.len() {
            Color::from_hex(&self.palette[index])
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn palette_color_or<F>(&self, index: usize, fallback: F) -> Color
    where
        F: Fn(&Self) -> Color,
    {
        if index < self.palette.len() {
            Color::from_hex(&self.palette[index]).unwrap_or_else(|| fallback(self))
        } else {
            fallback(self)
        }
    }
}


impl TextStyle {
    pub fn parse_font(font_str: &str) -> (String, bool, bool, i32) {
        let parts: Vec<&str> = font_str.split_whitespace().collect();
        if parts.is_empty() {
            return ("Arial".to_string(), false, false, 12);
        }

        let size = parts.last()
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

    pub fn create_font(&self, font_str: &str) -> HFONT {
        let (face, bold, italic, size) = Self::parse_font(font_str);
        let weight = if bold { FW_BOLD.0 } else { FW_NORMAL.0 };
        let italic = if italic { 1 } else { 0 };

        unsafe {
            CreateFontW(
                size, 0, 0, 0,
                weight as i32,
                italic, 0, 0,
                DEFAULT_CHARSET,
                OUT_DEVICE_PRECIS,
                CLIP_DEFAULT_PRECIS,
                CLEARTYPE_QUALITY,
                0,
                &windows::core::HSTRING::from(face)
            )
        }
    }

    pub fn header_font(&self) -> HFONT {
        self.create_font(&self.header_font)
    }

    pub fn pad_header_font(&self) -> HFONT {
        self.create_font(&self.pad_header_font)
    }

    pub fn pad_text_font(&self) -> HFONT {
        self.create_font(&self.pad_text_font)
    }

    pub fn pad_id_font(&self) -> HFONT {
        self.create_font(&self.pad_id_font)
    }

    pub fn tag_font(&self) -> HFONT {
        self.create_font(&self.tag_font)
    }

    pub fn palette(&self) -> &Vec<String> {
        &self.palette
    }

    #[allow(dead_code)]
    pub fn palette_font(&self, index: usize) -> Option<HFONT> {
        if index < self.palette.len() {
            Some(self.create_font(&self.palette[index]))
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn palette_font_or<F>(&self, index: usize, fallback: F) -> HFONT
    where
        F: Fn(&Self) -> HFONT,
    {
        if index < self.palette.len() {
            self.create_font(&self.palette[index])
        } else {
            fallback(self)
        }
    }
}



impl From<(PadId, core::data::Pad)> for Pad {
    fn from((pad_id, pad): (PadId, core::data::Pad)) -> Self {
        Pad {
            pad_id,
            data: pad,
            color_scheme: None,
            text_style: None,
            tags: Vec::new(),
        }
    }
}
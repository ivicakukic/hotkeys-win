use std::collections::{BTreeMap, HashMap};
use serde::{Deserialize, Serialize, Serializer};

use super::integration::{ActionType, BoardType, Param};

const DEFAULT_SCHEME: &str = "default";
const DEFAULT_TEXT_STYLE: &str = "default";
const DEFAULT_OPACITY: f64 = 0.80;
const DEFAULT_BACKGROUND: &str = "#00007f";
const DEFAULT_FOREGROUND1: &str = "#6464b4";
const DEFAULT_FOREGROUND2: &str = "#dbdbec";
const DEFAULT_TAG_COLOR: &str = "#dbdbec";

const DEFAULT_HEADER_FONT: &str = "Comic Sans MS Bold 36";
const DEFAULT_PAD_HEADER_FONT: &str = "Consolas 20";
const DEFAULT_PAD_TEXT_FONT: &str = "Comic Sans MS Bold 26";
const DEFAULT_PAD_ID_FONT: &str = "Nirmala UI 18";
const DEFAULT_TAG_FONT: &str = "Consolas Bold 18";

pub const DEFAULT_TIMEOUT : u64 = 4;
pub const DEFAULT_FEEDBACK : u64 = 0;
pub const HOME_BOARD_NAME: &str = "home";
pub const DEFAULT_EDITOR: &str = "notepad.exe";

/// For use with serde's [serialize_with] attribute
fn ordered_map<S, K: Ord + Serialize, V: Serialize>(
    value: &HashMap<K, V>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    let ordered: BTreeMap<_, _> = value.iter().collect();
    ordered.serialize(serializer)
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ColorScheme {
    pub name: String,
    pub opacity: f64,
    pub background: String,
    pub foreground1: String, // lines
    pub foreground2: String, // text
    pub tag_foreground: String, // tags
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub palette: Vec<String>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct TextStyle {
    pub name: String,
    pub header_font: String, // e.g. "Impact Bold 24"
    pub pad_header_font: String, // e.g. "Consolas 14"
    pub pad_text_font: String, // e.g. "Arial Bold 16"
    pub pad_id_font: String, // e.g. "Impact Bold 16"
    pub tag_font: String, // e.g. "Consolas Bold 14"
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub palette: Vec<String>
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Detection {
    Win32(String),
    None,
}


#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
enum PadSetType {
    Static
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Board {
    #[serde(default, skip_serializing_if = "BoardType::is_static")]
    #[serde(rename = "kind")] // TODO: update to "type" both in windows and linux versions
    pub board_type: BoardType,
    pub name: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    #[serde(default)]
    pub detection: Detection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_style: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_pads: Option<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    #[serde(serialize_with = "ordered_map")]
    pub modifier_pads: HashMap<String, String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct PadSet {
    #[serde(default, skip_serializing_if = "PadSetType::is_static")]
    #[serde(rename = "kind")] // TODO: update to "type" both in windows and linux versions
    padset_type: PadSetType,

    pub name: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<Pad>,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Pad {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionType>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub board: Option<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub board_params: Vec<Param>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_style: Option<String>,
}


impl Default for BoardType {
    fn default() -> Self {
        BoardType::Static
    }
}

impl Default for PadSetType {
    fn default() -> Self {
        PadSetType::Static
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
            tag_foreground: DEFAULT_TAG_COLOR.to_owned(),
            palette: vec![]
        }
    }
}

impl Default for TextStyle {
    fn default() -> Self {
        Self {
            name: DEFAULT_TEXT_STYLE.to_owned(),
            header_font: DEFAULT_HEADER_FONT.to_owned(),
            pad_header_font: DEFAULT_PAD_HEADER_FONT.to_owned(),
            pad_text_font: DEFAULT_PAD_TEXT_FONT.to_owned(),
            pad_id_font: DEFAULT_PAD_ID_FONT.to_owned(),
            tag_font: DEFAULT_TAG_FONT.to_owned(),
            palette: vec![]
        }
    }
}

impl Default for Detection {
    fn default() -> Self {
        Detection::None
    }
}

impl BoardType {
    pub fn is_static(&self) -> bool {
        matches!(self, BoardType::Static)
    }
}

impl PadSetType {
    pub fn is_static(&self) -> bool {
        matches!(self, PadSetType::Static)
    }
}

impl Detection {
    pub fn is_match(&self, process_name: &str) -> bool {
        match self {
            Detection::Win32(keyword) => process_name.to_lowercase().contains(&keyword.to_lowercase()),
            Detection::None => false,
        }
    }
}

impl PadSet {
    pub fn new(name: &str, items: Vec<Pad>) -> Self {
        Self {
            name: name.to_owned(),
            padset_type: PadSetType::Static,
            items,
        }
    }
}

impl Board {

    // pub fn is_match(&self, matcher: &BoardMatch) -> bool {
    //     match matcher {
    //         BoardMatch::Name(name) => name.eq(&self.name),
    //         BoardMatch::Detection(process_name) => self.detection.is_match(process_name),
    //     }
    // }

    pub fn title(&self) -> &str {
        self.title.as_deref().unwrap_or(&self.name)
    }

    pub fn icon(&self) -> &str {
        self.icon.as_deref().unwrap_or("")
    }


    pub fn padset_name(&self, modifier: Option<&str>) -> Option<&str> {
        if let Some(mod_key) = modifier {
            if let Some(padset_name) = self.modifier_pads.get(mod_key) {
                return Some(padset_name);
            }
        }
        self.base_pads.as_deref()
    }

    pub fn has_modifier(&self, modifier: &str) -> bool {
        return self.modifier_pads.get(modifier).is_some()
    }

}


impl Pad {
    fn has_actions(&self) -> bool {
        !self.actions.is_empty()
    }

    fn has_board(&self) -> bool {
        self.board.is_some() && !self.board.as_deref().unwrap_or("").is_empty()
    }

    pub fn is_interactive(&self) -> bool {
        self.has_actions() || self.has_board()
    }
}
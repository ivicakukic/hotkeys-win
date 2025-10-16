use std::{collections::HashMap, fs, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::{Board, ColorScheme, PadSet, TextStyle, Resources};
use crate::core::data::{DEFAULT_EDITOR, DEFAULT_FEEDBACK, DEFAULT_TIMEOUT};
use super::validation::SettingsValidator;


#[derive(Debug, Clone)]
struct SourceMapping {
    type_name: String,
    name: String,
    source: Option<String>,
}

trait SourceMappable {
    fn type_name(&self) -> String;
    fn name(&self) -> String;
}

trait SourceMapper {
    fn map(&self, mappable: &dyn SourceMappable) -> Option<String>;
}

impl SourceMapper for Vec<SourceMapping> {
    fn map(&self, mappable: &dyn SourceMappable) -> Option<String> {
        fn find_mappable(mappings: &Vec<SourceMapping>, mappable: &dyn SourceMappable) -> Option<SourceMapping> {
            mappings.iter()
                .find(|m| m.type_name == mappable.type_name() && m.name == mappable.name())
                .cloned()
        }

        fn find_name(mappings: &Vec<SourceMapping>, name: &str) -> Option<SourceMapping> {
            mappings.iter()
                .find(|m| m.name == name)
                .cloned()
        }

        fn get_first_name(name: String) -> String {
            name.split('/').next().unwrap_or(&name).to_string()
        }

        fn default_mapping(type_name: String) -> Option<String> {
            match type_name.as_str() {
                "TextStyle" => Some("settings.styling.json".to_string()),
                "ColorScheme" => Some("settings.styling.json".to_string()),
                "KeyboardLayout" => Some("settings.layouts.json".to_string()),
                "Board" => None,
                "PadSet" => None,
                _ => None,
            }
        }

        find_mappable(self, mappable)
            .or_else(|| find_name(self, &get_first_name(mappable.name())))
            .and_then(|m| m.source)
            .or_else(|| default_mapping(mappable.type_name()))
    }
}



impl SourceMappable for TextStyle {
    fn type_name(&self) -> String { "TextStyle".to_string() }
    fn name(&self) -> String { self.name.clone() }
}
impl SourceMappable for ColorScheme {
    fn type_name(&self) -> String { "ColorScheme".to_string() }
    fn name(&self) -> String { self.name.clone() }
}
impl SourceMappable for Board {
    fn type_name(&self) -> String { "Board".to_string() }
    fn name(&self) -> String { self.name.clone() }
}
impl SourceMappable for PadSet {
    fn type_name(&self) -> String { "PadSet".to_string() }
    fn name(&self) -> String { self.name.clone() }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct LayoutSettings {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub window_style: String, // "Window" | "Floating" | "Taskbar"
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[serde(default)]
struct ComponentsData {
    color_schemes: Vec<ColorScheme>,
    text_styles: Vec<TextStyle>,
    // keyboard_layouts: Vec<KeyboardLayout>,
    boards: Vec<Board>,
    padsets: Vec<PadSet>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SettingsData {
    pub timeout: u64,
    pub feedback: u64,
    pub editor: String,
    pub color_schemes: Vec<ColorScheme>,
    pub text_styles: Vec<TextStyle>,
    pub boards: Vec<Board>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub padsets: Vec<PadSet>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout: Option<LayoutSettings>,

    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub natural_key_order: bool,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    includes: Vec<String>,

    #[serde(skip)]
    source_mappings: Vec<SourceMapping>,
}


impl Default for SettingsData {
    fn default() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            feedback: DEFAULT_FEEDBACK,
            editor: DEFAULT_EDITOR.to_owned(),
            color_schemes: vec![ColorScheme::default()],
            text_styles: vec![TextStyle::default()],
            boards: vec![],
            padsets: vec![],
            layout: None,
            natural_key_order: false,
            includes: vec![],
            source_mappings: vec![],
        }
    }
}


impl ComponentsData {
    fn all_mappings_for(&self, source: Option<String>) -> Vec<SourceMapping> {
        vec![].into_iter()
            .chain(self.text_styles.iter().map(|ts| (ts.type_name(), ts.name())))
            .chain(self.color_schemes.iter().map(|cs| (cs.type_name(), cs.name())))
            .chain(self.boards.iter().map(|b| (b.type_name(), b.name())))
            .chain(self.padsets.iter().map(|ps| (ps.type_name(), ps.name())))
            .map(|(type_name, name)| SourceMapping {
                type_name, name, source : source.clone(),
            })
            .collect()
    }

    fn separate_by(&self, mappings: &Vec<SourceMapping>) -> HashMap<Option<String>, ComponentsData> {
        let mut separated: HashMap<Option<String>, ComponentsData> = HashMap::new();

        for text_style in &self.text_styles {
            let source = mappings.map(text_style);
            separated.entry(source)
                .or_insert_with(ComponentsData::default)
                .text_styles.push(text_style.clone());
        }

        for color_scheme in &self.color_schemes {
            let source = mappings.map(color_scheme);
            separated.entry(source)
                .or_insert_with(ComponentsData::default)
                .color_schemes.push(color_scheme.clone());
        }

        for board in &self.boards {
            let source = mappings.map(board);
            separated.entry(source)
                .or_insert_with(ComponentsData::default)
                .boards.push(board.clone());
        }

        for padset in &self.padsets {
            let source = mappings.map(padset);
            separated.entry(source)
                .or_insert_with(ComponentsData::default)
                .padsets.push(padset.clone());
        }

        separated
    }
}


impl SettingsData {
    fn as_components(&self) -> ComponentsData {
        ComponentsData {
            color_schemes: self.color_schemes.clone(),
            text_styles: self.text_styles.clone(),
            boards: self.boards.clone(),
            padsets: self.padsets.clone(),
        }
    }

    fn append_all(&mut self, components: ComponentsData) {
        self.color_schemes.extend(components.color_schemes);
        self.text_styles.extend(components.text_styles);
        self.boards.extend(components.boards);
        self.padsets.extend(components.padsets);
    }

    fn set_all(&mut self, components: ComponentsData) {
        self.color_schemes = components.color_schemes;
        self.text_styles = components.text_styles;
        self.boards = components.boards;
        self.padsets = components.padsets;
    }
}


pub struct SettingsFileStroage {
    resources: Resources,
}

impl SettingsFileStroage {
    pub fn new(resources: Resources) -> Self {
        Self { resources }
    }

    /// Load settings from the main settings file and all included files
    pub fn load(&self) -> Result<SettingsData, Box<dyn std::error::Error>> {
        let settings_path: PathBuf = self.resources.settings_json().unwrap();

        if !settings_path.exists() {
            return Err(format!("Settings file does not exist: {:?}", settings_path).into());
        }

        log::info!("Loading settings: {:?}", settings_path);
        let text = fs::read_to_string(settings_path.clone())?;
        let mut settings = serde_json::from_str::<SettingsData>(&text)?;

        let mut source_mappings: Vec<SourceMapping> = vec![];
        source_mappings.extend(settings.as_components().all_mappings_for(None));
        settings.validate_name_uniquenes()
            .map_err(|e| format!("Validation error in main settings file '{:?}': {}", settings_path, e))?;

        // Load includes
        for include in &settings.includes.clone() {
            let include_path = self.resources.file(include)
                .ok_or_else(|| format!("Included settings file not found: {}", include))?;

            log::info!("Loading components: {:?}", include_path);
            let components = self.load_components(include_path.to_str().ok_or("Invalid path")?)?;
            source_mappings.extend(components.all_mappings_for(Some(include.clone())));

            // Merge components and validate uniqueness after each include
            settings.append_all(components);
            settings.validate_name_uniquenes()
                .map_err(|e| format!("Validation error in included file '{:?}': {}", include_path, e))?;
        }

        // Validate the entire settings configuration (data integrity only)
        settings.validate_data_integrity()
            .map_err(|e| format!("Settings data integrity validation failed: {}", e))?;

        // Validate resource-dependent aspects
        self.validate_icons_availability(&settings)
            .map_err(|e| format!("Icon availability validation failed: {}", e))?;

        settings.source_mappings = source_mappings;

        Ok(settings)
    }

    /// Load components from a specific file
    fn load_components(&self, file_path: &str) -> Result<ComponentsData, Box<dyn std::error::Error>> {
        let text = fs::read_to_string(file_path)?;
        let components = serde_json::from_str::<ComponentsData>(&text)?;
        Ok(components)
    }

    /// Save settings to the main settings file, separating components into their respective files
    #[allow(dead_code)]
    pub fn save(&self, settings: &SettingsData) -> Result<(), Box<dyn std::error::Error>> {
        let settings_path: PathBuf = self.resources.settings_json_or();

        // Separate components by their source mappings
        let separated = settings.as_components().separate_by(&settings.source_mappings);

        // Save each component group to its respective file
        for (source, components) in &separated {
            if let Some(ref source_file) = source {
                let text = serde_json::to_string_pretty(components)?;
                let source_path = self.resources.file(source_file)
                    .or_else(|| self.resources.new_file(source_file))
                    .ok_or_else(|| format!("Source file path not found in resources: {}", source_file))?;
                log::info!("Saving components to: {:?}", source_path);
                fs::write(source_path, text)?;
            }
        }
        // Save the main settings file with references to included files
        let mut main_settings = settings.clone();
        let main_components = separated.get(&None).cloned().unwrap_or_default();
        main_settings.set_all(main_components);
        main_settings.includes = separated.keys()
            .filter_map(|k| k.clone())
            .collect();
        main_settings.includes.sort();

        let main_text = serde_json::to_string_pretty(&main_settings)?;
        log::info!("Saving main settings to: {:?}", settings_path);
        fs::write(settings_path, main_text)?;

        Ok(())
    }

    /// Validate that all referenced icons exist in resources
    fn validate_icons_availability(&self, settings: &SettingsData) -> Result<(), String> {
        for board in &settings.boards {
            if !board.icon().is_empty() {
                if self.resources.icon(&board.icon()).is_none() {
                    return Err(format!("Icon '{}' not found for board '{}'", board.icon(), board.name));
                }
            }
        }

        for padset in &settings.padsets {
            for pad in &padset.items {
                if let Some(ref icon) = pad.icon {
                    if self.resources.icon(icon).is_none() {
                        return Err(format!("Icon '{}' not found for pad '{:?}' in padset '{}'", icon, pad.header, padset.name));
                    }
                }
            }
        }

        Ok(())
    }

}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::core::data::{TextStyle, ColorScheme, Board, PadSet};

    fn new_mapping(type_name: &str, name: &str, source: Option<&str>) -> SourceMapping {
        SourceMapping {
            type_name: type_name.to_string(),
            name: name.to_string(),
            source: source.map(|s| s.to_string()),
        }
    }

    fn new_board(name: &str) -> Board {
        let mut board = Board::default();
        board.name = name.to_string();
        board
    }

    fn new_padset(name: &str) -> PadSet {
        let mut padset = PadSet::default();
        padset.name = name.to_string();
        padset
    }

    fn new_text_style(name: &str) -> TextStyle {
        let mut text_style = TextStyle::default();
        text_style.name = name.to_string();
        text_style
    }

    fn new_color_scheme(name: &str) -> ColorScheme {
        let mut color_scheme = ColorScheme::default();
        color_scheme.name = name.to_string();
        color_scheme
    }

    #[test]
    fn test_find_mappable() {
        let mappings = vec![
            new_mapping("Board", "code", Some("settings.code.json")),
            new_mapping("PadSet", "code", Some("settings.code.json")),
            new_mapping("TextStyle", "default", Some("settings.styling.json")),
            new_mapping("ColorScheme", "default", Some("settings.styling.json")),
            new_mapping("KeyboardLayout", "default", Some("settings.layouts.json")),
            new_mapping("Board", "home", None),
        ];

        let mut text_style = TextStyle::default();
        let text_style_source = mappings.map(&text_style);
        assert_eq!(text_style_source, Some("settings.styling.json".to_string()));

        text_style.name = "custom".to_string();
        let text_style_source = mappings.map(&text_style);
        assert_eq!(text_style_source, Some("settings.styling.json".to_string()));

        let color_scheme = ColorScheme::default();
        let color_scheme_source = mappings.map(&color_scheme);
        assert_eq!(color_scheme_source, Some("settings.styling.json".to_string()));

        let mut board = Board::default();
        board.name = "code".to_string();
        let board_source = mappings.map(&board);
        assert_eq!(board_source, Some("settings.code.json".to_string()));

        board.name = "home".to_string();
        let board_source = mappings.map(&board);
        assert_eq!(board_source, None);

        board.name = "code/extra".to_string();
        let board_source = mappings.map(&board);
        assert_eq!(board_source, Some("settings.code.json".to_string()));

        board.name = "unknown".to_string();
        let board_source = mappings.map(&board);
        assert_eq!(board_source, None);

        let mut padset = PadSet::default();
        padset.name = "code".to_string();
        let padset_source = mappings.map(&padset);
        assert_eq!(padset_source, Some("settings.code.json".to_string()));

        padset.name = "home".to_string();
        let padset_source = mappings.map(&padset);
        assert_eq!(padset_source, None);

        padset.name = "code/extra".to_string();
        let padset_source = mappings.map(&padset);
        assert_eq!(padset_source, Some("settings.code.json".to_string()));

        padset.name = "unknown".to_string();
        let padset_source = mappings.map(&padset);
        assert_eq!(padset_source, None);
    }

    #[test]
    fn test_separate_by() {
        let mappings = vec![
            new_mapping("Board", "code", Some("settings.code.json")),
            new_mapping("PadSet", "code", Some("settings.code.json")),
            new_mapping("TextStyle", "default", Some("settings.styling.json")),
            new_mapping("ColorScheme", "default", Some("settings.styling.json")),
            new_mapping("Board", "home", None),
        ];

        let mut components = ComponentsData::default();
        components.boards.push(new_board("code"));
        components.boards.push(new_board("home"));
        components.boards.push(new_board("other"));

        components.padsets.push(new_padset("code"));
        components.padsets.push(new_padset("other"));

        components.text_styles.push(new_text_style("default"));
        components.text_styles.push(new_text_style("custom"));

        components.color_schemes.push(new_color_scheme("default"));
        components.color_schemes.push(new_color_scheme("custom"));


        let separated = components.separate_by(&mappings);

        assert_eq!(separated.len(), 4);

        let settings_styles = separated.get(&Some("settings.styling.json".to_string())).unwrap();
        assert_eq!(settings_styles.boards.len(), 0);
        assert_eq!(settings_styles.padsets.len(), 0);
        assert_eq!(settings_styles.text_styles.len(), 2);
        assert_eq!(settings_styles.text_styles[0].name, "default");
        assert_eq!(settings_styles.text_styles[1].name, "custom");
        assert_eq!(settings_styles.color_schemes.len(), 2);
        assert_eq!(settings_styles.color_schemes[0].name, "default");
        assert_eq!(settings_styles.color_schemes[1].name, "custom");


        let settings_code = separated.get(&Some("settings.code.json".to_string())).unwrap();
        assert_eq!(settings_code.boards.len(), 1);
        assert_eq!(settings_code.boards[0].name, "code");
        assert_eq!(settings_code.padsets.len(), 1);
        assert_eq!(settings_code.padsets[0].name, "code");
        assert_eq!(settings_code.text_styles.len(), 0);
        assert_eq!(settings_code.color_schemes.len(), 0);

        let settings_none = separated.get(&None).unwrap();
        assert_eq!(settings_none.boards.len(), 2);
        assert_eq!(settings_none.boards[0].name, "home");
        assert_eq!(settings_none.boards[1].name, "other");
        assert_eq!(settings_none.padsets.len(), 1);
        assert_eq!(settings_none.padsets[0].name, "other");
        assert_eq!(settings_none.text_styles.len(), 0);
        assert_eq!(settings_none.color_schemes.len(), 0);

    }

    #[test]
    fn test_save_load_cycle() {
        let config_dir = std::env::current_dir().unwrap().join("test_resources");
        std::fs::create_dir_all(&config_dir.clone()).unwrap();

        let resources = Resources::new(vec![config_dir.clone()]);
        let manager = SettingsFileStroage::new(resources);

        let mappings = vec![
            new_mapping("Board", "code", Some("settings.code.json")),
            new_mapping("PadSet", "code", Some("settings.code.json")),
            new_mapping("TextStyle", "default", Some("settings.styling.json")),
            new_mapping("ColorScheme", "default", Some("settings.styling.json")),
            new_mapping("KeyboardLayout", "default", Some("settings.layouts.json")),
            new_mapping("Board", "home", None),
        ];

        let mut components = ComponentsData::default();
        components.boards.push(new_board("home"));
        components.boards.push(new_board("other"));
        components.boards.push(new_board("code"));

        components.padsets.push(new_padset("other"));
        components.padsets.push(new_padset("code"));

        components.text_styles.push(new_text_style("default"));
        components.text_styles.push(new_text_style("custom"));

        components.color_schemes.push(new_color_scheme("default"));
        components.color_schemes.push(new_color_scheme("custom"));


        let mut settings = SettingsData {
            timeout: 500,
            feedback: 200,
            editor: "notepad".to_string(),
            color_schemes: vec![],
            text_styles: vec![],
            boards: vec![],
            padsets: vec![],
            layout: None,
            natural_key_order: true,
            includes: vec![],
            source_mappings: vec![],
        };

        settings.append_all(components);
        settings.source_mappings = mappings;

        // Save settings
        manager.save(&settings).unwrap();
        let loaded_settings = manager.load().unwrap();

        manager.save(&loaded_settings).unwrap();
        let reloaded_settings = manager.load().unwrap();


        // Compare original and reloaded settings
        assert_eq!(settings.timeout, reloaded_settings.timeout);
        assert_eq!(settings.feedback, reloaded_settings.feedback);
        assert_eq!(settings.editor, reloaded_settings.editor);

        assert_eq!(settings.color_schemes.len(), reloaded_settings.color_schemes.len());
        for (original, reloaded) in settings.color_schemes.iter().zip(reloaded_settings.color_schemes.iter()) {
            assert_eq!(original.name, reloaded.name);
        }

        assert_eq!(settings.text_styles.len(), reloaded_settings.text_styles.len());
        for (original, reloaded) in settings.text_styles.iter().zip(reloaded_settings.text_styles.iter()) {
            assert_eq!(original.name, reloaded.name);
        }

        assert_eq!(settings.boards.len(), reloaded_settings.boards.len());
        for (original, reloaded) in settings.boards.iter().zip(reloaded_settings.boards.iter()) {
            assert_eq!(original.name, reloaded.name);
        }

        assert_eq!(settings.padsets.len(), reloaded_settings.padsets.len());
        for (original, reloaded) in settings.padsets.iter().zip(reloaded_settings.padsets.iter()) {
            assert_eq!(original.name, reloaded.name);
        }

        // Clean up test files
        std::fs::remove_dir_all(&config_dir).unwrap();
    }
}
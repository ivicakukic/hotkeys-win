use std::collections::HashSet;

use crate::core::{ColorScheme, PadSet, TextStyle};

use super::persistence::SettingsData;
pub trait SettingsValidator {
    fn validate_data_integrity(&self) -> Result<(), String>;
    fn validate_name_uniquenes(&self) -> Result<(), String>;
}

impl SettingsValidator for SettingsData {
    fn validate_data_integrity(&self) -> Result<(), String> {
        self.validate_data_integrity()
    }
    fn validate_name_uniquenes(&self) -> Result<(), String> {
        self.validate_unique_names()
    }
}

impl SettingsData {

    fn find_scheme(&self, name: &String) -> Option<ColorScheme> {
        self.color_schemes
            .iter()
            .find(|cs| cs.name == *name)
            .map(|cs| cs.clone())
    }

    fn find_text_style(&self, name: &String) -> Option<TextStyle> {
        self.text_styles
            .iter()
            .find(|ts| ts.name == *name)
            .map(|ts| ts.clone())
    }

    fn find_padset(&self, name: &str) -> Option<&PadSet> {
        self.padsets
            .iter()
            .find(|ps| ps.name == name)
    }

    /// Validate if no two components of the same type have equal name
    fn validate_unique_names(&self) -> Result<(), String> {
        let mut seen = HashSet::new();
        for scheme in &self.color_schemes {
            if !seen.insert(scheme.name.clone()) {
                return Err(format!("Duplicate 'ColorScheme' name found: {}", scheme.name));
            }
        }

        let mut seen = HashSet::new();
        for style in &self.text_styles {
            if !seen.insert(style.name.clone()) {
                return Err(format!("Duplicate 'TextStyle' name found: {}", style.name));
            }
        }

        let mut seen = HashSet::new();
        for board in &self.boards {
            if !seen.insert(board.name.clone()) {
                return Err(format!("Duplicate 'Board' name found: {}", board.name));
            }
        }

        let mut seen = HashSet::new();
        for padset in &self.padsets {
            if !seen.insert(padset.name.clone()) {
                return Err(format!("Duplicate 'PadSet' name found: {}", padset.name));
            }
        }

        Ok(())
    }

    /// Validate color scheme references (no resource dependency)
    fn validate_color_scheme_references(&self) -> Result<(), String> {
        for board in &self.boards {
            if let Some(scheme_name) = &board.color_scheme {
                if self.find_scheme(scheme_name).is_none() {
                    return Err(format!("Color scheme '{}' for board '{}' not found in settings", scheme_name, board.name));
                }
            }
        }
        Ok(())
    }

    /// Validate text style references (no resource dependency)
    fn validate_text_style_references(&self) -> Result<(), String> {
        for board in &self.boards {
            if let Some(text_style) = &board.text_style {
                if self.find_text_style(text_style).is_none() {
                    return Err(format!("Text style '{}' for board '{}' not found in settings", text_style, board.name));
                }
            }
        }
        Ok(())
    }

    /// Validate pad references (no resource dependency)
    fn validate_pad_references(&self) -> Result<(), String> {
        for board in &self.boards {
            if let Some(ref padset_name) = board.base_pads {
                if self.find_padset(padset_name).is_none() {
                    return Err(format!("Base pad set '{}' not found for board '{}'", padset_name, board.name));
                }
            }

            for (modifier, padset_name) in &board.modifier_pads {
                if self.find_padset(padset_name).is_none() {
                    return Err(format!("Modifier pad set '{}' not found for board '{}' with modifier '{}'", padset_name, board.name, modifier));
                }
            }
        }
        Ok(())
    }

    /// Validate cross-board references (no resource dependency)
    fn validate_cross_board_references(&self) -> Result<(), String> {
        for padset in &self.padsets {
            for pad in &padset.items {
                if let Some(ref board_ref) = pad.board {
                    let found = self.boards.iter().any(|b| b.name == *board_ref);
                    if !found {
                        return Err(format!("Invalid board reference '{}' in pad '{:?}' of padset '{}'", board_ref, pad.header, padset.name));
                    }
                }

                // Validate pad-level color scheme references
                if let Some(ref scheme_name) = pad.color_scheme {
                    if self.find_scheme(scheme_name).is_none() {
                        return Err(format!("Color scheme '{}' not found for pad '{:?}' in padset '{}'", scheme_name, pad.header, padset.name));
                    }
                }

                // Validate pad-level text style references
                if let Some(ref style_name) = pad.text_style {
                    if self.find_text_style(style_name).is_none() {
                        return Err(format!("Text style '{}' not found for pad '{:?}' in padset '{}'", style_name, pad.header, padset.name));
                    }
                }
            }
        }
        Ok(())
    }

    /// Validate settings data integrity (no resource dependencies)
    fn validate_data_integrity(&self) -> Result<(), String> {
        if self.boards.is_empty() {
            return Err("No boards defined in settings".to_owned());
        }

        self.validate_color_scheme_references()
            .map_err(|e| format!("Color scheme validation failed: {}", e))?;

        self.validate_text_style_references()
            .map_err(|e| format!("Text style validation failed: {}", e))?;

        self.validate_pad_references()
            .map_err(|e| format!("Pad reference validation failed: {}", e))?;

        self.validate_cross_board_references()
            .map_err(|e| format!("Cross board validation failed: {}", e))?;

        Ok(())
    }

}

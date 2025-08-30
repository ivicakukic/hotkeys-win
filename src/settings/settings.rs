use std::cell::{RefCell, Cell};
use std::rc::Rc;

use crate::core::data::{Board, ColorScheme, Detection, PadSet, TextStyle};
use crate::core::repository::{SettingsRepository, SettingsRepositoryMut};
use crate::core::Resources;

use super::persistence::{SettingsData, SettingsFileStroage, LayoutSettings};
use crate::core::data::{HOME_BOARD_NAME};


/// Main Settings implementation - orchestrates domain and infrastructure
pub struct Settings {
    data: RefCell<SettingsData>,
    dirty: Cell<bool>,
    resources: Resources,
}

impl Settings {
    /// Create Settings from loaded data
    fn from_data(data: SettingsData, resources: Resources) -> Rc<Self> {
        let settings = Rc::new(Self {
            data: RefCell::new(data),
            dirty: Cell::new(false),
            resources,
        });

        settings
    }


    /// Load Settings from persistence
    pub fn load(resources: Resources) -> Result<Rc<Self>, Box<dyn std::error::Error>> {
        let file_storage = SettingsFileStroage::new(resources.clone());
        let data = file_storage.load()?;
        Ok(Self::from_data(data, resources))
    }

    pub fn detect(&self, detection: &str) -> Option<String> {
        let data = self.data.borrow();
        for board in &data.boards {
            if board.detection.is_match(detection) {
                return Some(board.name.clone());
            }
        }
        None
    }

    pub fn detections(&self) -> Vec<Detection> {
        self.data.borrow().boards.iter()
            .map(|b| b.detection.clone())
            // filter only Detection::Win32
            .filter(|d| d != &Detection::None)
            .collect()
    }

    pub fn home_board_name(&self) -> String {
        HOME_BOARD_NAME.to_string()
    }

    pub fn get_layout_settings(&self) -> Option<LayoutSettings> {
        self.data.borrow().layout.clone()
    }

    pub fn set_layout_settings(&self, layout: LayoutSettings) {
        self.data.borrow_mut().layout = Some(layout);
        self.mark_dirty();
    }

    pub fn get_resources(&self) -> &Resources {
        &self.resources
    }


    #[allow(dead_code)]
    pub fn modify_board<F>(&self, board_name: &str, modifier: F) -> Result<(), Box<dyn std::error::Error>>
    where
        F: FnOnce(&mut Board),
    {
        let mut data = self.data.borrow_mut();
        if let Some(board) = data.boards.iter_mut().find(|b| b.name == board_name) {
            modifier(board);
            self.mark_dirty();
            Ok(())
        } else {
            Err(format!("Board '{}' not found", board_name).into())
        }
    }

}


impl SettingsRepository for Settings {

    fn timeout(&self) -> u64 {
        self.data.borrow().timeout
    }
    fn feedback(&self) -> u64 {
        self.data.borrow().feedback
    }
    fn editor(&self) -> String {
        self.data.borrow().editor.clone()
    }

    fn get_text_style(&self, name: &str) -> Option<TextStyle> {
        self.data.borrow().text_styles.iter()
            .find(|ts| ts.name == name)
            .cloned()
    }

    fn get_color_scheme(&self, name: &str) -> Option<ColorScheme> {
        self.data.borrow().color_schemes.iter()
            .find(|cs| cs.name == name)
            .cloned()
    }


    fn resolve_color_scheme(&self, name: &Option<String>) -> ColorScheme {
        let default = ColorScheme::default();

        match name {
            None => self.get_color_scheme(&default.name),
            Some(scheme_name) => self.get_color_scheme(scheme_name)
        }.unwrap_or(default).into()
    }

    fn resolve_text_style(&self, name: &Option<String>) -> TextStyle {
        let default = TextStyle::default();

        match name {
            None => self.get_text_style(&default.name),
            Some(style_name) => self.get_text_style(style_name)
        }.unwrap_or(default).into()
    }

    fn get_board(&self, name: &str) -> Result<Board, Box<dyn std::error::Error>> {
        self.data.borrow().boards.iter()
            .find(|b| b.name == name)
            .cloned()
            .ok_or(format!("Board '{}' not found", name).into())
    }

    fn get_padset(&self, name: &str) -> Result<PadSet, Box<dyn std::error::Error>> {
        self.data.borrow().padsets.iter()
            .find(|ps| ps.name == name)
            .cloned()
            .ok_or(format!("PadSet '{}' not found", name).into())
    }

    fn color_schemes(&self) -> Vec<String> {
        self.data.borrow().color_schemes.iter().map(|cs| cs.name.clone()).collect()
    }

    fn text_styles(&self) -> Vec<String> {
        self.data.borrow().text_styles.iter().map(|ts| ts.name.clone()).collect()
    }

    fn boards(&self) -> Vec<String> {
        self.data.borrow().boards.iter().map(|b| b.name.clone()).collect()
    }

}


impl SettingsRepositoryMut for Settings {

    fn add_board(&self, board: Board) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.borrow_mut();
        if data.boards.iter().any(|b| b.name == board.name) {
            return Err(format!("Board '{}' already exists", board.name).into());
        }
        data.boards.push(board);
        self.mark_dirty();
        Ok(())
    }

    fn add_padset(&self, padset: PadSet) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.borrow_mut();
        if data.padsets.iter().any(|ps| ps.name == padset.name) {
            return Err(format!("PadSet '{}' already exists", padset.name).into());
        }
        data.padsets.push(padset);
        self.mark_dirty();
        Ok(())
    }

    fn add_color_scheme(&self, color_scheme: ColorScheme) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.borrow_mut();
        if data.color_schemes.iter().any(|cs| cs.name == color_scheme.name) {
            return Err(format!("ColorScheme '{}' already exists", color_scheme.name).into());
        }
        data.color_schemes.push(color_scheme);
        self.mark_dirty();
        Ok(())
    }

    fn add_text_style(&self, text_style: TextStyle) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.borrow_mut();
        if data.text_styles.iter().any(|ts| ts.name == text_style.name) {
            return Err(format!("TextStyle '{}' already exists", text_style.name).into());
        }
        data.text_styles.push(text_style);
        self.mark_dirty();
        Ok(())
    }

    fn delete_color_scheme(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.borrow_mut();
        if let Some(pos) = data.color_schemes.iter().position(|cs| cs.name == name) {
            data.color_schemes.remove(pos);
            self.mark_dirty();
            Ok(())
        } else {
            Err(format!("ColorScheme '{}' not found", name).into())
        }
    }

    fn delete_text_style(&self, name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.borrow_mut();
        if let Some(pos) = data.text_styles.iter().position(|ts| ts.name == name) {
            data.text_styles.remove(pos);
            self.mark_dirty();
            Ok(())
        } else {
            Err(format!("TextStyle '{}' not found", name).into())
        }
    }

    fn set_board(&self, board: Board) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.borrow_mut();
        if let Some(existing) = data.boards.iter_mut().find(|b| b.name == board.name) {
            *existing = board;
            self.mark_dirty();
            Ok(())
        } else {
            Err(format!("Board '{}' not found", board.name).into())
        }
    }

    fn set_padset(&self, padset: PadSet) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.borrow_mut();
        if let Some(existing) = data.padsets.iter_mut().find(|ps| ps.name == padset.name) {
            *existing = padset;
            self.mark_dirty();
            Ok(())
        } else {
            Err(format!("PadSet '{}' not found", padset.name).into())
        }
    }

    fn set_text_style(&self, text_style: TextStyle) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.borrow_mut();
        if let Some(existing) = data.text_styles.iter_mut().find(|ts| ts.name == text_style.name) {
            *existing = text_style;
            self.mark_dirty();
            Ok(())
        } else {
            Err(format!("TextStyle '{}' not found", text_style.name).into())
        }
    }

    fn set_color_scheme(&self, color_scheme: ColorScheme) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.borrow_mut();
        if let Some(existing) = data.color_schemes.iter_mut().find(|cs| cs.name == color_scheme.name) {
            *existing = color_scheme;
            self.mark_dirty();
            Ok(())
        } else {
            Err(format!("ColorScheme '{}' not found", color_scheme.name).into())
        }
    }

    fn rename_color_scheme(&self, old_name: &str, new_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.borrow_mut();
        if data.color_schemes.iter().any(|cs| cs.name == new_name) {
            return Err(format!("ColorScheme '{}' already exists", new_name).into());
        }

        let rename_refefence = |cs_name: &mut Option<String>| {
            if let Some(ref mut name) = cs_name {
                if name == old_name {
                    *name = new_name.to_string();
                }
            }
        };

        if let Some(existing) = data.color_schemes.iter_mut().find(|cs| cs.name == old_name) {
            existing.name = new_name.to_string();

            for board in &mut data.boards {
                rename_refefence(&mut board.color_scheme);
            }
            for padset in &mut data.padsets {
                for pad in &mut padset.items {
                    rename_refefence(&mut pad.color_scheme);
                }
            }

            self.mark_dirty();
            Ok(())
        } else {
            Err(format!("ColorScheme '{}' not found", old_name).into())
        }
    }

    fn rename_text_style(&self, old_name: &str, new_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut data = self.data.borrow_mut();
        if data.text_styles.iter().any(|ts| ts.name == new_name) {
            return Err(format!("TextStyle '{}' already exists", new_name).into());
        }
        if let Some(existing) = data.text_styles.iter_mut().find(|ts| ts.name == old_name) {
            existing.name = new_name.to_string();

            for board in &mut data.boards {
                if let Some(ref mut ts_name) = board.text_style {
                    if ts_name == old_name {
                        *ts_name = new_name.to_string();
                    }
                }
            }
            for padset in &mut data.padsets {
                for pad in &mut padset.items {
                    if let Some(ref mut ts_name) = pad.text_style {
                        if ts_name == old_name {
                            *ts_name = new_name.to_string();
                        }
                    }
                }
            }

            self.mark_dirty();
            Ok(())
        } else {
            Err(format!("TextStyle '{}' not found", old_name).into())
        }
    }

    fn mark_dirty(&self) {
        self.dirty.set(true);
    }

    fn is_dirty(&self) -> bool {
        self.dirty.get()
    }

    fn flush(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_dirty() {
            let file_storage = SettingsFileStroage::new(self.resources.clone());
            file_storage.save(&self.data.borrow())?;
            self.dirty.set(false);
        }
        Ok(())
    }

    fn reload(&self) -> Result<(), Box<dyn std::error::Error>> {
        let file_storage = SettingsFileStroage::new(self.resources.clone());
        let data = file_storage.load()?;
        *self.data.borrow_mut() = data;
        self.dirty.set(false);
        Ok(())
    }
}
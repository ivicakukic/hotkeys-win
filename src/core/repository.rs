use super::data::{Board, PadSet, TextStyle, ColorScheme};

/// Core repository interface for read operations
pub trait SettingsRepository {
    fn timeout(&self) -> u64;
    fn feedback(&self) -> u64;
    fn editor(&self) -> String;

    fn get_text_style(&self, name: &str) -> Option<TextStyle>;
    fn get_color_scheme(&self, name: &str) -> Option<ColorScheme>;
    fn get_board(&self, name: &str) -> Result<Board, Box<dyn std::error::Error>>;
    fn get_padset(&self, name: &str) -> Result<PadSet, Box<dyn std::error::Error>>;

    fn resolve_color_scheme(&self, name: &Option<String>) -> ColorScheme;
    fn resolve_text_style(&self, name: &Option<String>) -> TextStyle;

    fn color_schemes(&self) -> Vec<String>;
    fn text_styles(&self) -> Vec<String>;
    fn boards(&self) -> Vec<String>;
}


/// Repository interface for write operations
#[allow(dead_code)]
pub trait SettingsRepositoryMut {
    fn add_board(&self, board: Board) -> Result<(), Box<dyn std::error::Error>>;
    fn add_padset(&self, padset: PadSet) -> Result<(), Box<dyn std::error::Error>>;
    fn set_board(&self, board: Board) -> Result<(), Box<dyn std::error::Error>>;
    fn set_padset(&self, padset: PadSet) -> Result<(), Box<dyn std::error::Error>>;
    fn set_text_style(&self, text_style: TextStyle) -> Result<(), Box<dyn std::error::Error>>;
    fn set_color_scheme(&self, color_scheme: ColorScheme) -> Result<(), Box<dyn std::error::Error>>;
    fn add_color_scheme(&self, color_scheme: ColorScheme) -> Result<(), Box<dyn std::error::Error>>;
    fn add_text_style(&self, text_style: TextStyle) -> Result<(), Box<dyn std::error::Error>>;
    fn rename_color_scheme(&self, old_name: &str, new_name: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn rename_text_style(&self, old_name: &str, new_name: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn delete_color_scheme(&self, name: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn delete_text_style(&self, name: &str) -> Result<(), Box<dyn std::error::Error>>;
    fn mark_dirty(&self);
    fn is_dirty(&self) -> bool;
    fn flush(&self) -> Result<(), Box<dyn std::error::Error>>;
    fn reload(&self) -> Result<(), Box<dyn std::error::Error>>;
}




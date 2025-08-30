use std::cell::RefCell;
use windows::Win32::{
    Foundation::{HWND, COLORREF},
    UI::Controls::Dialogs::{ChooseColorW, CHOOSECOLORW, CC_FULLOPEN, CC_RGBINIT},
};

use crate::model::Color;

// Thread-local storage for recently used colors
// Maintains up to 16 custom colors as supported by Windows color picker
thread_local! {
    static RECENT_COLORS: RefCell<Vec<COLORREF>> = RefCell::new(Vec::new());
}

/// Result of color editor dialog
#[derive(Debug, Clone, PartialEq)]
enum DialogResult {
    Ok,
    Cancel,
}

/// Windows native color picker wrapper
struct ColorSelector {
    initial_color: Color,
    selected_color: Color,
}

impl ColorSelector {
    /// Create a new color selector with the given initial color
    fn new(initial_color: Color) -> Self {
        Self {
            initial_color: initial_color.clone(),
            selected_color: initial_color,
        }
    }

    /// Show the modal color picker dialog
    /// Returns DialogResult indicating how the dialog was closed
    fn show_modal(&mut self, parent: Option<HWND>) -> DialogResult {
        // Convert Color to COLORREF
        let (r, g, b) = self.initial_color.to_rgb();
        let initial_colorref = COLORREF((r as u32) | ((g as u32) << 8) | ((b as u32) << 16));

        // Prepare custom colors array (16 colors as required by Windows)
        let mut custom_colors = [COLORREF(0xFFFFFF); 16];

        // Fill with recently used colors
        RECENT_COLORS.with(|recent| {
            let recent_colors = recent.borrow();
            for (i, &color) in recent_colors.iter().take(16).enumerate() {
                custom_colors[i] = color;
            }
        });

        // Set up CHOOSECOLOR structure
        let mut choose_color = CHOOSECOLORW {
            lStructSize: std::mem::size_of::<CHOOSECOLORW>() as u32,
            hwndOwner: parent.unwrap_or_default(),
            rgbResult: initial_colorref,
            lpCustColors: custom_colors.as_mut_ptr(),
            Flags: CC_FULLOPEN | CC_RGBINIT, // Show full dialog with custom colors
            ..Default::default()
        };

        // Show the dialog
        let result = unsafe { ChooseColorW(&mut choose_color) };

        if result.as_bool() {
            // User clicked OK
            let selected_colorref = choose_color.rgbResult;

            // Convert COLORREF back to Color
            self.selected_color = Color::from_colorref(selected_colorref);

            // Add to recent colors if it's different from what we started with
            if selected_colorref != initial_colorref {
                self.add_to_recent_colors(selected_colorref);
            }

            DialogResult::Ok
        } else {
            // User cancelled
            DialogResult::Cancel
        }
    }

    /// Get the selected color (only valid after show_modal returns Ok)
    fn get_selected_color(&self) -> Color {
        self.selected_color.clone()
    }

    /// Add a color to the recent colors list
    fn add_to_recent_colors(&self, color: COLORREF) {
        RECENT_COLORS.with(|recent| {
            let mut recent_colors = recent.borrow_mut();

            // Remove if already exists to avoid duplicates
            recent_colors.retain(|&existing| existing != color);

            // Add to front of list
            recent_colors.insert(0, color);

            // Keep only the most recent 16 colors
            recent_colors.truncate(16);
        });
    }

    /// Get the current list of recent colors (for debugging/testing)
    #[allow(dead_code)]
    fn get_recent_colors() -> Vec<Color> {
        RECENT_COLORS.with(|recent| {
            recent
                .borrow()
                .iter()
                .map(|&colorref| Color::from_colorref(colorref))
                .collect()
        })
    }

    /// Clear the recent colors list (for testing)
    #[allow(dead_code)]
    fn clear_recent_colors() {
        RECENT_COLORS.with(|recent| {
            recent.borrow_mut().clear();
        });
    }
}

/// Convenience function to show color picker and return the result
/// Returns Some(color) if user selected a color, None if cancelled
pub fn open_color_picker(initial_color: Color, parent: Option<HWND>) -> Option<Color> {
    let mut editor = ColorSelector::new(initial_color);
    match editor.show_modal(parent) {
        DialogResult::Ok => Some(editor.get_selected_color()),
        DialogResult::Cancel => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_editor_creation() {
        let initial_color = Color { r: 255, g: 128, b: 64 };
        let editor = ColorSelector::new(initial_color.clone());
        assert_eq!(editor.initial_color, initial_color);
        assert_eq!(editor.selected_color, initial_color);
    }

    #[test]
    fn test_recent_colors_management() {
        // Clear any existing colors
        ColorSelector::clear_recent_colors();
        assert_eq!(ColorSelector::get_recent_colors().len(), 0);

        // Test adding colors
        let editor = ColorSelector::new(Color { r: 255, g: 0, b: 0 });
        editor.add_to_recent_colors(COLORREF(0x0000FF)); // Red
        editor.add_to_recent_colors(COLORREF(0x00FF00)); // Green

        let recent = ColorSelector::get_recent_colors();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0], Color { r: 0, g: 255, b: 0 }); // Green (most recent)
        assert_eq!(recent[1], Color { r: 255, g: 0, b: 0 }); // Red
    }

    #[test]
    fn test_recent_colors_deduplication() {
        ColorSelector::clear_recent_colors();

        let editor = ColorSelector::new(Color { r: 255, g: 0, b: 0 });
        let red_colorref = COLORREF(0x0000FF);

        // Add same color multiple times
        editor.add_to_recent_colors(red_colorref);
        editor.add_to_recent_colors(COLORREF(0x00FF00)); // Green
        editor.add_to_recent_colors(red_colorref); // Red again

        let recent = ColorSelector::get_recent_colors();
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0], Color { r: 255, g: 0, b: 0 }); // Red (moved to front)
        assert_eq!(recent[1], Color { r: 0, g: 255, b: 0 }); // Green
    }

    #[test]
    fn test_recent_colors_limit() {
        ColorSelector::clear_recent_colors();

        let editor = ColorSelector::new(Color { r: 0, g: 0, b: 0 });

        // Add 20 colors (more than the 16 limit)
        for i in 0..20 {
            let colorref = COLORREF(i as u32);
            editor.add_to_recent_colors(colorref);
        }

        let recent = ColorSelector::get_recent_colors();
        assert_eq!(recent.len(), 16); // Should be limited to 16
    }
}
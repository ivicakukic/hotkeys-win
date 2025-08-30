use std::collections::HashMap;

use windows::{
    Win32::{Graphics::Gdi::{HFONT, HBRUSH, HPEN, CreatePen, CreateSolidBrush, DeleteObject, PS_SOLID}, Foundation::COLORREF},
};

use crate::model::{ColorScheme, TextStyle};

pub struct Assets<'a> {
    fonts: HashMap<&'a str, HFONT>,
    brushes: HashMap<&'a str, HBRUSH>,
    pens: HashMap<&'a str, HPEN>,
    colors: HashMap<&'a str, COLORREF>,
    color_scheme: ColorScheme,
    text_style: TextStyle,
}

impl<'a> Assets<'a> {
    pub fn new(colors: &ColorScheme, text_style: &TextStyle) -> Self {
        let mut assets = Self {
            fonts: HashMap::new(),
            brushes: HashMap::new(),
            pens: HashMap::new(),
            colors: HashMap::new(),
            color_scheme: colors.clone(),
            text_style: text_style.clone(),
        };
        unsafe { assets.initialize(); }
        assets
    }

    pub fn tile_header_font(&self) -> HFONT {
        self.fonts.get("tile_header_font").unwrap().clone()
    }

    pub fn tile_text_font(&self) -> HFONT {
        self.fonts.get("tile_text_font").unwrap().clone()
    }

    pub fn tile_id_font(&self) -> HFONT {
        self.fonts.get("tile_id_font").unwrap().clone()
    }

    pub fn header_font(&self) -> HFONT {
        self.fonts.get("header_font").unwrap().clone()
    }

    pub fn background_brush(&self) -> HBRUSH {
        self.brushes.get("background_brush").unwrap().clone()
    }

    pub fn selected_tile_brush(&self) -> HBRUSH {
        self.brushes.get("selected_tile_brush").unwrap().clone()
    }

    pub fn line_pen(&self) -> HPEN {
        self.pens.get("line_pen").unwrap().clone()
    }

    pub fn font_color(&self) -> COLORREF {
        self.colors.get("font_color").unwrap().clone()
    }

    pub fn tag_color(&self) -> COLORREF {
        self.colors.get("tag_color").unwrap().clone()
    }

    pub fn tag_font(&self) -> HFONT {
        self.fonts.get("tag_font").unwrap().clone()
    }

    pub fn palette_color(&self, index: usize) -> Option<COLORREF> {
        self.colors.get(&format!("palette_color_{}", index) as &str).cloned()
    }

    #[allow(dead_code)]
    pub fn palette_color_or<F>(&self, index: usize, fallback: F) -> COLORREF
    where
        F: Fn(&Self) -> COLORREF,
    {
        self.colors.get(&format!("palette_color_{}", index) as &str).cloned().unwrap_or_else(|| fallback(self))
    }

    pub fn palette_font(&self, index: usize) -> Option<HFONT> {
        self.fonts.get(&format!("palette_font_{}", index) as &str).cloned()
    }

    #[allow(dead_code)]
    pub fn palette_font_or<F>(&self, index: usize, fallback: F) -> HFONT
    where
        F: Fn(&Self) -> HFONT,
    {
        self.fonts.get(&format!("palette_font_{}", index) as &str).cloned().unwrap_or_else(|| fallback(self))
    }

    pub fn color_scheme(&self) -> &ColorScheme {
        &self.color_scheme
    }

    pub fn text_style(&self) -> &TextStyle {
        &self.text_style
    }

    unsafe fn initialize(&mut self) {
        let colors = &self.color_scheme;
        let text_style = &self.text_style;

        let palette_color_names = vec![
            "palette_color_0",
            "palette_color_1",
            "palette_color_2",
            "palette_color_3",
            "palette_color_4",
            "palette_color_5",
            "palette_color_6",
            "palette_color_7",
            "palette_color_8",
            "palette_color_9",
        ];

        let palette_font_names = vec![
            "palette_font_0",
            "palette_font_1",
            "palette_font_2",
            "palette_font_3",
            "palette_font_4",
            "palette_font_5",
            "palette_font_6",
            "palette_font_7",
            "palette_font_8",
            "palette_font_9",
        ];

        self.colors.insert("background_color", colors.background().to_colorref());
        self.colors.insert("line_color", colors.foreground1().to_colorref());
        self.colors.insert("font_color", colors.foreground2().to_colorref());
        self.colors.insert("tag_color", colors.tag_foreground().to_colorref());
        for (i, _) in colors.palette().iter().enumerate() {
            self.colors.insert(palette_color_names[i], colors.palette_color(i).expect("Cannot fail").to_colorref());
        }

        self.brushes.insert("background_brush", CreateSolidBrush(self.colors.get("background_color").unwrap().clone()));
        self.brushes.insert("selected_tile_brush", CreateSolidBrush(self.colors.get("line_color").unwrap().clone()));
        self.pens.insert("line_pen", CreatePen(PS_SOLID, 2, self.colors.get("line_color").unwrap().clone()));

        self.fonts.insert("tile_id_font", text_style.pad_id_font());
        self.fonts.insert("tile_header_font", text_style.pad_header_font());
        self.fonts.insert("tile_text_font", text_style.pad_text_font());
        self.fonts.insert("header_font", text_style.header_font());
        self.fonts.insert("tag_font", text_style.tag_font());
        for (i, font_str) in text_style.palette().iter().enumerate() {
            self.fonts.insert(palette_font_names[i], text_style.create_font(font_str));
        }
    }

    pub unsafe fn destroy(&mut self) {
        self.brushes.iter_mut().for_each(|(_, brush)| unsafe { let _ = DeleteObject((*brush).into()); });
        self.fonts.iter_mut().for_each(|(_, font)| unsafe { let _ = DeleteObject((*font).into()); });
        self.pens.iter_mut().for_each(|(_, pen)| unsafe { let _ = DeleteObject((*pen).into()); });

        self.brushes.clear();
        self.pens.clear();
        self.pens.clear();
    }

}


impl<'a> Drop for Assets<'a> {
    fn drop(&mut self) {
        unsafe { self.destroy(); }
        log::trace!("Dropped Assets: {:p}", self);
    }
}

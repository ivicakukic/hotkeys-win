use std::collections::HashMap;

use windows::{
    Win32::{Graphics::Gdi::{HFONT, HBRUSH, HPEN, CreatePen, CreateSolidBrush, DeleteObject, CreateFontW, FW_BOLD, OUT_DEVICE_PRECIS, CLEARTYPE_QUALITY, PS_SOLID, DEFAULT_CHARSET, CLIP_DEFAULT_PRECIS}, Foundation::COLORREF},
    core::h
};

use crate::app::settings::ColorScheme;

pub struct Assets<'a> {
    fonts: HashMap<&'a str, HFONT>,
    brushes: HashMap<&'a str, HBRUSH>,
    pens: HashMap<&'a str, HPEN>,
    colors: HashMap<&'a str, COLORREF>,
}

impl<'a> Assets<'a> {
    pub fn new() -> Self {
        Self {
            fonts: HashMap::new(),
            brushes: HashMap::new(),
            pens: HashMap::new(),
            colors: HashMap::new(),
        }
    }

    pub fn from(colors: &ColorScheme) -> Self {
        let mut assets = Self::new();
        unsafe { assets.initialize(colors); }
        assets
    }

    pub fn tile_title_font(&self) -> HFONT {
        self.fonts.get("tile_title_font").unwrap().clone()
    }

    pub fn tile_desc_font(&self) -> HFONT {
        self.fonts.get("tile_desc_font").unwrap().clone()
    }

    pub fn tile_id_font(&self) -> HFONT {
        self.fonts.get("tile_id_font").unwrap().clone()
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

    unsafe fn initialize(&mut self, colors: &ColorScheme) {
        self.colors.insert("background_color", colors.background().to_colorref());
        self.colors.insert("line_color", colors.foreground1().to_colorref());
        self.colors.insert("font_color", colors.foreground2().to_colorref());

        self.brushes.insert("background_brush", CreateSolidBrush(self.colors.get("background_color").unwrap().clone()));
        self.brushes.insert("selected_tile_brush", CreateSolidBrush(self.colors.get("line_color").unwrap().clone()));
        self.pens.insert("line_pen", CreatePen(PS_SOLID, 2, self.colors.get("line_color").unwrap().clone()));

        self.fonts.insert("tile_id_font", CreateFontW(20,0,0,0,0,0,0,0,DEFAULT_CHARSET, OUT_DEVICE_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, 0,h!("Impact")));
        self.fonts.insert("tile_title_font", CreateFontW(20,0,0,0,0,0,0,0,DEFAULT_CHARSET, OUT_DEVICE_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, 0,h!("Consolas")));
        self.fonts.insert("tile_desc_font", CreateFontW(
            26,0,0,0,
            FW_BOLD.0 as i32,
            0,0,0,
            DEFAULT_CHARSET, OUT_DEVICE_PRECIS, CLIP_DEFAULT_PRECIS, CLEARTYPE_QUALITY, 0,
            h!("Helvetica")));
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

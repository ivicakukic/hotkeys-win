use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use crate::core::Resources;
use windows::Win32::Graphics::Gdi::{
    AlphaBlend, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC
};
use resvg::{usvg, tiny_skia};
use once_cell::unsync::Lazy;

pub struct SvgIcon {
    hbitmap: HBITMAP,
    size: i32,
}

impl SvgIcon {
    fn from_svg(svg_data: &[u8], size: i32, color: (u8,u8,u8), hdc: HDC) -> Option<Self> {
        // Parse + rasterize
        let mut options = usvg::Options::default();
        let color_str = format!("rgb({}, {}, {})", color.0, color.1, color.2);
        options.style_sheet = Some(format!(
            ".board-s {{ stroke: {}; }} \
             .board-f {{ fill: {}; }} \
             .board-sf {{ stroke: {}; fill: {}; }} ",
            color_str, color_str, color_str, color_str
        ));
        let tree = usvg::Tree::from_data(svg_data, &options).ok()?;

        let pixmap_size = tree.size().to_int_size();
        let scale_x = size as f64 / pixmap_size.width() as f64;
        let scale_y = size as f64 / pixmap_size.height() as f64;
        let scale = scale_x.min(scale_y);

        let mut pixmap = tiny_skia::Pixmap::new(size as u32, size as u32)?;
        resvg::render(&tree, tiny_skia::Transform::from_scale(scale as f32, scale as f32), &mut pixmap.as_mut());

        unsafe {
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: size,
                    biHeight: -size,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits: *mut core::ffi::c_void = std::ptr::null_mut();
            let hbitmap_result = CreateDIBSection(Some(hdc), &bmi, DIB_RGB_COLORS, &mut bits, None, 0);
            let hbitmap = match hbitmap_result {
                Ok(hb) => hb,
                Err(_) => return None,
            };
            if hbitmap.is_invalid() {
                return None;
            }

            let dst = std::slice::from_raw_parts_mut(bits as *mut u8, (size * size * 4) as usize);
            dst.copy_from_slice(pixmap.data());

            Some(Self { hbitmap, size })
        }
    }

    pub fn paint(&self, hdc: HDC, x: i32, y: i32) {
        unsafe {
            let hdc_mem = CreateCompatibleDC(Some(hdc));
            let oldbmp = SelectObject(hdc_mem, self.hbitmap.into());

            let blend = BLENDFUNCTION {
                BlendOp: AC_SRC_OVER as u8,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: AC_SRC_ALPHA as u8,
            };

            let _ = AlphaBlend(hdc, x, y, self.size, self.size, hdc_mem, 0, 0, self.size, self.size, blend);

            SelectObject(hdc_mem, oldbmp);
            let _ = DeleteDC(hdc_mem);
        }
    }
}

impl Drop for SvgIcon {
    fn drop(&mut self) {
        unsafe { let _ = DeleteObject(self.hbitmap.into()); }
    }
}

pub struct IconCache {
    icons: Mutex<HashMap<String, Arc<SvgIcon>>>,
    resources: Option<Resources>,
}

impl IconCache {
    fn new() -> Self {
        Self {
            icons: Mutex::new(HashMap::new()),
            resources: None,
        }
    }

    pub fn initialize(&mut self, resources: Resources) {
        self.resources = Some(resources);
    }

    pub fn clear(&self) {
        let mut map = self.icons.lock().unwrap();
        map.clear();
    }

    /// Lazy paint: load only if needed
    pub fn paint(&self, hdc: HDC, icon_name: &str, size: i32, color: (u8,u8,u8), x: i32, y: i32) {
        let cache_key = format!("{}:{}:{},{},{}", icon_name, size, color.0, color.1, color.2);
        let mut map = self.icons.lock().unwrap();

        match map.get(&cache_key) {
            Some(icon) => {
                icon.paint(hdc, x, y);
            }
            _ => {
                if let Some(ref resources) = self.resources {
                    if let Some(icon_path) = resources.icon(icon_name) {
                        if let Ok(svg_data) = std::fs::read(&icon_path) {
                            if let Some(icon) = SvgIcon::from_svg(&svg_data, size, color, hdc) {
                                let arc = Arc::new(icon);
                                arc.paint(hdc, x, y);
                                map.insert(cache_key, arc);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Global singleton
use std::cell::RefCell;

thread_local! {
    pub static ICON_CACHE: RefCell<Lazy<IconCache>> = RefCell::new(Lazy::new(|| IconCache::new()));
}

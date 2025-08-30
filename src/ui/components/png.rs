use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    cell::RefCell,
};
use crate::core::Resources;
use windows::Win32::Graphics::Gdi::{
    AlphaBlend, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, AC_SRC_ALPHA, AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC
};
use once_cell::unsync::Lazy;

pub struct PngIcon {
    hbitmap: HBITMAP,
    size: i32,
}

impl PngIcon {
    fn from_png_file(path: &str, size: i32, hdc: HDC) -> Option<Self> {
        // Read the PNG file
        let png_data = std::fs::read(path).ok()?;

        // Decode PNG using resvg's image handling (via tiny_skia which supports PNG)
        let img = image::load_from_memory(&png_data).ok()?;
        let rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();

        // Scale the image to fit the requested size while maintaining aspect ratio
        let scale_x = size as f32 / width as f32;
        let scale_y = size as f32 / height as f32;
        let scale = scale_x.min(scale_y);

        let new_width = (width as f32 * scale) as u32;
        let new_height = (height as f32 * scale) as u32;

        let resized = image::imageops::resize(&rgba_img, new_width, new_height, image::imageops::FilterType::Triangle);

        unsafe {
            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: size,
                    biHeight: -size, // Top-down DIB
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

            // Fill bitmap with transparent background
            let dst = std::slice::from_raw_parts_mut(bits as *mut u8, (size * size * 4) as usize);
            for pixel in dst.chunks_exact_mut(4) {
                pixel[0] = 0; // B
                pixel[1] = 0; // G
                pixel[2] = 0; // R
                pixel[3] = 0; // A
            }

            // Calculate centering offset
            let offset_x = (size as u32 - new_width) / 2;
            let offset_y = (size as u32 - new_height) / 2;

            // Copy resized PNG data to bitmap, converting RGBA to BGRA with premultiplied alpha and centering
            let raw_data = resized.as_raw();
            for y in 0..new_height {
                for x in 0..new_width {
                    let src_idx = ((y * new_width + x) * 4) as usize;
                    let dst_x = x + offset_x;
                    let dst_y = y + offset_y;
                    let dst_idx = ((dst_y * size as u32 + dst_x) * 4) as usize;

                    if dst_idx + 3 < dst.len() && src_idx + 3 < raw_data.len() {
                        let alpha = raw_data[src_idx + 3] as u16;

                        // Premultiply RGB with alpha for proper transparency (using integer math)
                        dst[dst_idx] = ((raw_data[src_idx + 2] as u16 * alpha) / 255) as u8;     // B
                        dst[dst_idx + 1] = ((raw_data[src_idx + 1] as u16 * alpha) / 255) as u8; // G
                        dst[dst_idx + 2] = ((raw_data[src_idx] as u16 * alpha) / 255) as u8;     // R
                        dst[dst_idx + 3] = raw_data[src_idx + 3]; // A
                    }
                }
            }

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

impl Drop for PngIcon {
    fn drop(&mut self) {
        unsafe { let _ = DeleteObject(self.hbitmap.into()); }
    }
}

pub struct PngCache {
    icons: Mutex<HashMap<String, Arc<PngIcon>>>,
    resources: Option<Resources>,
}

impl PngCache {
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
    pub fn paint(&self, hdc: HDC, icon_name: &str, size: i32, x: i32, y: i32) {
        let cache_key = format!("{}:{}", icon_name, size);
        let mut map = self.icons.lock().unwrap();

        match map.get(&cache_key) {
            Some(icon) => {
                icon.paint(hdc, x, y);
            }
            _ => {
                if let Some(ref resources) = self.resources {
                    if let Some(icon_path) = resources.icon(icon_name) {
                        if let Some(icon) = PngIcon::from_png_file(icon_path.to_str().unwrap(), size, hdc) {
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

thread_local! {
    pub static PNG_CACHE: RefCell<Lazy<PngCache>> = RefCell::new(Lazy::new(|| PngCache::new()));
}
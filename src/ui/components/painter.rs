use windows::Win32::{
    Foundation::{COLORREF, HWND, POINT, RECT},
    Graphics::Gdi::{
        DrawTextW, FillRect, Polyline, SelectObject, SetBkMode, SetTextColor, TextOutW, DT_BOTTOM, DT_CALCRECT, DT_CENTER, DT_NOCLIP, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_TOP, DT_VCENTER, DT_WORDBREAK, DT_WORD_ELLIPSIS, HDC, TRANSPARENT
    },
    UI::WindowsAndMessaging::GetClientRect,
};

use crate::model::{AnchorPin, Board, Color, ModifierState, Pad, PadId, Tag};
use super::{assets::Assets, png::PNG_CACHE, svg::ICON_CACHE};

#[repr(C)]
pub struct RGBA {
    pub b: u8, // Blue
    pub g: u8, // Green
    pub r: u8, // Red
    pub a: u8, // Alpha
}


pub struct BoardPainter<'a> {
    pub board: &'a dyn Board,
    pub timeout: u8,
    pub selected_pad: Option<PadId>,
}

struct TilePainter<'a> {
    pad_id: PadId,
    pad: &'a Pad,
    assets: &'a Assets<'a>,
}

struct HeaderPainter<'a> {
    title: &'a str,
    timeout: u8,
    assets: &'a Assets<'a>,
}

struct IconPainter {
}


fn alpha_blend_rect(pixels: &mut [RGBA], width: usize, rect: &RECT, bg_color: COLORREF, fg_color: COLORREF, bg_opacity: f32) {
    let (bg_r, bg_g, bg_b) = Color::from_colorref(bg_color).to_rgb();
    let (fg_r, fg_g, fg_b) = Color::from_colorref(fg_color).to_rgb();

    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            if x >= 0 && y >= 0 && x < width as i32 {
                let idx = y as usize * width + x as usize;
                if idx < pixels.len() {
                    let pixel = &mut pixels[idx];

                    // Unmultiply pixel colors first
                    let pixel_alpha = pixel.a as f32 / 255.0;
                    let unmult_r = if pixel_alpha > 0.0 { (pixel.r as f32 / pixel_alpha).min(255.0) } else { 0.0 };
                    let unmult_g = if pixel_alpha > 0.0 { (pixel.g as f32 / pixel_alpha).min(255.0) } else { 0.0 };
                    let unmult_b = if pixel_alpha > 0.0 { (pixel.b as f32 / pixel_alpha).min(255.0) } else { 0.0 };

                    // Calculate distance to background and foreground colors
                    let bg_dist = ((unmult_r as f32 - bg_r as f32).powi(2) +
                                  (unmult_g as f32 - bg_g as f32).powi(2) +
                                  (unmult_b as f32 - bg_b as f32).powi(2)).sqrt();

                    let fg_dist = ((unmult_r as f32 - fg_r as f32).powi(2) +
                                  (unmult_g as f32 - fg_g as f32).powi(2) +
                                  (unmult_b as f32 - fg_b as f32).powi(2)).sqrt();

                    let total_dist = bg_dist + fg_dist;

                    if total_dist > 0.0 {
                        let bg_weight = fg_dist / total_dist;
                        let fg_weight = bg_dist / total_dist;

                        let final_opacity = (bg_weight * bg_opacity) + (fg_weight * 1.0);
                        pixel.a = (final_opacity * 255.0) as u8;
                    }
                }
            }
        }
    }
}

fn set_opaque_rect(pixels: &mut [RGBA], width: usize, rect: &RECT) {
    for y in rect.top..rect.bottom {
        for x in rect.left..rect.right {
            if x >= 0 && y >= 0 && x < width as i32 {
                let idx = y as usize * width + x as usize;
                if idx < pixels.len() {
                    let pixel = &mut pixels[idx];
                    pixel.a = 255;
                }
            }
        }
    }
}

fn set_opaque_hline(pixels: &mut [RGBA], width: usize, y: i32, x1: i32, x2: i32, line_width: u8) {
    for dy in 0..line_width {
        let yy = y + dy as i32;
        for x in x1..x2 {
            if x >= 0 && yy >= 0 && x < width as i32 {
                let idx = yy as usize * width + x as usize;
                if idx < pixels.len() {
                    let pixel = &mut pixels[idx];
                    pixel.a = 255;
                }
            }
        }
    }
}

fn set_opaque_vline(pixels: &mut [RGBA], width: usize, x: i32, y1: i32, y2: i32, line_width: u8) {
    for dx in 0..line_width {
        let xx = x + dx as i32;
        for y in y1..y2 {
            if xx >= 0 && y >= 0 && xx < width as i32 {
                let idx = y as usize * width + xx as usize;
                if idx < pixels.len() {
                    let pixel = &mut pixels[idx];
                    pixel.a = 255;
                }
            }
        }
    }
}

fn draw_hline(hdc: HDC, pixels: &mut [RGBA], width: usize, y: i32, x1: i32, x2: i32, line_width: u8) {
    unsafe {
        let _ = Polyline(hdc, &[POINT{ x:x1, y:y }, POINT{ x:x2, y:y }]);
    }
    set_opaque_hline(pixels, width, y - 1, x1, x2, line_width);
}

fn draw_vline(hdc: HDC, pixels: &mut [RGBA], width: usize, x: i32, y1: i32, y2: i32, line_width: u8) {
    unsafe {
        let _ = Polyline(hdc, &[POINT{ x:x, y:y1 }, POINT{ x:x, y:y2 }]);
    }
    set_opaque_vline(pixels, width, x - 1, y1, y2, line_width);
}

// fn draw_rect(hdc: HDC, pixels: &mut [RGBA], width: usize, rect: &RECT, line_width: u8) {
//     unsafe {
//         let points = [
//             POINT{ x:rect.left, y:rect.top },
//             POINT{ x:rect.right, y:rect.top },
//             POINT{ x:rect.right, y:rect.bottom },
//             POINT{ x:rect.left, y:rect.bottom },
//             POINT{ x:rect.left, y:rect.top },
//         ];
//         let _ = Polyline(hdc, &points);
//     }
//     set_opaque_hline(pixels, width, rect.top - 1, rect.left, rect.right, line_width);
//     set_opaque_hline(pixels, width, rect.bottom - 1, rect.left, rect.right, line_width);
//     set_opaque_vline(pixels, width, rect.left - 1, rect.top, rect.bottom, line_width);
//     set_opaque_vline(pixels, width, rect.right - 1, rect.top, rect.bottom, line_width);
// }

fn resize_rect(rect: &RECT, dx: i32, dy: i32) -> RECT {
    RECT {
        left: rect.left - dx,
        right: rect.right + dx,
        top: rect.top - dy,
        bottom: rect.bottom + dy,
    }
}

impl<'a> BoardPainter<'a> {
    pub unsafe fn paint(&self, hwnd: HWND, hdc: HDC, pixels: &mut [RGBA], width: usize, modifier_state: ModifierState) {
        let mut rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut rect);

        let (w,h) = (rect.right, rect.bottom);
        let (wtile, htile) = (w/3, (h as f32/(10./3.)) as i32);

        // Create board assets locally
        let color_scheme = self.board.color_scheme();
        let text_style = self.board.text_style();
        let board_assets = Assets::new(&color_scheme, &text_style);
        // Don't fill background - it's already initialized in bitmap

        // Draw grid lines
        let hpen_original = SelectObject(hdc, board_assets.line_pen().into());
        // 3 horizontal lines
        draw_hline(hdc, pixels, width, 1*h/10, 0, w, 2);
        draw_hline(hdc, pixels, width, 4*h/10, 0, w, 2);
        draw_hline(hdc, pixels, width, 7*h/10, 0, w, 2);
        // 2 vertical lines
        draw_vline(hdc, pixels, width, 1*w/3, h/10, h, 2);
        draw_vline(hdc, pixels, width, 2*w/3, h/10, h, 2);
        // outer frame
        let frame = RECT { left: 1, right: w+1, top: 1, bottom: h+1 };
        draw_hline(hdc, pixels, width, frame.top, frame.left, frame.right, 2);
        draw_vline(hdc, pixels, width, frame.left, frame.top, frame.bottom, 2);
        draw_vline(hdc, pixels, width, frame.right, frame.top, frame.bottom, 2);
        draw_hline(hdc, pixels, width, frame.bottom, frame.left, frame.right, 2);
        SelectObject(hdc, hpen_original);

        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, board_assets.font_color()); // 0x00ffffff 0x003c3a3d
        for pad_id in PadId::all() {
            let row = pad_id.row();
            let col = pad_id.col();
            // let id = pad_id.as_keypad_int();

            let rect = RECT {
                left: wtile*col,  right: wtile*(col+1), top: h-htile*(row+1), bottom: h-htile*row
            };

            if self.selected_pad == Some(pad_id) {
                FillRect(hdc, &rect, board_assets.selected_tile_brush());
                set_opaque_rect(pixels, width, &rect);
            }

            let pad = &self.board.padset(Some(modifier_state.clone())).pad(pad_id);

            // Check if we need pad-specific assets
            let pad_assets;
            let (assets_to_use, repaint_background) = if pad.color_scheme.is_some() || pad.text_style.is_some() {
                // Create new assets with pad-specific overrides
                let color_scheme = pad.color_scheme.as_ref().unwrap_or(board_assets.color_scheme());
                let text_style = pad.text_style.as_ref().unwrap_or(board_assets.text_style());
                pad_assets = Assets::new(color_scheme, text_style);
                (&pad_assets, pad_assets.color_scheme().background != board_assets.color_scheme().background)
            } else {
                // Use board assets
                (&board_assets, false)
            };

            TilePainter { pad_id, pad, assets: assets_to_use }
                .paint(hdc, &rect, repaint_background, pixels, width);
        }

        let header_rect = RECT { left: 0, right: w, top: 0, bottom: (h as f32/10.) as i32 };
        SetTextColor(hdc, board_assets.font_color());
        HeaderPainter { title: &self.board.title(), timeout: self.timeout, assets: &board_assets }
            .paint(hdc, &header_rect, self.board.icon(), pixels, width);

        self.board.tags().iter().for_each(|tag| {
            TagPainter::draw_tag(hdc, tag, &header_rect, &board_assets, pixels, width);
        });

        // // Debugging: draw main screen anchor points
        // let new_tags = vec![Anchor::NE, Anchor::NW, Anchor::SE, Anchor::SW].into_iter().map(|p| {
        //     NewTag {
        //         text: "XXX".to_string(),
        //         anchor_handle: Some(AnchorHandle::default_for_anchor_point(&p)),
        //         anchor_point: p,
        //         color_idx: Some(1),
        //         font_idx: None,
        //     }
        // }).collect::<Vec<NewTag>>();
        // new_tags.iter().for_each(|tag| {
        //     NewTagPainter::draw_tag(hdc, tag, &rect, &board_assets, pixels, width);
        // });
    }
}

impl<'a> TilePainter<'a> {

    pub fn paint(&self, hdc: HDC, rect: &RECT, repaint_background: bool, pixels: &mut [RGBA], width: usize) {
        unsafe {
            if repaint_background {
                FillRect(hdc, rect, self.assets.background_brush());
                set_opaque_rect(pixels, width, rect);
            }
            SetTextColor(hdc, self.assets.font_color());

            let previous_font = SelectObject(hdc, self.assets.tile_id_font().into());
            let _ = TextOutW(hdc, rect.right-15, rect.bottom-25, to_wstr(&self.pad_id.to_string()).as_slice());
            let id_rect = RECT {
                left: rect.right-20,
                right: rect.right-3,
                top: rect.bottom-25,
                bottom: rect.bottom-3
            };
            alpha_blend_rect(pixels, width, &id_rect, self.assets.color_scheme().background().to_colorref(), self.assets.font_color(), self.assets.color_scheme().opacity() as f32);

            SelectObject(hdc, self.assets.tile_header_font().into());

            // Header at top of tile
            let header_height = 60; // Enough space for 3 lines
            let mut header_rect = RECT{
                left: rect.left,
                right: rect.right,
                top: rect.top,
                bottom: rect.top + header_height
            };

            DrawTextW(hdc, to_wstr(&self.pad.header()).as_mut_slice(),
                &mut header_rect, DT_CENTER | DT_TOP | DT_WORDBREAK | DT_WORD_ELLIPSIS | DT_NOPREFIX);

            // Apply alpha blending to header text
            let bg_color = self.assets.color_scheme().background().to_colorref();
            let fg_color = self.assets.font_color();
            let bg_opacity = self.assets.color_scheme().opacity();
            alpha_blend_rect(pixels, width, &resize_rect(&header_rect, -2, -1), bg_color, fg_color, bg_opacity as f32);

            // Main content area: icon or text - vertically centered in tile (independent of header)
            SelectObject(hdc, self.assets.tile_text_font().into());
            let mut text_size = RECT::default();
            DrawTextW(hdc, to_wstr(&self.pad.text()).as_mut_slice(), &mut text_size, DT_CALCRECT | DT_NOPREFIX);

            let content_rect = RECT {
                left: rect.left,
                right: rect.right,
                top: rect.top + 25,        // 25px margin from top (header area)
                bottom: rect.bottom - 25   // 25px margin from bottom (pad ID area)
            };

            let mut icon_size = 0;

            if ! self.pad.icon().is_empty() {
                // Draw icon if available
                icon_size = text_size.bottom; // Use text height as icon size
                let center_x = (content_rect.left + content_rect.right) / 2;
                let center_y = (content_rect.top + content_rect.bottom) / 2;
                IconPainter::paint(
                    hdc,
                    &self.pad.icon(),
                    self.assets.font_color(),
                    center_x - icon_size / 2,
                    center_y - icon_size / 2,
                    icon_size
                );
            }

            // Draw text
            let gap = POINT { x: (content_rect.right - content_rect.left - text_size.right)/2,
                            y: (content_rect.bottom - content_rect.top - text_size.bottom)/2 };
            let mut text_rect = RECT {
                left: content_rect.left + gap.x,
                right: content_rect.right - gap.x,
                bottom: content_rect.bottom + icon_size - gap.y,
                top: content_rect.top + icon_size + gap.y };

            DrawTextW(hdc, to_wstr(&self.pad.text()).as_mut_slice(),
                &mut text_rect, DT_WORDBREAK | DT_CENTER | DT_BOTTOM | DT_WORD_ELLIPSIS | DT_NOCLIP | DT_NOPREFIX);

            // Apply alpha blending to main text
            alpha_blend_rect(pixels, width, &text_rect, bg_color, fg_color, bg_opacity as f32);

            // Draw tags
            self.pad.tags().iter().for_each(|tag| {
                TagPainter::draw_tag(hdc, tag, rect, self.assets, pixels, width);
            });

            SelectObject(hdc, previous_font);
        }
    }
}

impl<'a> HeaderPainter<'a> {
    pub fn paint(&self, hdc: HDC, rect: &RECT, icon: Option<String>,  pixels: &mut [RGBA], width: usize) {
        unsafe {
            let previous_font = SelectObject(hdc, self.assets.header_font().into());

            if let Some(icon_path) = icon {
                if !icon_path.is_empty() {
                    // Calculate text width to know how much space we need
                    let mut text_size = RECT::default();
                    DrawTextW(hdc, to_wstr(&self.title).as_mut_slice(), &mut text_size, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);

                    let icon_size = text_size.bottom; // Use text height as icon size
                    let text_width = text_size.right;
                    let total_width = icon_size + 10 + text_width; // icon + gap + text

                    let start_x = (rect.left + rect.right - total_width) / 2;
                    let center_y = (rect.top + rect.bottom) / 2;
                    let icon_y = center_y - icon_size / 2;
                    let text_x = start_x + icon_size + 10;

                    // Draw icon
                    IconPainter::paint(
                        hdc,
                        icon_path.as_str(),
                        self.assets.font_color(),
                        start_x,
                        icon_y,
                        icon_size
                    );

                    // Draw title next to icon
                    let mut title_rect = RECT {
                        left: text_x,
                        right: text_x + text_width,
                        top: rect.top + 5,
                        bottom: rect.bottom - 5,
                    };
                    DrawTextW(hdc, to_wstr(&self.title).as_mut_slice(), &mut title_rect, DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
                    alpha_blend_rect(pixels, width, &resize_rect(&title_rect, -1, -1), self.assets.color_scheme().background().to_colorref(), self.assets.font_color(), self.assets.color_scheme().opacity() as f32);
                } else {
                    // Just draw title centered (no icon)
                    let mut title_rect = RECT {
                        left: rect.left + 10,
                        right: rect.right - 10,
                        top: rect.top + 5,
                        bottom: rect.bottom - 5,
                    };
                    DrawTextW(hdc, to_wstr(&self.title).as_mut_slice(), &mut title_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
                    alpha_blend_rect(pixels, width, &resize_rect(&title_rect, -1, -1), self.assets.color_scheme().background().to_colorref(), self.assets.font_color(), self.assets.color_scheme().opacity() as f32);
                }
            } else {
                // Just draw title centered (no icon)
                let mut title_rect = RECT {
                    left: rect.left + 10,
                    right: rect.right - 10,
                    top: rect.top + 5,
                    bottom: rect.bottom - 5,
                };
                DrawTextW(hdc, to_wstr(&self.title).as_mut_slice(), &mut title_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
                alpha_blend_rect(pixels, width, &resize_rect(&title_rect, -1, -1), self.assets.color_scheme().background().to_colorref(), self.assets.font_color(), self.assets.color_scheme().opacity() as f32);
            }

            // Draw the timeout dots, VCENTER, RIGHT
            if self.timeout > 0 {
                let timeout_text = ".".repeat(self.timeout as usize);
                let mut timeout_rect = RECT {
                    left: rect.right - 100,
                    right: rect.right - 10,
                    top: rect.top + 5,
                    bottom: rect.bottom - 5,
                };
                DrawTextW(hdc, to_wstr(&timeout_text).as_mut_slice(), &mut timeout_rect, DT_RIGHT | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
            }

            SelectObject(hdc, previous_font);
        }
    }
}

impl IconPainter {
    pub fn paint(hdc: HDC, icon_path: &str, color: COLORREF, x: i32, y: i32, size: i32) {
        if !icon_path.is_empty() {
            // Check if this is a PNG file by extension
            if icon_path.to_lowercase().ends_with(".png") {
                // Handle PNG files through cache
                PNG_CACHE.with(|cache| {
                    let cache = cache.borrow();
                    cache.paint(hdc, &icon_path, size, x, y);
                });
            } else {
                // Handle SVG files through existing cache
                let rgb_color = Color::from_colorref(color).to_rgb();
                ICON_CACHE.with(|cache| {
                    let cache = cache.borrow();
                    cache.paint(hdc, &icon_path, size, rgb_color, x, y);
                });
            }
        }
    }
}

struct TagPainter;

impl TagPainter {
    pub fn draw_tag(hdc: HDC, tag: &Tag, rect: &RECT, assets: &Assets, pixels: &mut [RGBA], width: usize) {
        unsafe {
            let font = tag.get_font(assets);
            let color = tag.get_color(assets);
            let handle = tag.get_effective_handle();

            let previous_font = SelectObject(hdc, font.into());
            let previous_color = SetTextColor(hdc, color);

            // Calculate text size for the given font
            let mut text_size = RECT::default();
            DrawTextW(hdc, to_wstr(&tag.text).as_mut_slice(), &mut text_size, DT_CALCRECT | DT_NOPREFIX);

            let text_width = text_size.right;
            let text_height = text_size.bottom;

            // Get anchor point coordinates in the target rect
            let (anchor_x, anchor_y) = tag.anchor.to_coords(rect);

            // Calculate target rect based on handle position
            let target_rect = Self::calculate_target_rect(
                anchor_x as i32,
                anchor_y as i32,
                text_width,
                text_height,
                handle
            );

            // Debugging: uncomment these, and comment out the alpha_blend_rect part below
            // FillRect(hdc, &target_rect, assets.selected_tile_brush());
            // set_opaque_rect(pixels, width, rect);

            // Use Windows text alignment within the calculated rect
            let dt_flags = handle.to_dt_flags() | DT_VCENTER | DT_NOPREFIX; //  | DT_SINGLELINE;
            let mut draw_rect = target_rect;
            DrawTextW(hdc, to_wstr(&tag.text).as_mut_slice(), &mut draw_rect, dt_flags);

            // Apply alpha blending for transparency
            let bg_color = assets.color_scheme().background().to_colorref();
            let fg_color = color;
            let bg_opacity = assets.color_scheme().opacity() as f32;
            alpha_blend_rect(pixels, width, &target_rect, bg_color, fg_color, bg_opacity);

            SelectObject(hdc, previous_font);
            SetTextColor(hdc, previous_color);
        }
    }

    fn calculate_target_rect(anchor_x: i32, anchor_y: i32, text_width: i32, text_height: i32, handle: AnchorPin) -> RECT {
        // Windows text rendering has internal margins that we need to account for
        // These values compensate for the inherent padding in DrawTextW
        let margin_x = 5; // Slight horizontal adjustment
        let margin_y_top = 2; // Slight vertical adjustment
        let margin_y_bottom = 4; // Slight vertical adjustment

        let (offset_x, offset_y) = match handle {
            AnchorPin::NW => (margin_x, margin_y_top),
            AnchorPin::N => (-text_width / 2, margin_y_top),
            AnchorPin::NE => (-text_width + margin_x, margin_y_top),
            AnchorPin::W => (margin_x, -text_height / 2),
            AnchorPin::C => (-text_width / 2, -text_height / 2),
            AnchorPin::E => (-text_width + margin_x, -text_height / 2),
            AnchorPin::SW => (margin_x, -text_height - margin_y_bottom),
            AnchorPin::S => (-text_width / 2, -text_height - margin_y_bottom),
            AnchorPin::SE => (-text_width + margin_x, -text_height - margin_y_bottom),
        };

        // let (offset_x, offset_y) = match handle {
        //     AnchorHandle::NW => (-margin_x, -margin_y),
        //     AnchorHandle::N => (-text_width / 2, -margin_y),
        //     AnchorHandle::NE => (-text_width + margin_x, -margin_y),
        //     AnchorHandle::W => (-margin_x, -text_height / 2),
        //     AnchorHandle::C => (-text_width / 2, -text_height / 2),
        //     AnchorHandle::E => (-text_width + margin_x, -text_height / 2),
        //     AnchorHandle::SW => (-margin_x, -text_height + margin_y),
        //     AnchorHandle::S => (-text_width / 2, -text_height + margin_y),
        //     AnchorHandle::SE => (-text_width + margin_x, -text_height + margin_y),
        // };

        RECT {
            left: anchor_x + offset_x,
            right: anchor_x + offset_x + text_width,
            top: anchor_y + offset_y,
            bottom: anchor_y + offset_y + text_height,
        }
    }
}

fn to_wstr(str: &str) -> Vec<u16> {
    str.encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>()
}

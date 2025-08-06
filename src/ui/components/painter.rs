use windows::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::{
        BeginPaint, EndPaint, FillRect, Polyline, SelectObject, SetBkMode, SetTextColor, DrawTextW,
        PAINTSTRUCT, TRANSPARENT, HDC, DT_VCENTER, DT_CENTER, DT_SINGLELINE, DT_RIGHT,
        TextOutW, DT_TOP, DT_CALCRECT, DT_WORDBREAK, DT_BOTTOM, DT_WORD_ELLIPSIS, DT_NOCLIP,
    },
    UI::WindowsAndMessaging::GetClientRect,
};

use crate::{app::settings::{Profile, Pad}, ui::components::assets::Assets};

pub struct BoardPainter<'a> {
    pub profile: &'a Profile,
    pub timeout: u8,
    pub selected_pad: Option<u8>,
    pub assets: &'a Assets<'a>
}

struct TilePainter<'a> {
    id: i32,
    pad: &'a Pad,
    assets: &'a Assets<'a>,
}

struct HeaderPainter<'a> {
    title: &'a str,
    timeout: u8,
    assets: &'a Assets<'a>,
}



impl<'a> BoardPainter<'a> {
    pub unsafe fn paint(&self, hwnd: HWND) {
        let mut ps = PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);

        let mut rect = RECT::default();
        let _ = GetClientRect(hwnd, &mut rect);

        let (w,h) = (rect.right, rect.bottom);
        let (wtile, htile) = (w/3, (h as f32/(10./3.)) as i32);

        FillRect(hdc, &ps.rcPaint, self.assets.background_brush());
        let hpen_original = SelectObject(hdc, self.assets.line_pen().into());
        let _ = Polyline(hdc, &[POINT{ x:0, y:1*h/10 }, POINT{ x:w, y:1*h/10 }]); // 3 hlines
        let _ = Polyline(hdc, &[POINT{ x:0, y:4*h/10 }, POINT{ x:w, y:4*h/10 }]);
        let _ = Polyline(hdc, &[POINT{ x:0, y:7*h/10 }, POINT{ x:w, y:7*h/10 }]);
        let _ = Polyline(hdc, &[POINT{ x:1*w/3, y:h/10 }, POINT{ x:1*w/3, y: h }]); // 2 vlines
        let _ = Polyline(hdc, &[POINT{ x:2*w/3, y:h/10 }, POINT{ x:2*w/3, y: h }]);
        let _ = Polyline(hdc, &[POINT{x:0,y:0}, POINT{x:w,y:0}, POINT{x:w,y:h}, POINT{x:0,y:h}, POINT{x:0,y:0}]); // box
        SelectObject(hdc, hpen_original);

        SetBkMode(hdc, TRANSPARENT);
        SetTextColor(hdc, self.assets.font_color()); // 0x00ffffff 0x003c3a3d
        for row in [0,1,2] {
            for col in [0,1,2] {
                let id = 3*row + col;

                let rect = RECT {
                    left: wtile*col,  right: wtile*(col+1), bottom: h-htile*(row+1), top: h-htile*row
                };

                if self.selected_pad == Some(id as u8) {
                    FillRect(hdc, &rect, self.assets.selected_tile_brush());
                }

                let pad = &self.profile.get_or_default(id as usize);

                TilePainter { id, pad, assets: self.assets }
                    .paint(hdc, &rect);
            }
        }

        HeaderPainter { title: &self.profile.name, timeout: self.timeout, assets: self.assets }
            .paint(hdc, &RECT { left: 0, right: w, top: 0, bottom: (h as f32/10.) as i32 });

        let _ = EndPaint(hwnd, &ps);
    }
}

impl<'a> TilePainter<'a> {

    pub fn paint(&self, hdc: HDC, rect: &RECT) {
        unsafe {
            let previous_font = SelectObject(hdc, self.assets.tile_id_font().into());
            let _ = TextOutW(hdc, rect.right-15, rect.top-25, to_wstr(&(self.id+1).to_string()).as_slice());

            SelectObject(hdc, self.assets.tile_title_font().into());
            DrawTextW(hdc, to_wstr(&self.pad.title).as_mut_slice(),
                &mut RECT{ left: rect.left, right:rect.right, bottom: rect.bottom, top: rect.bottom + 25},
                DT_VCENTER | DT_CENTER | DT_TOP | DT_SINGLELINE);

            SelectObject(hdc, self.assets.tile_desc_font().into());
            let mut size = RECT::default();
            DrawTextW(hdc, to_wstr(&self.pad.description).as_mut_slice(), &mut size, DT_CALCRECT);

            let rect = RECT { left: rect.left, right: rect.right, bottom: rect.bottom + 25, top: rect.top };
            let gap = POINT { x: (rect.right - rect.left - size.right)/2,
                              y: (rect.top - rect.bottom - size.bottom)/2 };
            let mut rect = RECT { left: rect.left + gap.x, right: rect.right - gap.x,
                                  bottom: rect.bottom + gap.y - size.bottom, top: rect.top - gap.y - size.bottom };

            DrawTextW(hdc, to_wstr(&self.pad.description).as_mut_slice(),
                &mut rect, DT_WORDBREAK | DT_CENTER | DT_BOTTOM | DT_WORD_ELLIPSIS | DT_NOCLIP);

            SelectObject(hdc, previous_font);
        }
    }
}

impl<'a> HeaderPainter<'a> {
    pub fn paint(&self, hdc: HDC, rect: &RECT) {
        unsafe {
            let previous_font = SelectObject(hdc, self.assets.tile_id_font().into());

            SelectObject(hdc, self.assets.tile_desc_font().into());

            // Draw the title, VCENTER, CENTER
            let mut title_rect = RECT {
                left: rect.left + 10,
                right: rect.right - 10,
                top: rect.top + 5,
                bottom: rect.bottom - 5,
            };
            DrawTextW(hdc, to_wstr(&self.title).as_mut_slice(), &mut title_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

            // Draw the timeout dots, VCENTER, RIGHT
            if self.timeout > 0 {
                let timeout_text = ".".repeat(self.timeout as usize);
                let mut timeout_rect = RECT {
                    left: rect.right - 100,
                    right: rect.right - 10,
                    top: rect.top + 5,
                    bottom: rect.bottom - 5,
                };
                DrawTextW(hdc, to_wstr(&timeout_text).as_mut_slice(), &mut timeout_rect, DT_RIGHT | DT_VCENTER | DT_SINGLELINE);
            }

            SelectObject(hdc, previous_font);
        }
    }
}

fn to_wstr(str: &str) -> Vec<u16> {
    str.encode_utf16()
        .chain(Some(0))
        .collect::<Vec<_>>()
}

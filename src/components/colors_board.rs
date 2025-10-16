use std::{cell::RefCell, rc::Rc};

use windows::Win32::{Foundation::RECT, Graphics::Gdi::{DrawTextW, SelectObject, DT_CALCRECT, DT_NOPREFIX, HDC}, UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_C, VK_D, VK_DELETE, VK_DOWN, VK_E, VK_ESCAPE, VK_F2, VK_LEFT, VK_R, VK_RETURN, VK_RIGHT, VK_S, VK_UP}};

use super::{
    BoardComponent, ChildWindowRequest, DelegatingBoard, HasBoard, UiEvent, UiEventHandler, UiEventResult, EnumAll, EnumTraversal,
    apply_bool, apply_string, string_editor_board, yes_no_question_board,
    HSlider, NumericSpinnerPad, Tags,
};

use crate::{
    core::{self, SettingsRepository, SettingsRepositoryMut}, impl_board_component, impl_board_component_generic, impl_has_board,
    input::{ModifierState},
    model::{Anchor, AnchorPin, Board, Color, ColorScheme, ColorSchemeHandle, Pad, PadId, PadSet, Tag, TextStyle},
    ui::dialogs::open_color_picker
};


pub struct ColorSchemeEditorBoard<R: SettingsRepository + SettingsRepositoryMut> {
    handle: ColorSchemeHandle<R>,
    repository: Rc<R>,
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> ColorSchemeEditorBoard<R> {
    pub fn new(repository: Rc<R>, name: Option<String>) -> Self {
        Self {
            handle: ColorSchemeHandle::new(repository.clone(), name),
            repository,
        }
    }

    fn key_down(&mut self, key: u32, _: ModifierState) -> UiEventResult {
        let vk_code = VIRTUAL_KEY(key as u16);
        match vk_code {
            VK_LEFT | VK_RIGHT => {
                if vk_code == VK_RIGHT {
                    self.handle.move_next();
                } else {
                    self.handle.move_prev();
                }
                UiEventResult::RequiresRedraw
            }
            VK_C => {
                if let Ok(cs) = self.handle.as_data() {
                    let mut new_cs = cs.clone();
                    new_cs.name = format!("{} Copy", cs.name);

                    self.repository.add_color_scheme(new_cs.clone())
                        .and_then(|_| Ok(self.handle.select(new_cs.name)))
                        .unwrap_or_else(|e| log::error!("Failed to copy color scheme: {}", e));
                }
                UiEventResult::RequiresRedraw
            }
            VK_D | VK_DELETE => {
                if let Ok(_) = self.handle.as_data() {
                    return UiEventResult::PushState {
                        board: Box::new(yes_no_question_board(
                            format!("Delete\n\"{}\"?", self.handle.name()), self
                        )),
                        context: Box::new("Delete"),
                    }
                } else {
                    return UiEventResult::NotHandled
                }
            }
            VK_DOWN | VK_RETURN => {
                let edit_board = EditModeBoard::new(self.repository.clone(), self.handle.as_data().unwrap());
                UiEventResult::PushState {
                    board: Box::new(edit_board),
                    context: Box::new(()),
                }
            }
            VK_R | VK_F2 => {
                if let Ok(_) = self.handle.as_data() {
                    let board_box = Box::new(string_editor_board(
                        self.title(), self, "Title".to_string()
                    ));
                    UiEventResult::PushState {
                        board: board_box,
                        context: Box::new("Title"),
                    }
                } else {
                    UiEventResult::NotHandled
                }
            },
            VK_ESCAPE => {
                // Pop state without saving
                UiEventResult::PopState {
                    result: Box::new(()),
                }
            }
            _ => {
                UiEventResult::NotHandled
            }
        }
    }

    fn rename_color_scheme(&mut self, new_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(cs) = self.handle.as_data() {
            let old_name = cs.name.clone();
            let new_name = new_name.trim();

            if old_name != new_name {
                self.repository.rename_color_scheme(&old_name, new_name)?;
                self.handle.select(new_name.to_string());
            }
        }
        Ok(())
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> Board for ColorSchemeEditorBoard<R> {

    fn title(&self) -> String {
        self.handle.name().to_string()
    }

    fn name(&self) -> String {
        "ColorSchemeEditor".to_string()
    }

    fn color_scheme(&self) -> ColorScheme {
        self.handle.as_data().unwrap()
    }

    fn text_style(&self) -> TextStyle {
        self.repository.resolve_text_style(&None)
    }

    fn icon(&self) -> Option<String> {
        None
    }

    fn padset(&self, _modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        Box::new(vec![ EditModeBoard::<R>::preview_pad() ])
    }

    fn tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        vec![
            Tags::LeftRight.default(),
            Tags::EscEnter.default(),
            Tag{ text: "Colors Schemes".to_string(), anchor: Anchor::NW, ..Default::default() },
            Tag{ text: "c: copy, d: delete, f2: rename".to_string(), anchor: Anchor::SW, font_idx: Some(0), ..Default::default() },
        ]
    }
}


impl<R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for ColorSchemeEditorBoard<R> {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown ( ke) => self.key_down(ke.key, ke.modifiers),
            _ => UiEventResult::NotHandled,
        }
    }

    fn handle_child_result(&mut self, context: Box<dyn std::any::Any>, result: Box<dyn std::any::Any>) -> UiEventResult {
        if let Some(context) = context.downcast_ref::<&str>() {
            if *context == "Title" {
                return apply_string(result, |new_name| self.rename_color_scheme(new_name))
            } else if *context == "Delete" {
                return apply_bool(result, |confirm| {
                    if confirm {
                        if let Ok(cs) = self.handle.as_data() {
                            self.handle.move_next();
                            self.repository.delete_color_scheme(&cs.name)
                                .unwrap_or_else(|e| log::error!("Failed to delete color scheme: {}", e));
                        }
                    }
                    Ok(())
                });
            }
        }
        UiEventResult::NotHandled
    }
}

impl_board_component_generic!(ColorSchemeEditorBoard<R>);


#[derive(Clone, Debug, PartialEq, Eq)]
enum EditMode {
    Background,
    Opacity,
    Lines,
    Text,
    Tag,
    Palette(i32),
}

impl EnumAll<EditMode> for EditMode {
    fn all() -> Vec<EditMode> {
        vec![
            EditMode::Background,
            EditMode::Opacity,
            EditMode::Lines,
            EditMode::Text,
            EditMode::Tag,
            EditMode::Palette(0),
            EditMode::Palette(1),
            EditMode::Palette(2),
        ]
    }
}

impl EditMode {

    fn rows(&self, cs: &ColorScheme) -> Vec<TableRow> {
        let label = |mode| if *self == mode { "■■■■■■" } else { "■■■" };
        let font = |mode| if *self == mode { None } else { Some(0) };
        use EditMode::*;
        vec![
            TableRow::from_str("Background", cs.background().to_hex().as_str(), None, font(Background)),
            TableRow::from_str("Opacity", format!("{:0.2}", cs.opacity).as_str(), None, font(Opacity)),
            TableRow::from_str("Lines", label(Lines), Some(4), font(Lines)),
            TableRow::from_str("Text", label(Text), Some(5), font(Text)),
            TableRow::from_str("Tag", label(Tag), None, font(Tag)),
            TableRow::from_str("Palette 0", label(Palette(0)), Some(0), font(Palette(0))),
            TableRow::from_str("Palette 1", label(Palette(1)), Some(1), font(Palette(1))),
            TableRow::from_str("Palette 2", label(Palette(2)), Some(2), font(Palette(2))),
        ]
    }

}

struct EditModeBoard<R: SettingsRepository + SettingsRepositoryMut> {
    original_color_scheme: ColorScheme,
    color_scheme: ColorScheme,
    repository: Rc<R>,
    mode: EditMode,
    inactive_menu: bool,
    line_spacing: RefCell<Option<i32>>,
}

impl<R: SettingsRepository + SettingsRepositoryMut> Clone for EditModeBoard<R> {
    fn clone(&self) -> Self {
        Self {
            original_color_scheme: self.original_color_scheme.clone(),
            color_scheme: self.color_scheme.clone(),
            repository: self.repository.clone(),
            mode: self.mode.clone(),
            inactive_menu: self.inactive_menu,
            line_spacing: RefCell::new(*self.line_spacing.borrow()),
        }
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> EditModeBoard<R> {
    pub fn new(repository: Rc<R>, color_scheme: ColorScheme) -> Self {
        Self {
            original_color_scheme: color_scheme.clone(),
            color_scheme,
            repository,
            mode: EditMode::Background,
            inactive_menu: false,
            line_spacing: RefCell::new(None),
        }
    }

    fn with_inactive_menu(mut self, inactive: bool) -> Self {
        self.inactive_menu = inactive;
        self
    }

    fn is_dirty(&self) -> bool {
        self.original_color_scheme != self.color_scheme
    }

    fn get_line_spacing(&self) -> i32 {
        if let Some(spacing) = *self.line_spacing.borrow() {
            return spacing;
        }

        pub fn calculate_line_spacing(hdc: HDC, font: windows::Win32::Graphics::Gdi::HFONT) -> i32 {
            let to_wstr = |str: &str| str.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
            unsafe {
                let previous_font = SelectObject(hdc, font.into());
                let mut text_size = RECT::default();
                DrawTextW(hdc, to_wstr("Ay").as_mut_slice(), &mut text_size, DT_CALCRECT | DT_NOPREFIX);
                SelectObject(hdc, previous_font);
                text_size.bottom - text_size.top
            }
        }

        let hdc = unsafe { windows::Win32::Graphics::Gdi::CreateCompatibleDC(Some(HDC(std::ptr::null_mut()))) };
        let font = self.text_style().tag_font();
        let spacing = calculate_line_spacing(hdc, font);
        *self.line_spacing.borrow_mut() = Some(spacing);
        spacing
    }


    fn get_menu_pad(&self, inactive: bool) -> Pad {
        let index = Some(self.mode.index());
        let rows = self.mode.rows(&self.color_scheme);
        let spacing = self.get_line_spacing();

        let mut cs = self.color_scheme.clone();
        cs.palette.push(self.color_scheme.background().to_hex()); // idx 3
        cs.palette.push(self.color_scheme.foreground1().to_hex()); // idx 4
        cs.palette.push(self.color_scheme.foreground2().to_hex()); // idx 5

        PadId::Seven.with_data(core::Pad {
            ..Default::default()
        }).with_tags(
            TableView { rows, index }
                .render(spacing, 2, spacing + spacing / 2, spacing/2, inactive)
                .concat()
        )
        .with_color_scheme(cs)

    }

    fn preview_pad() -> Pad {
        PadId::Five.with_data(core::Pad {
            text: Some("Text color".to_string()),
            ..Default::default()
        }).with_tags(vec![
            Tag { text: "Tag color".to_string(), anchor: Anchor::NW, font_idx: None, color_idx: None, ..Default::default() },
            Tag { text: "Palette 0".to_string(), anchor: Anchor::W, font_idx: Some(0), color_idx: Some(0), ..Default::default() },
            Tag { text: "Palette 1".to_string(), anchor: Anchor::WSW, font_idx: Some(0), color_idx: Some(1), ..Default::default() },
            Tag { text: "Palette 2".to_string(), anchor: Anchor::SW, font_idx: Some(0), color_idx: Some(2), ..Default::default() },
        ])
    }

}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> Board for EditModeBoard<R> {

    fn title(&self) -> String {
        self.color_scheme.name.clone()
    }

    fn name(&self) -> String {
        "ColorSchemeEditorEditMode".to_string()
    }

    fn color_scheme(&self) -> ColorScheme {
        let mut cs = self.color_scheme.clone();
        if self.inactive_menu {
            return cs;
        }

        let tag_color = match &self.mode {
            EditMode::Opacity => self.color_scheme.foreground2(),
            EditMode::Background => self.color_scheme.background(),
            EditMode::Lines => self.color_scheme.foreground1(),
            EditMode::Text => self.color_scheme.foreground2(),
            EditMode::Tag => self.color_scheme.tag_foreground(),
            EditMode::Palette(idx) => self.color_scheme.palette_color(*idx as usize).unwrap_or(self.color_scheme.foreground2()),
        };
        cs.palette.push(tag_color.to_hex());
        cs
    }

    fn text_style(&self) -> TextStyle {
        self.repository.resolve_text_style(&None)
    }

    fn icon(&self) -> Option<String> {
        None
    }

    fn padset(&self, _modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        let mut pads = vec![
            self.get_menu_pad(self.inactive_menu),
            EditModeBoard::<R>::preview_pad()
        ];
        match self.mode {
            EditMode::Opacity => {
                pads.push(PadId::Eight.with_data(core::Pad {
                    text: Some(format!("{:0.2}", self.color_scheme.opacity)),
                    ..Default::default()
                }));
            },
            EditMode::Background | EditMode::Text | EditMode::Lines | EditMode::Tag | EditMode::Palette(_) => {
                let system_color = match &self.mode {
                    EditMode::Background => SystemColor::Background,
                    EditMode::Text => SystemColor::Text,
                    EditMode::Lines => SystemColor::Lines,
                    EditMode::Tag => SystemColor::Tag,
                    EditMode::Palette(i) if *i == 0 => SystemColor::PalleteR,
                    EditMode::Palette(i) if *i == 1 => SystemColor::PalleteG,
                    EditMode::Palette(i) if *i == 2 => SystemColor::PalleteB,
                    _ => unreachable!(),
                };
                let color_editor = ColorEditor::new(
                    Box::new(self.clone().with_inactive_menu(true)),
                        system_color.to_color(&self.color_scheme),
                    system_color
                );
                pads.push(color_editor.get_pad(PadId::Eight, false));
            }
        }
        Box::new(pads)
    }



    fn tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        if self.inactive_menu || !self.is_dirty() {
            vec![
                Tags::DownUp.default(),
                Tags::EscEnter.default(),
                Tag{ text: "Colors Schemes".to_string(), anchor: Anchor::NW, ..Default::default() },
            ]
        } else {
            vec![
                Tags::DownUp.default(),
                Tags::EscEnter.default(),
                Tag{ text: "Colors Schemes".to_string(), anchor: Anchor::NW, ..Default::default() },
                Tag{ text: "s: save".to_string(), anchor: Anchor::SW, font_idx: Some(0), ..Default::default() },
            ]

        }
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for EditModeBoard<R> {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown ( ke) => {
                let vk_code = VIRTUAL_KEY(ke.key as u16);
                match vk_code {
                    VK_UP | VK_DOWN => {
                        if vk_code == VK_DOWN {
                            self.mode = self.mode.next();
                        } else {
                            self.mode = self.mode.previous();
                        }
                        UiEventResult::RequiresRedraw
                    }
                    VK_S => {
                        self.repository.set_color_scheme(self.color_scheme.clone())
                            .unwrap_or_else(|e| log::error!("Failed to save color scheme: {}", e));
                        // self.repository.flush().unwrap_or_else(|e| log::error!("Failed to flush settings: {}", e));
                        self.original_color_scheme = self.color_scheme.clone();
                        UiEventResult::RequiresRedraw
                    }
                    VK_RETURN => {
                        match &self.mode {
                            EditMode::Opacity => {
                                let edit_board = OpacityEditor::new(
                                    Box::new(self.clone().with_inactive_menu(true)), self.repository.clone());
                                UiEventResult::PushState {
                                    board: Box::new(edit_board),
                                    context: Box::new(EditMode::Opacity),
                                }
                            },
                            EditMode::Background | EditMode::Text | EditMode::Lines | EditMode::Tag | EditMode::Palette(_) => {
                                let system_color = match &self.mode {
                                    EditMode::Background => SystemColor::Background,
                                    EditMode::Text => SystemColor::Text,
                                    EditMode::Lines => SystemColor::Lines,
                                    EditMode::Tag => SystemColor::Tag,
                                    EditMode::Palette(i) if *i == 0 => SystemColor::PalleteR,
                                    EditMode::Palette(i) if *i == 1 => SystemColor::PalleteG,
                                    EditMode::Palette(i) if *i == 2 => SystemColor::PalleteB,
                                    _ => unreachable!(),
                                };
                                let color_editor = ColorEditor::new(
                                    Box::new(self.clone().with_inactive_menu(true)),
                                     system_color.to_color(&self.color_scheme),
                                    system_color
                                );
                                UiEventResult::PushState {
                                    board: Box::new(color_editor),
                                    context: Box::new(EditMode::Background),
                                }
                            }
                        }
                    }
                    VK_ESCAPE => {
                        // Pop state without saving
                        UiEventResult::PopState {
                            result: Box::new(()),
                        }
                    }
                    _ => {
                        UiEventResult::NotHandled
                    }
                }
            },
            _ => UiEventResult::NotHandled,
        }
    }

    fn handle_child_result(&mut self, _context: Box<dyn std::any::Any>, result: Box<dyn std::any::Any>) -> UiEventResult {
        if let Some(new_cs) = result.downcast_ref::<ColorScheme>() {
            self.color_scheme = new_cs.clone();
            return UiEventResult::RequiresRedraw
        }
        UiEventResult::RequiresRedraw
    }
}

impl_board_component_generic!(EditModeBoard<R>);



struct OpacityEditor<R: SettingsRepository + SettingsRepositoryMut> {
    inner: Box<dyn Board>,
    spinner: NumericSpinnerPad<f64>,
    #[allow(dead_code)]
    repository: Rc<R>,
}

impl<R: SettingsRepository + SettingsRepositoryMut> OpacityEditor<R> {
    pub fn new(inner: Box<dyn Board>, repository: Rc<R>) -> Self {
        let format = |v: f64| format!("{:0.2}", v);
        let initial = inner.color_scheme().opacity;
        Self {
            inner,
            spinner: NumericSpinnerPad::new(PadId::Eight, initial, 0.0, 1.0, 0.01, Some(format)),
            repository,
        }
    }
}

impl_has_board!(OpacityEditor<R>);

impl<R: SettingsRepository + SettingsRepositoryMut> DelegatingBoard for OpacityEditor<R> {
    fn delegate_color_scheme(&self) -> ColorScheme {
        let mut cs = self.inner.color_scheme();
        cs.opacity = self.spinner.parsed_formatted_value().unwrap_or(self.spinner.value());
        cs
    }

    fn delegate_padset(&self, modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        Box::new(self.inner.padset(modifier).overlay(vec![self.spinner.get_pad()]))
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut> UiEventHandler for OpacityEditor<R> {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(ke) => {
                let vk_code = VIRTUAL_KEY(ke.key as u16);
                match vk_code {
                    VK_UP | VK_DOWN => self.spinner.key_down(ke),
                    VK_RETURN => UiEventResult::PopState { result: Box::new(self.delegate_color_scheme()) },
                    VK_ESCAPE => UiEventResult::PopState { result: Box::new(()) },
                    _ => UiEventResult::NotHandled,
                }
            },
            UiEvent::KeyUp(ke) => self.spinner.key_up(ke),
            _ => UiEventResult::NotHandled,
        }
    }
}

impl_board_component_generic!(OpacityEditor<R>);

enum SystemColor {
    Background,
    Text,
    Lines,
    Tag,
    PalleteR,
    PalleteG,
    PalleteB,
}

impl SystemColor {
    fn to_color(&self, cs: &ColorScheme) -> Color {
        match self {
            SystemColor::Background => cs.background(),
            SystemColor::Text => cs.foreground2(),
            SystemColor::Lines => cs.foreground1(),
            SystemColor::Tag => cs.tag_foreground(),
            SystemColor::PalleteR => cs.palette_color(0).unwrap_or(cs.foreground2()),
            SystemColor::PalleteG => cs.palette_color(1).unwrap_or(cs.foreground2()),
            SystemColor::PalleteB => cs.palette_color(2).unwrap_or(cs.foreground2()),
        }
    }

    fn set_color(&self, cs: &mut ColorScheme, color: &Color) {
        match self {
            SystemColor::Background => cs.background = color.to_hex(),
            SystemColor::Text => cs.foreground2 = color.to_hex(),
            SystemColor::Lines => cs.foreground1 = color.to_hex(),
            SystemColor::Tag => cs.tag_foreground = color.to_hex(),
            SystemColor::PalleteR => { if cs.palette.len() > 0 { cs.palette[0] = color.to_hex(); } },
            SystemColor::PalleteG => { if cs.palette.len() > 1 { cs.palette[1] = color.to_hex(); } },
            SystemColor::PalleteB => { if cs.palette.len() > 2 { cs.palette[2] = color.to_hex(); } },
        }
    }
}

enum ColorComponent {
    R,
    G,
    B,
}

struct ColorEditor {
    inner: Box<dyn Board>,
    color: Color,
    system_color: SystemColor,
    cur_component: ColorComponent
}

impl ColorEditor {
    pub fn new(inner: Box<dyn Board>, color: Color, system_color: SystemColor) -> Self {
        Self {
            inner,
            color,
            system_color,
            cur_component: ColorComponent::R
        }
    }

    fn get_sliders(&self) -> (HSlider<i32>, HSlider<i32>, HSlider<i32>) {
        let (r, g, b) = self.color.to_rgb();
        let r_slider = HSlider::new("R".to_string(), r as i32, 0, 255, 1, Some(|v| format!("{:<3 }", v)));
        let g_slider = HSlider::new("G".to_string(), g as i32, 0, 255, 1, Some(|v| format!("{:<3 }", v)));
        let b_slider = HSlider::new("B".to_string(), b as i32, 0, 255, 1, Some(|v| format!("{:<3 }", v)));
        (r_slider, g_slider, b_slider)
    }

    fn set_sliders(&mut self, r: HSlider<i32>, g: HSlider<i32>, b: HSlider<i32>) {
        self.color = Color { r: r.value() as u8, g: g.value() as u8, b: b.value() as u8 };
    }

    fn get_tags(&self, add_current_marker: bool) -> Vec<Tag> {
        let (r, g, b) = self.get_sliders();
        let mut tags = vec![
            r.get_tag(Anchor::CN),
            g.get_tag(Anchor::C),
            b.get_tag(Anchor::CS),
        ];

        if add_current_marker {
            let (anchor_l, anchor_r) = match self.cur_component {
                ColorComponent::R => (Anchor::WNW, Anchor::ENE),
                ColorComponent::G => (Anchor::W, Anchor::E),
                ColorComponent::B => (Anchor::WSW, Anchor::ESE),
            };
            tags.push(Tags::RightBlack.tag(anchor_l));
            tags.push(Tags::LeftBlack.tag(anchor_r));

            tags.push(Tag { text: "e: edit".to_string(), anchor: Anchor::SW, font_idx: Some(0), ..Default::default() });
        }

        tags
    }

    fn get_pad(&self, pad_id: PadId, add_current_marker: bool) -> Pad {
        pad_id.with_data(core::Pad {
            header: Some(self.color.to_hex()),
            ..Default::default()
        }).with_tags(self.get_tags(add_current_marker))
    }
}

impl HasBoard for ColorEditor {
    fn board(&self) -> &dyn Board {
        self.inner.as_ref()
    }
}

impl DelegatingBoard for ColorEditor {
    fn delegate_color_scheme(&self) -> ColorScheme {
        let mut cs = self.inner.color_scheme();
        self.system_color.set_color(&mut cs, &self.color);
        cs
    }

    fn delegate_padset(&self, modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        let pad = self.get_pad(PadId::Eight, true);

        Box::new(
            self.inner.padset(modifier).overlay(vec![pad])
        )
    }
}

impl UiEventHandler for ColorEditor {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(ke) => {
                let vk_code = VIRTUAL_KEY(ke.key as u16);
                match vk_code {
                    VK_UP | VK_DOWN => {
                        self.cur_component = match self.cur_component {
                            ColorComponent::R => if vk_code == VK_DOWN { ColorComponent::G } else { ColorComponent::B },
                            ColorComponent::G => if vk_code == VK_DOWN { ColorComponent::B } else { ColorComponent::R },
                            ColorComponent::B => if vk_code == VK_DOWN { ColorComponent::R } else { ColorComponent::G },
                        };
                        UiEventResult::RequiresRedraw
                    }
                    VK_LEFT | VK_RIGHT => {
                        let (mut r, mut g, mut b) = self.get_sliders();
                        let slider = match self.cur_component {
                            ColorComponent::R => &mut r,
                            ColorComponent::G => &mut g,
                            ColorComponent::B => &mut b,
                        };
                        slider.key_down(ke);
                        self.set_sliders(r, g, b);

                        UiEventResult::RequiresRedraw
                    }
                    VK_RETURN => UiEventResult::PopState { result: Box::new(self.delegate_color_scheme()) },
                    VK_ESCAPE => UiEventResult::PopState { result: Box::new(()) },
                    VK_E => UiEventResult::RequestChildWindow(ChildWindowRequest::ColorEditor),
                    _ => UiEventResult::NotHandled,
                }
            },
            _ => UiEventResult::NotHandled,
        }
    }

    fn create_child_window(&mut self, request: ChildWindowRequest, parent_hwnd: windows::Win32::Foundation::HWND) -> UiEventResult {
        match request {
            ChildWindowRequest::ColorEditor => {
                if let Some(selected_color) = open_color_picker(self.color.clone(), Some(parent_hwnd)) {
                    self.color = selected_color;
                }
                UiEventResult::RequiresRedraw
            }
            _ => UiEventResult::NotHandled,
        }
    }
}

impl_board_component!(ColorEditor);


struct TableRow {
    col1: Tag,
    col2: Tag,
}

impl TableRow {
    pub fn from_str(col1: &str, col2: &str, col2_color_idx: Option<usize>, col2_font_idx: Option<usize>) -> Self {
        Self {
            col1: Tag {
                text: col1.to_string(),
                pin: Some(AnchorPin::NW),
                anchor: Anchor::NW,
                color_idx: None,
                font_idx: None,
            },
            col2: Tag {
                text: col2.to_string(),
                pin: Some(AnchorPin::NE),
                anchor: Anchor::NE,
                color_idx: col2_color_idx,
                font_idx: col2_font_idx,
            },
        }
    }
}

struct TableView {
    rows: Vec<TableRow>,
    index: Option<usize>,
}

impl TableView {

    pub fn render(&self, line_spacing: i32, padding_left: i32, padding_right: i32, padding_top: i32, inactive: bool) -> Vec<Vec<Tag>> {
        let mut tags = Vec::new();


        for (i, row) in self.rows.iter().enumerate() {
            let y = padding_top + i as i32 * line_spacing;

            // 3 columns: col1, col2, selection indicator
            let mut col1 = row.col1.clone();
            let mut col2 = row.col2.clone();

            col1.anchor = Anchor::Abs2(padding_left + line_spacing, y);
            col1.pin = Some(AnchorPin::NW);
            col2.anchor = Anchor::Abs2(-padding_right, y);
            col2.pin = Some(AnchorPin::NE);

            if self.index == Some(i) {
                let indicator = Tag {
                    text: "▶".to_string(),
                    pin: Some(AnchorPin::NW),
                    anchor: Anchor::Abs2(padding_left, y),
                    color_idx: if inactive { None } else { Some(0) },
                    font_idx: None,
                };
                tags.push(vec![col1, col2, indicator]);
            } else {
                tags.push(vec![col1, col2]);
            }
        }

        tags
    }


}



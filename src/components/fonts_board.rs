use std::{rc::Rc};

use windows::Win32::{Foundation::HWND, UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_C, VK_D, VK_DELETE, VK_DOWN, VK_ESCAPE, VK_F2, VK_LEFT, VK_R, VK_RETURN, VK_RIGHT, VK_S, VK_UP}};

use super::{ apply_string, BoardComponent, ChildWindowRequest, StringEditorBoard, UiEvent, UiEventHandler, UiEventResult, controls::Tags };

use crate::{
    core::{self, SettingsRepository, SettingsRepositoryMut}, impl_board_component_generic, input::{ModifierState, TextCapture}, model::{Anchor, Board, ColorScheme, Pad, PadId, PadSet, Tag, TextStyle, TextStyleHandle}, ui::dialogs::open_font_editor
};


pub struct TextStyleEditorBoard<R: SettingsRepository + SettingsRepositoryMut> {
    handle: TextStyleHandle<R>,
    repository: Rc<R>,
    color_scheme: ColorScheme,
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> TextStyleEditorBoard<R> {
    pub fn new(repository: Rc<R>, name: Option<String>, color_scheme: ColorScheme) -> Self {
        Self {
            handle: TextStyleHandle::new(repository.clone(), name),
            repository,
            color_scheme,
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
                    let mut new_ts = cs.clone();
                    new_ts.name = format!("{} Copy", cs.name);

                    self.repository.add_text_style(new_ts.clone())
                        .and_then(|_| Ok(self.handle.select(new_ts.name)))
                        .unwrap_or_else(|e| log::error!("Failed to copy text style: {}", e));
                }
                UiEventResult::RequiresRedraw
            }
            VK_D | VK_DELETE => {
                if let Ok(ts) = self.handle.as_data() {
                    self.handle.move_next();
                    self.repository.delete_text_style(&ts.name)
                        .unwrap_or_else(|e| log::error!("Failed to delete text style: {}", e));
                }
                UiEventResult::RequiresRedraw
            }
            VK_DOWN | VK_RETURN => {
                UiEventResult::PushState {
                    board: Box::new(EditModeBoard::new(self.repository.clone(), self.handle.as_data().unwrap(), self.color_scheme())),
                    context: Box::new(()),
                }
            }
            VK_R | VK_F2 => {
                match self.handle.as_data().ok() {
                    Some(_) => {
                        UiEventResult::PushState {
                            board: Box::new(StringEditorBoard {
                                text_capture: TextCapture::new(self.title().into(), false),
                                text_style: Some(self.text_style()),
                                color_scheme: Some(self.color_scheme()),
                                tags: vec![
                                    Tag { text: "Title".to_string(), anchor: Anchor::NW, color_idx: Some(0), ..Default::default() },
                                    Tags::EscEnter.default(),
                                ],
                            }),
                            context: Box::new("Title"),
                        }
                    },
                    None => UiEventResult::NotHandled
                }
            },
            VK_ESCAPE => {
                UiEventResult::PopState {
                    result: Box::new(()),
                }
            }
            _ => UiEventResult::NotHandled
        }
    }


    fn rename_text_style(&mut self, new_name: &str) -> Result<(), Box<dyn std::error::Error>> {
        if let Ok(ts) = self.handle.as_data() {
            let old_name = ts.name.clone();
            let new_name = new_name.trim();

            if old_name != new_name {
                self.repository.rename_text_style(&old_name, new_name)?;
                self.handle.select(new_name.to_string());
            }
        }
        Ok(())
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut> Board for TextStyleEditorBoard<R> {

    fn title(&self) -> String {
        self.handle.name().to_string()
    }

    fn name(&self) -> String {
        "TextStyleEditor".to_string()
    }

    fn color_scheme(&self) -> ColorScheme {
        self.color_scheme.clone()
    }

    fn text_style(&self) -> TextStyle {
        self.handle.as_data().unwrap()
    }

    fn icon(&self) -> Option<String> {
        None
    }

    fn padset(&self, _modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        Box::new(vec![EditModeBoard::<R>::preview_pad()])
    }

    fn tags(&self) -> Vec<Tag> {
        vec![
            Tags::LeftRight.default(),
            Tags::EscEnter.default(),
            Tag{ text: "Text Styles".to_string(), anchor: Anchor::NW, ..Default::default() },
            Tag{ text: "c: copy, d: delete, f2: rename".to_string(), anchor: Anchor::SW, font_idx: Some(0), ..Default::default() },
        ]
    }



}


impl<R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for TextStyleEditorBoard<R> {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown ( ke) => self.key_down(ke.key, ke.modifiers),
            _ => UiEventResult::NotHandled,
        }
    }

    fn handle_child_result(&mut self, context: Box<dyn std::any::Any>, result: Box<dyn std::any::Any>) -> UiEventResult {
        if let Some(title) = context.downcast_ref::<&str>() {
            if *title == "Title" {
                return apply_string(result, |new_name| self.rename_text_style(new_name))
            }
        }
        UiEventResult::NotHandled
    }
}

impl_board_component_generic!(TextStyleEditorBoard<R>);

#[derive(Clone, Debug, PartialEq)]
enum EditMode {
    Header,
    PadHeader,
    PadText,
    PadId,
    Tag,
    Palette(i32)
}

impl EditMode {
    fn next(&self) -> EditMode {
        match self {
            EditMode::Header => EditMode::PadHeader,
            EditMode::PadHeader => EditMode::PadText,
            EditMode::PadText => EditMode::Tag,
            EditMode::Tag => EditMode::PadId,
            EditMode::PadId => EditMode::Palette(0),
            EditMode::Palette(i) if *i < 2 => EditMode::Palette(i + 1),
            EditMode::Palette(_) => EditMode::Header,
        }
    }

    fn prev(&self) -> EditMode {
        match self {
            EditMode::Header => EditMode::Palette(2),
            EditMode::PadHeader => EditMode::Header,
            EditMode::PadText => EditMode::PadHeader,
            EditMode::Tag => EditMode::PadText,
            EditMode::PadId => EditMode::Tag,
            EditMode::Palette(i) if *i > 0 => EditMode::Palette(i - 1),
            EditMode::Palette(_) => EditMode::PadId,
        }
    }

    fn get_font(&self, text_style: &TextStyle) -> String {
        match self {
            EditMode::Header => text_style.header_font.clone(),
            EditMode::PadHeader => text_style.pad_header_font.clone(),
            EditMode::PadText => text_style.pad_text_font.clone(),
            EditMode::PadId => text_style.pad_id_font.clone(),
            EditMode::Tag => text_style.tag_font.clone(),
            EditMode::Palette(i) if *i >= 0 && *i < 3 => text_style.palette.get(*i as usize).cloned().unwrap_or_default(),
            EditMode::Palette(_) => "".to_string(),
        }
    }

    fn header_tags(&self) -> Vec<Tag> {
        match self {
            EditMode::Header => vec![
                Tags::DownBlack.default(),
                Tags::UpBlack.default(),
            ],
            _ => vec![],
        }
    }

    fn pad_tags(&self) -> Vec<(PadId, Tag)> {
        match &self {
            EditMode::PadHeader => vec![
                (PadId::Five, Tags::RightBlack.tag(Anchor::NNW)),
                (PadId::Five, Tags::LeftBlack.tag(Anchor::NNE))
            ],
            EditMode::PadText => vec![
                (PadId::Five, Tags::RightBlack.tag(Anchor::Rel(0.3, 0.5))),
                (PadId::Five, Tags::LeftBlack.tag(Anchor::Rel(0.7, 0.5))),
            ],
            EditMode::PadId => vec![
                (PadId::Six, Tags::LeftBlack.tag(Anchor::SW)),
            ],
            EditMode::Palette(i) => {
                let anchor = match i {
                    0 => Anchor::ENE,
                    1 => Anchor::E,
                    2 => Anchor::ESE,
                    _ => Anchor::C,
                };
                vec![(PadId::Four, Tags::RightBlack.tag(anchor))]
            }
            EditMode::Tag => vec![
                (PadId::Six, Tags::LeftBlack.tag(Anchor::W)),
            ],
            _ => vec![],
        }
    }

    fn set_font(&self, text_style: &mut TextStyle, font: String) {
        match self {
            EditMode::Header => text_style.header_font = font,
            EditMode::PadHeader => text_style.pad_header_font = font,
            EditMode::PadText => text_style.pad_text_font = font,
            EditMode::PadId => text_style.pad_id_font = font,
            EditMode::Tag => text_style.tag_font = font,
            EditMode::Palette(i) if *i >= 0 && *i < 3 => {
                if text_style.palette.len() <= *i as usize {
                    text_style.palette.resize(*i as usize + 1, "".to_string());
                }
                text_style.palette[*i as usize] = font;
            }
            EditMode::Palette(_) => { }
        }
    }

    fn select_font(&self, text_style: &mut TextStyle, parent: Option<HWND>) -> UiEventResult {
        let initial_font = self.get_font(text_style);
        if let Some(font) = open_font_editor(&initial_font, parent) {
            self.set_font(text_style, font);
            UiEventResult::RequiresRedraw
        } else {
            UiEventResult::NotHandled
        }
    }
}


struct EditModeBoard<R: SettingsRepository + SettingsRepositoryMut> {
    original_text_style: TextStyle,
    text_style: TextStyle,
    color_scheme: ColorScheme,
    repository: Rc<R>,
    mode: EditMode,
}

impl<R: SettingsRepository + SettingsRepositoryMut> EditModeBoard<R> {
    pub fn new(repository: Rc<R>, text_style: TextStyle, color_scheme: ColorScheme) -> Self {
        Self {
            original_text_style: text_style.clone(),
            text_style,
            repository,
            color_scheme,
            mode: EditMode::PadText
        }
    }

    fn is_dirty(&self) -> bool {
        self.original_text_style != self.text_style
    }

    fn preview_pad() -> Pad {
        PadId::Five.with_data(core::Pad {
            header: Some("Pad header".to_string()),
            text: Some("Pad text".to_string()),
            ..Default::default()
        }).with_tags(vec![
            Tag { text: "Tag".to_string(), anchor: Anchor::E, ..Default::default() },
            Tag { text: "P0: enter".to_string(), anchor: Anchor::WNW, font_idx: Some(0), ..Default::default() },
            Tag { text: "P1: enter".to_string(), anchor: Anchor::W, font_idx: Some(1), ..Default::default() },
            Tag { text: "P2: ◁▷".to_string(), anchor: Anchor::WSW, font_idx: Some(2), ..Default::default() },
            // Tag { text: "preview".to_string(), anchor: Anchor::S, color_idx: Some(0), ..Default::default() },
        ])
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> Board for EditModeBoard<R> {

    fn title(&self) -> String {
        self.text_style.name.clone()
    }

    fn name(&self) -> String {
        "EditTextStyle".to_string()
    }

    fn color_scheme(&self) -> ColorScheme {
        self.color_scheme.clone()
    }

    fn text_style(&self) -> TextStyle {
        self.text_style.clone()
    }

    fn icon(&self) -> Option<String> {
        None
    }

    fn padset(&self, _modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        let mut padset = vec![Self::preview_pad()];

        for (pad_id, tag) in self.mode.pad_tags() {
            let mut pad = padset.pad(pad_id);
            pad.tags.push(tag);
            padset.update(pad);
        }
        Box::new(padset)
    }

    fn tags(&self) -> Vec<Tag> {
        self.mode.header_tags().into_iter()
        .chain(
            vec![
                Tags::DownUp.default(),
                Tags::EscEnter.default(),
                Tag{ text: "Text Styles".to_string(), anchor: Anchor::NW, ..Default::default() },
            ]
        ).
        chain(
            if self.is_dirty() {
                vec![Tag{ text: "s: save".to_string(), anchor: Anchor::SW, font_idx: Some(0), ..Default::default() }]
            } else {
                vec![]
            }
        )
        .collect()
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
                            self.mode = self.mode.prev();
                        }
                        UiEventResult::RequiresRedraw
                    },
                    VK_RETURN => {
                        UiEventResult::RequestChildWindow(ChildWindowRequest::FontSelector)
                    }
                    VK_S => {
                        self.repository.set_text_style(self.text_style.clone())
                            .unwrap_or_else(|e| log::error!("Failed to save text style: {}", e));
                        // self.repository.flush().unwrap_or_else(|e| log::error!("Failed to flush settings: {}", e));
                        self.original_text_style = self.text_style.clone();
                        UiEventResult::RequiresRedraw
                    }
                    VK_ESCAPE => {
                        // Pop state without saving
                        UiEventResult::PopState {
                            result: Box::new(()),
                        }
                    }
                    _ => UiEventResult::NotHandled
                }
            },
            _ => UiEventResult::NotHandled,
        }
    }


    fn create_child_window(&mut self, request: ChildWindowRequest, parent_hwnd: HWND) -> UiEventResult {
        match request {
            ChildWindowRequest::FontSelector => {
                self.mode.select_font(&mut self.text_style, Some(parent_hwnd))
            }
            _ => UiEventResult::NotHandled
        }
    }
}

impl_board_component_generic!(EditModeBoard<R>);
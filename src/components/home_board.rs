use std::rc::Rc;

use crate::core::{self, ActionType, DetectedIcon, Param, Resources, SettingsRepository, SettingsRepositoryMut };
use crate::{impl_board_component_generic, impl_has_board};
use crate::model::{Anchor, AnchorPin, Board, ColorScheme, CreateDetectableBoardUseCase, ModifierState, Pad, PadId, PadSet, Tag, TextStyle};

use super::{ BoardComponent, UiEventHandler, DelegatingBoard, HasBoard, UiEvent, UiEventResult, controls::Tags, INITIAL_PATH_PARAM };


use windows::Win32::UI::Input::KeyboardAndMouse::*;

pub struct HomeBoard<R: SettingsRepository + SettingsRepositoryMut> {
    board: core::Board,
    params: Vec<Param>,
    resources: Resources,
    repository: Rc<R>
}

impl<R: SettingsRepository + SettingsRepositoryMut> Clone for HomeBoard<R> {
    fn clone(&self) -> Self {
        Self {
            board: self.board.clone(),
            params: self.params.clone(),
            resources: self.resources.clone(),
            repository: self.repository.clone()
        }
    }
}

impl <R: SettingsRepository + SettingsRepositoryMut> HomeBoard<R> {
    pub fn new(board: core::Board, params: Vec<Param>, resources: Resources, repository: Rc<R>) -> Self {
        Self {
            board,
            params,
            resources,
            repository
        }
    }


    fn get_inverted_styling(&self) -> (TextStyle, ColorScheme) {
        let comment_text_style = self.text_style();
        // let (face, _, _, size) = TextStyle::parse_font(comment_text_style.pad_header_font.as_str());
        // comment_text_style.pad_text_font = format!("{} Bold Italic {}", face, size-2);

        let mut color_scheme = self.color_scheme().inverted();
        color_scheme.opacity = 1.0;
        (comment_text_style, color_scheme)
    }

    fn get_modifier_padset(&self, modifier: ModifierState) -> Box<dyn PadSet> {
        let (comment_text_style, color_scheme) = self.get_inverted_styling();

        let modifier = modifier.to_string();
        let pads = vec![
            PadId::Two.with_data(core::Pad {
                header: Some("\nTo add new board:".to_string()),
                // text: Some("Specialized pad sets for\ndifferent modifier combinations.".to_string()),
                ..Default::default()
            })
            .with_color_scheme(color_scheme.clone())
            .with_text_style(comment_text_style.clone())
            .with_tags(vec![
                Tag {
                    text: "1. Minimize \"Hotkeys\" to tray\n2. Focus the target app\n3. Press CTRL+ALT+NumPad 0".to_string(),
                    anchor: Anchor::Rel(0.05, 0.35),
                    pin: Some(AnchorPin::NW),
                    font_idx: None,
                    ..Default::default()
                }
            ]),


            PadId::Five.with_data(core::Pad {
                text: Some(format!("😊 {} 😊", modifier)),
                ..Default::default()
            })
        ];
        Box::new(pads)
    }

    fn get_base_padset(&self) -> Box<dyn PadSet> {
        let pads: Vec<Pad> = vec![
            PadId::Two.with_data(core::Pad {
                header: Some("Press a NumPad key,\na modifier key or Esc".to_string()),
                ..Default::default()
            }),

            PadId::Four.with_data(core::Pad {
                text: Some("Settings".to_string()),
                board: Some("settings".to_string()),
                ..Default::default()
            }),

            PadId::Six.with_data(core::Pad {
                text: Some("Documentation".to_string()),
                actions: vec![ ActionType::OpenUrl("https://github.com/ivicakukic/hotkeys-win/blob/main/README.md".to_string()) ],
                ..Default::default()
            }),

            PadId::Five.with_data(core::Pad {
                text: Some("About".to_string()),
                actions: vec![ ActionType::OpenUrl("https://github.com/ivicakukic/hotkeys-win".to_string()) ],
                ..Default::default()
            }),

        ];
        Box::new(pads)
    }

    fn get_process_name(&self) -> Option<String> {
        self.params.iter()
            .find(|p| p.name == "process_name")
            .map(|p| p.value.clone())
    }

    fn get_window_title(&self) -> String {
        self.params.iter()
            .find(|p| p.name == "window_title")
            .map(|p| p.value.clone())
            .or(self.get_process_name())
            .unwrap_or_else(|| "New board".to_string())
    }
}

impl <R: SettingsRepository + SettingsRepositoryMut> Board for HomeBoard<R> {
    fn name(&self) -> String {
        self.board.name.clone()
    }

    fn title(&self) -> String {
        "HotKeys".to_string()
    }

    fn icon(&self) -> Option<String> {
        Some("icon.png".to_string())
    }

    fn color_scheme(&self) -> ColorScheme {
        self.repository.resolve_color_scheme(&self.board.color_scheme)
    }

    fn text_style(&self) -> TextStyle {
        self.repository.resolve_text_style(&self.board.text_style)
    }

    fn padset(&self, modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        if let Some(modifier) = modifier {
            if !modifier.is_none() && modifier != (ModifierState { alt: true, ..Default::default() }) {
                return self.get_modifier_padset(modifier);
            }
        }
        self.get_base_padset()
    }

    fn tags(&self) -> Vec<Tag> {
        vec![]
    }

}

impl <R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for HomeBoard<R> {

    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(key_event) => {
                let vk_code = VIRTUAL_KEY(key_event.key as u16);
                match vk_code {
                    VK_S => UiEventResult::PadSelected(PadId::Four),
                    VK_A => UiEventResult::PadSelected(PadId::Five),
                    VK_D => UiEventResult::PadSelected(PadId::Six),
                    _ => UiEventResult::NotHandled,
                }
            }
            _ => UiEventResult::NotHandled,
        }
    }

    fn activate(&mut self) -> UiEventResult {
        match self.get_process_name() {
            Some(process_name) => {

                let board = CreateProcessBoard {
                    inner: Box::new(self.clone()),
                    process_name: process_name.clone(),
                    window_title: self.get_window_title(),
                    icon: self.resources.detected_icon(process_name).as_option(),
                    repository: self.repository.clone()
                 };


                UiEventResult::PushState {
                    board: Box::new(board),
                    context: Box::new(()),
                }
            },
            None => UiEventResult::NotHandled,
        }
    }
}

impl_board_component_generic!(HomeBoard<R>);


struct CreateProcessBoard<R: SettingsRepository + SettingsRepositoryMut> {
    inner: Box<dyn Board>,
    process_name: String,
    window_title: String,
    icon: Option<DetectedIcon>,
    repository: Rc<R>,
}

impl<R: SettingsRepository + SettingsRepositoryMut> CreateProcessBoard<R> {

    fn get_uc(&self) -> CreateDetectableBoardUseCase<R> {
        CreateDetectableBoardUseCase::new(
            self.repository.clone(),
            self.process_name.clone()
        ).with_window_title(Some(self.window_title.clone()))
        .with_icon(self.icon.clone())
    }
}

impl_has_board!(CreateProcessBoard<R>);


impl<R: SettingsRepository + SettingsRepositoryMut> DelegatingBoard for CreateProcessBoard<R> {
    fn delegate_icon(&self) -> Option<String> {
        self.icon.as_ref().map(|i| i.name()).or_else(|| None)
    }
    fn delegate_padset(&self, _modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        let uc = self.get_uc();
        Box::new(vec![
            PadId::Five.with_data(core::Pad {
                text: Some("Create new board?".to_string()),
                board: Some(uc.board_name()),
                board_params: vec![
                    Param { name: INITIAL_PATH_PARAM.to_string(), value: "edit".to_string() },
                ],
                ..Default::default()
            }).with_tags(vec![
                Tags::RightBlack.tag(Anchor::W),
                Tags::LeftBlack.tag(Anchor::E),
                Tag { text: "esc(N)".to_string(), anchor: Anchor::NW, font_idx: Some(0), ..Default::default() },
                Tag { text: "enter(Y)".to_string(), anchor: Anchor::NE, font_idx: Some(0), ..Default::default() }
            ])
        ])
    }
    fn delegate_tags(&self) -> Vec<Tag> {
        vec![ ]
    }
    fn delegate_title(&self) -> String {
        self.window_title.clone()
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut> UiEventHandler for CreateProcessBoard<R> {
    // Handle Enter or Y to create the new board, or Esc to cancel
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(key_event) => {
                let vk_code = VIRTUAL_KEY(key_event.key as u16);
                if vk_code == VK_RETURN || vk_code == VK_Y {
                    match self.get_uc().execute() {
                        Ok(_) => UiEventResult::PadSelected(PadId::Five), // Indicate that the new board should be selected
                        Err(err) => {
                            log::error!("Error creating detectable board: {}", err);
                            UiEventResult::NotHandled
                        }
                    }
                } else if vk_code == VK_ESCAPE || vk_code == VK_N {
                    UiEventResult::PopState { result: Box::new(()) }
                } else {
                    UiEventResult::NotHandled
                }
            }
            UiEvent::RightMouseDown(_) => UiEventResult::Handled, // Ignore mouse clicks here
            _ => UiEventResult::NotHandled,
        }
    }
}

impl_board_component_generic!(CreateProcessBoard<R>);
use std::rc::Rc;

use crate::components::PadMapping;
use crate::core::{self, ActionType, DetectedIcon, Param, Resources, SettingsRepository, SettingsRepositoryMut };
use crate::{impl_board_component_generic, impl_has_board};
use crate::model::{Anchor, AnchorPin, Board, ColorScheme, CreateDetectableBoardUseCase, ModifierState, Pad, PadId, PadSet, Tag, TextStyle};

use super::{ BoardComponent, UiEventHandler, DelegatingBoard, HasBoard, UiEvent, UiEventResult, Tags, INITIAL_PATH_PARAM };


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

impl <R: SettingsRepository + SettingsRepositoryMut + 'static> HomeBoard<R> {
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
                header: Some("\nPress a NumPad key,\na modifier key or Esc".to_string()),
                ..Default::default()
            }),

            PadId::Four.with_data(core::Pad {
                text: Some("Settings".to_string()),
                board: Some("settings".to_string()),
                ..Default::default()
            }),

            PadId::Five.with_data(core::Pad {
                text: Some("Quick Tour".to_string()),
                ..Default::default()
            }),

            PadId::Six.with_data(core::Pad {
                text: Some("Documentation".to_string()),
                actions: vec![ ActionType::OpenUrl("https://github.com/ivicakukic/hotkeys-win/blob/main/README.md".to_string()) ],
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

    fn start_tour(&self) -> UiEventResult {
        let tour_board = TourBoard::new(self.board.clone(), self.repository.clone());
        UiEventResult::PushState {
            board: Box::new(tour_board),
            context: Box::new(()),
        }
    }
}

impl <R: SettingsRepository + SettingsRepositoryMut + 'static> Board for HomeBoard<R> {
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

    fn tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        vec![]
    }

}

impl <R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for HomeBoard<R> {

    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        let mapping = PadMapping { repository: self.repository.clone() };

        match event {
            UiEvent::KeyDown(key_event) => {
                let vk_code = VIRTUAL_KEY(key_event.key as u16);
                let pad_id = mapping.map(vk_code);
                match (pad_id, vk_code) {
                    (Some(PadId::Four), _) | (_, VK_S) => UiEventResult::PadSelected(PadId::Four),
                    (Some(PadId::Five), _) | (_, VK_T) => self.start_tour(),
                    (Some(PadId::Six), _) | (_, VK_D) => UiEventResult::PadSelected(PadId::Six),
                    _ => UiEventResult::NotHandled,
                }
            },
            UiEvent::RightMouseDown(me) => {
                match me.target {
                    super::MouseEventTarget::Pad(pad_id) => {
                        match pad_id {
                            PadId::Five => self.start_tour(),
                            _ => UiEventResult::NotHandled,
                        }
                    },
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
            self.process_name.clone(),
            self.window_title.clone().into(),
            self.icon.clone()
        )
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
    fn delegate_tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        vec![ ]
    }
    fn delegate_title(&self) -> String {
        let uc = self.get_uc();
        uc.board_name()
        // self.window_title.clone()
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut> UiEventHandler for CreateProcessBoard<R> {
    // Handle Enter or Y to create the new board, or Esc to cancel
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(key_event) => {
                let vk_code = VIRTUAL_KEY(key_event.key as u16);
                if vk_code == VK_RETURN || vk_code == VK_Y {
                    match self.get_uc().create_board() {
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




struct TourBoard<R: SettingsRepository + SettingsRepositoryMut> {
    board: core::Board,
    repository: Rc<R>,
    step: u32,
}

impl<R: SettingsRepository + SettingsRepositoryMut> Clone for TourBoard<R> {
    fn clone(&self) -> Self {
        Self {
            board: self.board.clone(),
            repository: self.repository.clone(),
            step: self.step,
        }
    }
}

impl <R: SettingsRepository + SettingsRepositoryMut> TourBoard<R> {
    pub fn new(board: core::Board, repository: Rc<R>) -> Self {
        Self {
            board,
            repository,
            step: 0,
        }
    }
    fn max_step(&self) -> u32 {
        7
    }
    fn step_forward(&mut self) {
        if self.step < self.max_step() {
            self.step += 1;
        }
    }
    fn reset(&mut self) {
        self.step = 0;
    }

    fn palette(&self, disabled: bool) -> Option<usize> {
        if disabled {
            Some(self.color_scheme().palette.len() - 1)
        } else {
            None
        }
    }
}

impl <R: SettingsRepository + SettingsRepositoryMut> Board for TourBoard<R> {
    fn name(&self) -> String {
        self.board.name.clone()
    }

    fn title(&self) -> String {
        "HotKeys Tour".to_string()
    }

    fn icon(&self) -> Option<String> {
        Some("icon.png".to_string())
    }

    fn color_scheme(&self) -> ColorScheme {
        let mut cs = self.repository.resolve_color_scheme(&self.board.color_scheme);
        cs.palette.push(cs.foreground2().equidistant(&cs.background()).to_hex());
        cs
    }

    fn text_style(&self) -> TextStyle {
        self.repository.resolve_text_style(&self.board.text_style)
    }

    fn padset(&self, modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        let step = self.step;

        let mut pads: Vec<Pad> = Vec::new();

        pads.push(
            match step {
                0 => PadId::One.with_data(core::Pad {
                    text: Some("Press '1' on\nnumeric keypad".to_string()),
                    ..Default::default()
                }),
                _ => PadId::One.with_data(core::Pad {
                    header: Some("\n\nPad selection:".to_string()),
                    text: Some("using numeric keypad".to_string()),
                    ..Default::default()
                })
                .as_disabled(),
            }
        );

        if step >= 1 {
            pads.push(
                match step {
                    1 => PadId::Two.with_data(core::Pad {
                        text: Some("Press '2' on\nregular keyboard".to_string()),
                        ..Default::default()
                    }),
                    _ => PadId::Two.with_data(core::Pad {
                        text: Some("or using regular keyboard".to_string()),
                        ..Default::default()
                    }).as_disabled(),
                }
            );
        }

        if step >= 2 {
            pads.push(
                match step {
                    2 => PadId::Three.with_data(core::Pad {
                        text: Some("Click here".to_string()),
                        ..Default::default()
                    }),
                    _ => PadId::Three.with_data(core::Pad {
                        text: Some("or with a mouse click.".to_string()),
                        ..Default::default()
                    }).as_disabled(),
                }
            );
        }

        if step >= 3 {
            pads.push(
                PadId::Nine.with_data(core::Pad {
                    ..Default::default()
                }).with_tags(vec![
                    Tag { text: "Use arrows to navigate".to_string(), anchor: Anchor::NE, font_idx: Some(0), color_idx: self.palette(step > 3), ..Default::default() },
                    Tag { text: if step == 3 { "" } else {  "▽" }.to_string(), anchor: Anchor::S, font_idx: Some(2), color_idx: self.palette(step > 4), ..Default::default() },
                    Tag { text: if step <= 4 { "" } else {  "as indicated by triangles." }.to_string(), anchor: Anchor::CS, font_idx: Some(0), color_idx: self.palette(true), ..Default::default() },
                ])
            );
        }

        if step >= 5 {
            pads.push(
                PadId::Seven.with_data(core::Pad {
                    ..Default::default()
                }).with_tags(vec![
                    Tag { text: "Available commands are\ndisplayed in header area".to_string(), anchor: Anchor::NW, font_idx: Some(0), color_idx: self.palette(true), ..Default::default() },
                    Tag { text: if step >= 6 { "or contextually\n" } else { "" }.to_string(), anchor: Anchor::SW, font_idx: Some(0), color_idx: self.palette(true), ..Default::default() },
                    Tag { text: if step >= 6 { "n: next step" } else { "" }.to_string(), anchor: Anchor::SW, font_idx: Some(0), color_idx: self.palette(step > 6), ..Default::default() },
                ])
            );
        }

        if step >= 7 {
            pads.push(
                PadId::Five.with_data(core::Pad {
                    text: Some("Congratulations!".to_string()),
                    ..Default::default()
                }).with_tags(vec![
                    Tag { text: "Hint: Press 'Ctrl' anywhere\nto show available commands.".to_string(), anchor: Anchor::SW, font_idx: Some(0), color_idx: self.palette(modifier.unwrap_or_default().ctrl), ..Default::default() },
                ])
            );
        }

        Box::new(pads)
    }

    fn tags(&self, modifier: Option<ModifierState>) -> Vec<Tag> {
        let step = self.step;
        let mut tags = Vec::new();

        if step >= 3 {
            tags.push(Tag { text: "▷".to_string(), anchor: Anchor::SE, font_idx: Some(2), color_idx: self.palette(step > 3), ..Default::default() });
        }

        if step >= 5 {
            tags.push(Tag { text: "c: command".to_string(), anchor: Anchor::SW, font_idx: Some(0), color_idx: self.palette(step > 5), ..Default::default() });
        }

        if step < 7 {
            tags.push(Tag { text: format!("Step {}/{}", self.step+1, self.max_step()+1), anchor: Anchor::NW, font_idx: None, ..Default::default() });
        } else if modifier.unwrap_or_default().ctrl{
            tags.push(Tag { text: "esc".to_string(), anchor: Anchor::NW, font_idx: Some(0), color_idx: None, ..Default::default() });
            tags.push(Tag { text: "r: restart".to_string(), anchor: Anchor::NE, font_idx: Some(0), color_idx: None, ..Default::default() });
        }
        tags
    }
}

impl <R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for TourBoard<R> {

    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        let step = self.step;
        match event {
            UiEvent::KeyDown(ke) => {
                let vk_code = VIRTUAL_KEY(ke.key as u16);
                match (step, vk_code) {
                    (0, VK_NUMPAD1) => {
                        self.step_forward();
                        UiEventResult::RequiresRedraw
                    },
                    (1, VK_2) => {
                        self.step_forward();
                        UiEventResult::RequiresRedraw
                    },
                    (3, VK_RIGHT) => {
                        self.step_forward();
                        UiEventResult::RequiresRedraw
                    },
                    (4, VK_DOWN) => {
                        self.step_forward();
                        UiEventResult::RequiresRedraw
                    },
                    (5, VK_C) => {
                        self.step_forward();
                        UiEventResult::RequiresRedraw
                    },
                    (6, VK_N) => {
                        self.step_forward();
                        UiEventResult::RequiresRedraw
                    },
                    (_, VK_R) => {
                        self.reset();
                        UiEventResult::RequiresRedraw
                    }
                    (_, VK_RETURN) => {
                        UiEventResult::Handled
                    }
                    _ => UiEventResult::NotHandled,
                }
            },
            UiEvent::RightMouseDown(me) => {
                if step == 2 {
                    match me.target {
                        super::MouseEventTarget::Pad(pad_id) if pad_id == PadId::Three => {
                            self.step_forward();
                            UiEventResult::RequiresRedraw
                        },
                        _ => UiEventResult::NotHandled,
                    }
                } else {
                    UiEventResult::NotHandled
                }
            }
            _ => UiEventResult::NotHandled,
        }
    }

}

impl_board_component_generic!(TourBoard<R>);
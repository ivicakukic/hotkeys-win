use std::rc::Rc;

use super::{BoardComponent, UiEventHandler, LayoutAction, LayoutBoard, UiEvent, UiEventResult};
use super::colors_board::ColorSchemeEditorBoard;
use super::fonts_board::TextStyleEditorBoard;
use super::controls::Tags;
use crate::components::{KeyboardEvent, MouseEventTarget};
use crate::core::{self, ActionType, Param, Resources, SettingsRepository, SettingsRepositoryMut };
use crate::impl_board_component_generic;
use crate::model::{Anchor, Board, ColorScheme, ModifierState, Pad, PadId, PadSet, Tag, TextStyle};

use windows::Win32::UI::Input::KeyboardAndMouse::*;

pub struct SettingsBoard<R: SettingsRepository + SettingsRepositoryMut> {
    board: core::Board,
    params: Vec<Param>,
    resources: Resources,
    repository: Rc<R>
}

impl<R: SettingsRepository + SettingsRepositoryMut> Clone for SettingsBoard<R> {
    fn clone(&self) -> Self {
        Self {
            board: self.board.clone(),
            params: self.params.clone(),
            resources: self.resources.clone(),
            repository: self.repository.clone(),
        }
    }
}

impl <R: SettingsRepository + SettingsRepositoryMut> SettingsBoard<R> {
    pub fn new(board: core::Board, params: Vec<Param>, resources: Resources, repository: Rc<R>) -> Self {
        Self {
            board,
            params,
            resources,
            repository
        }
    }

}

impl <R: SettingsRepository + SettingsRepositoryMut> Board for SettingsBoard<R> {
    fn name(&self) -> String {
        self.board.name.clone()
    }

    fn title(&self) -> String {
        "Settings".to_string()
    }

    fn icon(&self) -> Option<String> {
        Some("gear.svg".to_string())
    }

    fn color_scheme(&self) -> ColorScheme {
        self.repository.resolve_color_scheme(&self.board.color_scheme)
    }

    fn text_style(&self) -> TextStyle {
        self.repository.resolve_text_style(&self.board.text_style)
    }

    fn padset(&self, _modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        let pads: Vec<Pad> = vec![

            PadId::Two.with_data(core::Pad {
                text: Some("Move".to_string()),
                ..Default::default()
            }),
            PadId::Three.with_data(core::Pad {
                text: Some("Resize".to_string()),
                ..Default::default()
            }),

            PadId::Four.with_data(core::Pad {
                text: Some("Config file".to_string()),
                actions: vec![ ActionType::OpenUrl(
                    self.resources.settings_json()
                        .and_then(|p| p.to_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| self.resources.names().settings_json())
                )],
                ..Default::default()
            }),
            PadId::Five.with_data(core::Pad {
                text: Some("Reload".to_string()),
                board: Some(self.name()),
                ..Default::default()
            }),
            PadId::Six.with_data(core::Pad {
                text: Some("Save".to_string()),
                ..Default::default()
            }),

            PadId::Seven.with_data(core::Pad {
                text: Some("Boards".to_string()),
                ..Default::default()
            }),

            PadId::Eight.with_data(core::Pad {
                text: Some("Color Schemes".to_string()),
                ..Default::default()
            }),

            PadId::Nine.with_data(core::Pad {
                text: Some("Text Styles".to_string()),
                ..Default::default()
            }),

        ];
        Box::new(pads)
    }

    fn tags(&self) -> Vec<Tag> {
        if self.repository.is_dirty() {
            vec![ Tag { text:"(*)".to_string(), anchor: Anchor::NE, ..Default::default() } ]
        } else {
            vec![]
        }
    }

}

impl <R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for SettingsBoard<R> {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(key_event) => {
                let vk_code = VIRTUAL_KEY(key_event.key as u16);
                match vk_code {
                    VK_NUMPAD2 | VK_M => {
                        let board = LayoutBoard::new(Box::new(self.clone()), LayoutAction::Move);
                        UiEventResult::PushState {
                            board: Box::new(board),
                            context: Box::new(()),
                        }
                    },
                    VK_NUMPAD3 | VK_Z => {
                        let board = LayoutBoard::new(Box::new(self.clone()), LayoutAction::Resize);
                        UiEventResult::PushState {
                            board: Box::new(board),
                            context: Box::new(()),
                        }
                    },

                    VK_NUMPAD5 | VK_R => {
                        let _ = self.repository.reload();
                        UiEventResult::PadSelected(PadId::Five)
                    },

                    VK_NUMPAD6 | VK_S => {
                        let _ = self.repository.flush();
                        UiEventResult::RequiresRedraw
                    },

                    VK_NUMPAD7 | VK_B => {
                        let board = BoardListBoard::new(self.board.clone(), self.repository.clone());
                        UiEventResult::PushState {
                            board: Box::new(board),
                            context: Box::new(()),
                        }
                    },
                    VK_NUMPAD8 | VK_C => {
                        let board = ColorSchemeEditorBoard::new(self.repository.clone(), Some(self.color_scheme().name));
                        UiEventResult::PushState {
                            board: Box::new(board),
                            context: Box::new(()),
                        }
                    },
                    VK_NUMPAD9 | VK_T => {
                        let board = TextStyleEditorBoard::new(self.repository.clone(), Some(self.text_style().name), self.color_scheme());
                        UiEventResult::PushState {
                            board: Box::new(board),
                            context: Box::new(()),
                        }
                    },
                    _ => UiEventResult::NotHandled,
                }
            },
            UiEvent::RightMouseDown(me) => match me.target {
                MouseEventTarget::Pad(pad_id) => {
                    let key = VK_NUMPAD0.0 as u32 + pad_id.as_keypad_int() as u32;
                    self.handle_ui_event(UiEvent::KeyDown(KeyboardEvent { key, modifiers: me.modifiers }))
                },
                _ => UiEventResult::NotHandled,
            },

            _ => {
                UiEventResult::NotHandled
            }
        }


    }
}

impl_board_component_generic!(SettingsBoard<R>);


struct BoardListBoard<R: SettingsRepository + SettingsRepositoryMut> {
    board: core::Board,
    repository: Rc<R>,
    current_page: usize,
    max_page: usize,
}

impl<R: SettingsRepository + SettingsRepositoryMut> BoardListBoard<R> {
    pub fn new(board: core::Board, repository: Rc<R>) -> Self {
        let max_page = (repository.boards().len() - 6) / 3;
        Self {
            board,
            repository,
            current_page: 0,
            max_page,
        }
    }
}

impl <R: SettingsRepository + SettingsRepositoryMut> Board for BoardListBoard<R> {
    fn name(&self) -> String {
        "board_list".to_string()
    }

    fn title(&self) -> String {
        "Boards".to_string()
    }

    fn icon(&self) -> Option<String> {
        Some("gear.svg".to_string())
    }

    fn color_scheme(&self) -> ColorScheme {
        self.repository.resolve_color_scheme(&self.board.color_scheme)
    }

    fn text_style(&self) -> TextStyle {
        self.repository.resolve_text_style(&self.board.text_style)
    }

    fn padset(&self, _modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        let all_boards = self.repository.boards();
        let mut pads: Vec<Pad> = vec![];

        let start_index = self.current_page * 3;
        let end_index = (start_index + 9).min(all_boards.len());

        for cur_index in start_index..end_index {
            let board_name = &all_boards[cur_index];
            let board = self.repository.get_board(board_name).expect("Board not found");
            let pad_id = match cur_index - start_index {
                0 => PadId::Seven,
                1 => PadId::Eight,
                2 => PadId::Nine,
                3 => PadId::Four,
                4 => PadId::Five,
                5 => PadId::Six,
                6 => PadId::One,
                7 => PadId::Two,
                8 => PadId::Three,
                _ => unreachable!(),
            };

            let mut title = board.title().to_owned();
            if board.name.contains("/") {
                let parent_name = board.name.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
                if let Ok(parent_board) = self.repository.get_board(parent_name) {
                    title = format!("({}) {}", parent_board.title(), title);
                }
            }

            pads.push(
                pad_id.with_data(core::Pad {
                    text: Some(title.to_string()),
                    icon: board.icon,
                    board: Some(board.name),
                    ..Default::default()
                })
            );
        }
        Box::new(pads)
    }

    fn tags(&self) -> Vec<Tag> {
        let mut tags = vec![
            Tag { text: "esc".to_string(), anchor: Anchor::NW, font_idx: Some(0), color_idx: None, ..Default::default() }
        ];

        if self.current_page > 0 {
            tags.push(Tags::UpWhite.tag(Anchor::NE));
        }
        if self.current_page < self.max_page {
            tags.push(Tags::DownWhite.tag(Anchor::SE));
        }

        tags
    }
}


impl <R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for BoardListBoard<R> {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(key_event) => {
                let vk_code = VIRTUAL_KEY(key_event.key as u16);
                match vk_code {
                    VK_ESCAPE => {
                        UiEventResult::PopState { result: Box::new(()) }
                    },
                    VK_UP => {
                        if self.current_page > 0 {
                            self.current_page -= 1;
                            UiEventResult::RequiresRedraw
                        } else {
                            UiEventResult::NotHandled
                        }
                    },
                    VK_DOWN => {
                        if self.current_page < self.max_page {
                            self.current_page += 1;
                            UiEventResult::RequiresRedraw
                        } else {
                            UiEventResult::NotHandled
                        }
                    },
                    _ => UiEventResult::NotHandled,
                }
            },

            _ => {
                UiEventResult::NotHandled
            }
        }
    }
}


impl_board_component_generic!(BoardListBoard<R>);
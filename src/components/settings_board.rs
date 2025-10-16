use std::rc::Rc;

use super::{
    BoardComponent, UiEventHandler, LayoutAction, LayoutBoard, UiEvent, UiEventResult, Tags, KeyboardEvent, MouseEventTarget, HasBoard,
    error_board, string_editor_board, success_board,
    colors_board::ColorSchemeEditorBoard, fonts_board::TextStyleEditorBoard,
};

use crate::components::{yes_no_question_board, yes_no_warning_board, ChildWindowRequest, DelegatingBoard, DelegatingHandler, HasHandler, PadMapping};
use crate::core::integration::ChainParams;
use crate::core::{self, ActionType, BoardType, Detection, Param, Resources, SettingsRepository, SettingsRepositoryMut };
use crate::ui::dialogs::open_chain_editor;
use crate::{impl_board_component_generic};
use crate::model::{ConvertToBoardChainUseCase, DeleteBoardUseCase, create_board, create_new_chain_with_board, Anchor, Board, ColorScheme, ModifierState, Pad, PadId, PadSet, Tag, TextStyle};

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

            PadId::One.with_data(core::Pad {
                text: Some("🏠".to_string()),
                board: Some("home".to_string()),
                ..Default::default()
            }).with_tags(vec![
                Tag { text: "Home".to_string(), anchor: Anchor::Rel(0.5, 0.65), font_idx: Some(0), ..Default::default() }
            ]),

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

    fn tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
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
                let pad_id = PadMapping { repository: self.repository.clone() }.map(vk_code);

                match (pad_id, vk_code) {
                    (Some(PadId::Two), _) | (_, VK_M) => {
                        let board = LayoutBoard::new(Box::new(self.clone()), LayoutAction::Move);
                        UiEventResult::PushState {
                            board: Box::new(board),
                            context: Box::new(()),
                        }
                    },
                    (Some(PadId::Three), _) | (_, VK_Z) => {
                        let board = LayoutBoard::new(Box::new(self.clone()), LayoutAction::Resize);
                        UiEventResult::PushState {
                            board: Box::new(board),
                            context: Box::new(()),
                        }
                    },

                    (Some(PadId::Five), _) | (_, VK_R) => {
                        let _ = self.repository.reload();
                        UiEventResult::PadSelected(PadId::Five)
                    },

                    (Some(PadId::Six), _) | (_, VK_S) => {
                        let _ = self.repository.flush();
                        UiEventResult::RequiresRedraw
                    },

                    (Some(PadId::Seven), _) | (_, VK_B) => {
                        let board = MainBoardList::new(self.board.clone(), self.repository.clone());
                        UiEventResult::PushState {
                            board: Box::new(board),
                            context: Box::new(()),
                        }
                    },
                    (Some(PadId::Eight), _) | (_, VK_C) => {
                        let board = ColorSchemeEditorBoard::new(self.repository.clone(), Some(self.color_scheme().name));
                        UiEventResult::PushState {
                            board: Box::new(board),
                            context: Box::new(()),
                        }
                    },
                    (Some(PadId::Nine), _) | (_, VK_T) => {
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


/// Base board list implementation with filtering

struct BoardListBase<R: SettingsRepository + SettingsRepositoryMut> {
    board: core::Board,
    repository: Rc<R>,
    filter_function: Rc<dyn Fn(&core::Board) -> bool>,
    current_page: usize,
}

impl<R: SettingsRepository + SettingsRepositoryMut> Clone for BoardListBase<R> {
    fn clone(&self) -> Self {
        Self {
            board: self.board.clone(),
            repository: self.repository.clone(),
            filter_function: self.filter_function.clone(),
            current_page: self.current_page,
        }
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> BoardListBase<R> {
    pub fn new<F: 'static + Fn(&core::Board) -> bool>(
        board: core::Board,
        repository: Rc<R>,
        filter_function: F
    ) -> Self {
        Self {
            board,
            repository: repository.clone(),
            filter_function: Rc::new(filter_function),
            current_page: 0,
        }
    }

    fn boards(&self) -> Vec<core::Board> {
        self.repository.boards().iter()
            .filter_map(|name| self.repository.get_board(name).ok())
            .filter(|b| (self.filter_function)(b))
            .collect()
    }

    /// 0-9 boards: 0 pages
    /// 10-12 boards: 1 page
    /// 13-15 boards: 2 pages
    /// ...
    fn max_page(&self) -> usize {
        let num_boards = self.boards().len();
        if num_boards <= 9 {
            0
        } else {
            (num_boards as f64 / 3.0).ceil() as usize - 3
        }
    }

    fn clamp_current_page(&mut self) {
        let max_page = self.max_page();
        if self.current_page > max_page {
            self.current_page = max_page;
        }
    }

    fn move_to_end(&mut self) {
        self.current_page = self.max_page();
    }

    fn get_pads(&self, _modifier: Option<ModifierState>) -> Vec<Pad> {
        let all_boards = self.boards();
        let mut pads: Vec<Pad> = vec![];

        let start_index = self.current_page * 3;
        let end_index = (start_index + 9).min(all_boards.len());

        for cur_index in start_index..end_index {
            let board = &all_boards[cur_index];
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

            let mut header = "".to_string();
            let title = board.title().to_owned();

            if board.name.contains("/") {
                let parent_name = board.name.rsplit_once('/').map(|(p, _)| p).unwrap_or("");
                if let Ok(parent_board) = self.repository.get_board(parent_name) {
                    header = parent_board.title().to_string();
                }
            }

            let tags = match &board.board_type {
                BoardType::Home => match board.name.as_str() {
                    "home" => vec!["🏠"],
                    "settings" => vec!["⚙"],
                    _ => vec![]
                },
                BoardType::Chain(_) => match board.detection {
                    Detection::None => vec!["🔗"],
                    _ => vec!["🔗", "🎯"],
                },
                BoardType::Static => match board.detection {
                    Detection::None => vec!["📋"],
                    _ => vec!["🎯"],
                },
                _ => vec![],
            };

            pads.push(pad_id
                .with_data(core::Pad {
                    header: Some(header),
                    text: Some(title),
                    icon: board.icon.clone(),
                    board: Some(board.name.clone()),
                    ..Default::default()
                })
                .with_tags(vec![
                    Tag { text: tags.join(""), anchor: Anchor::NW, font_idx: Some(0), ..Default::default() }
                ])
            );
        }
        pads
    }

    fn pad_mapping(&self) -> PadMapping<R> {
        PadMapping { repository: self.repository.clone() }
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> Board for BoardListBase<R> {
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

    fn padset(&self, modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        Box::new(self.get_pads(modifier))
    }

    fn tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        let mut tags = vec![
            Tag { text: "esc".to_string(), anchor: Anchor::NW, font_idx: Some(0), ..Default::default() }
        ];

        if self.current_page > 0 {
            tags.push(Tags::UpWhite.tag(Anchor::NE));
        }
        if self.current_page < self.max_page() {
            tags.push(Tags::DownWhite.tag(Anchor::SE));
        }

        tags
    }
}

impl <R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for BoardListBase<R> {
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
                        if self.current_page < self.max_page() {
                            self.current_page += 1;
                            UiEventResult::RequiresRedraw
                        } else {
                            UiEventResult::NotHandled
                        }
                    },
                    other => {
                        if let Some(pad_id) = self.pad_mapping().map(other) {
                            return UiEventResult::PadSelected(pad_id);
                        }
                        UiEventResult::NotHandled
                    }
                }
            },
            UiEvent::RightMouseDown(me) => match me.target {
                MouseEventTarget::Pad(pad_id) => {
                    UiEventResult::PadSelected(pad_id)
                },
                _ => UiEventResult::NotHandled,
            },

            _ => {
                UiEventResult::NotHandled
            }
        }
    }

}


impl_board_component_generic!(BoardListBase<R>);


/// Main boards overview screen

struct MainBoardList<R: SettingsRepository + SettingsRepositoryMut> {
    inner: BoardListBase<R>,
    repository: Rc<R>,
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> MainBoardList<R> {
    pub fn new(board: core::Board, repository: Rc<R>) -> Self {
        Self {
            inner: BoardListBase::new(board, repository.clone(), |_| true),
            repository
        }
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> HasBoard for MainBoardList<R> {
    fn board(&self) -> &dyn Board {
        &self.inner
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> HasHandler for MainBoardList<R> {
    fn handler(&mut self) -> Option<&mut dyn UiEventHandler> {
        Some(&mut self.inner)
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingBoard for MainBoardList<R> {
    fn delegate_tags(&self, modifier: Option<ModifierState>) -> Vec<Tag> {
        let mut tags = self.inner.tags(modifier);
        if modifier.unwrap_or_default().ctrl {
            tags.push(
                Tag { text: "(+) create    (c) collections\n(-) delete    (m) main boards".to_string(), anchor: Anchor::SW, font_idx: Some(0), ..Default::default() },
            );
        }
        tags
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingHandler for MainBoardList<R> {
    fn delegate_handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(key_event) => {
                let vk_code = VIRTUAL_KEY(key_event.key as u16);
                match vk_code {
                    VK_OEM_MINUS | VK_SUBTRACT => {
                        UiEventResult::PushState {
                            board: Box::new(DeleteBoardList::new(self.inner.board.clone(), self.repository.clone())),
                            context: Box::new(()),
                        }
                    },
                    VK_OEM_PLUS | VK_ADD => {
                        UiEventResult::PushState {
                            board: Box::new(CreateBoardList::new(self.inner.board.clone(), self.repository.clone())),
                            context: Box::new(()),
                        }
                    },
                    VK_C => {
                        UiEventResult::PushState {
                            board: Box::new(ChainBoardList::new(self.inner.board.clone(), self.repository.clone())),
                            context: Box::new(()),
                        }
                    },
                    VK_M => {
                        UiEventResult::PushState {
                            board: Box::new(ConvertBoardList::new(self.inner.board.clone(), self.repository.clone())),
                            context: Box::new(()),
                        }
                    },
                    _ => self.inner.handle_ui_event(event),
                }
            },
            _ => self.inner.handle_ui_event(event),
        }
    }

    fn delegate_handle_child_result(&mut self, _context: Box<dyn std::any::Any>, _result: Box<dyn std::any::Any>) -> UiEventResult {
        self.inner.clamp_current_page();
        UiEventResult::RequiresRedraw
    }
}

impl_board_component_generic!(MainBoardList<R>);


/// Delete boards screen

struct DeleteBoardList<R: SettingsRepository + SettingsRepositoryMut> {
    inner: BoardListBase<R>,
    repository: Rc<R>
}

enum DeleteBoardListContext {
    Confirmation(String),
    Success,
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DeleteBoardList<R> {
    pub fn new(board: core::Board, repository: Rc<R>) -> Self {
        Self {
            inner: BoardListBase::new(
                board,
                repository.clone(),
                |b| { !matches!(b.board_type, BoardType::Home) }
            ),
            repository
        }
    }

    fn uc(&self, board_name: &str) -> DeleteBoardUseCase<R> {
        DeleteBoardUseCase::new(self.repository.clone(), board_name.to_string())
    }

    fn request_delete_board(&mut self, pad_id: PadId, modifiers: ModifierState) -> UiEventResult {
        match self.padset(Some(modifiers)).pad(pad_id).board().clone() {
            Some(board_name) => {
                return UiEventResult::PushState {
                    board: Box::new(yes_no_warning_board(format!("Delete board\n\"{}\"?", board_name), self)),
                    context: Box::new(DeleteBoardListContext::Confirmation(board_name)),
                }
            },
            None => UiEventResult::NotHandled
        }
    }

    fn delete_board(&mut self, board_name: &str) -> UiEventResult {
        match self.uc(board_name).delete() {
            Ok(_) => {
                UiEventResult::PushState {
                    board: Box::new(success_board(format!("Deleted\n\"{}\"", board_name), self)),
                    context: Box::new(DeleteBoardListContext::Success),
                }
            }
            Err(err) => {
                UiEventResult::PushState {
                    board: Box::new(error_board(format!("{}", err), self)),
                    context: Box::new(()),
                }
            }
        }
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> HasBoard for DeleteBoardList<R> {
    fn board(&self) -> &dyn Board {
        &self.inner
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> HasHandler for DeleteBoardList<R> {
    fn handler(&mut self) -> Option<&mut dyn UiEventHandler> {
        Some(&mut self.inner)
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingBoard for DeleteBoardList<R> {
    fn delegate_title(&self) -> String {
        "Delete Board".to_string()
    }
    fn delegate_tags(&self, modifier: Option<ModifierState>) -> Vec<Tag> {
        self.inner.tags(modifier).into_iter().chain(
            vec![
                Tag { text: "1-9: delete".to_string(), anchor: Anchor::SW, font_idx: Some(0), ..Default::default() },
            ]
        ).collect()
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingHandler for DeleteBoardList<R> {
    fn delegate_handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match self.inner.handle_ui_event(event.clone()) {
            UiEventResult::PadSelected(pad_id) => {
                self.request_delete_board(pad_id, event.modifiers())
            },
            result => result,
        }
    }

    fn delegate_handle_child_result(&mut self, context: Box<dyn std::any::Any>, result: Box<dyn std::any::Any>) -> UiEventResult {
        if let Some(context) = context.downcast_ref::<DeleteBoardListContext>() {
            match context {
                DeleteBoardListContext::Confirmation(board_name) => {
                    if let Some(confirmed) = result.downcast_ref::<bool>() {
                        if *confirmed {
                            return self.delete_board(board_name);
                        }
                    }
                },
                DeleteBoardListContext::Success => {
                    self.inner.clamp_current_page();
                    return UiEventResult::RequiresRedraw;
                }
            }
        }
        UiEventResult::NotHandled
    }
}

impl_board_component_generic!(DeleteBoardList<R>);


/// Create boards screen

struct CreateBoardList<R: SettingsRepository + SettingsRepositoryMut> {
    inner: BoardListBase<R>,
    repository: Rc<R>
}

enum CreateBoardListContext {
    NewSubBoard(String),
    NewCollection(String),
    CollectionCreated,
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> CreateBoardList<R> {
    pub fn new(board: core::Board, repository: Rc<R>) -> Self {
        Self {
            inner: BoardListBase::new(
                board,
                repository.clone(),
                |b| { matches!(b.board_type, BoardType::Static)  }
            ),
            repository
        }
    }

    fn request_new(&self, pad_id: PadId, modifiers: ModifierState) -> UiEventResult {
        match self.padset(Some(modifiers)).pad(pad_id).board().clone() {
            Some(board_name) => {
                if modifiers.ctrl {
                    return UiEventResult::PushState {
                        board: Box::new(yes_no_question_board(
                            format!("Create new Collection with board\n\"{}\"", board_name), self
                        )),
                        context: Box::new(CreateBoardListContext::NewCollection(board_name)),
                    }
                } else {
                    UiEventResult::PushState {
                        board: Box::new(string_editor_board("New board".to_string(), self, "Create Board".to_string())),
                        context: Box::new(CreateBoardListContext::NewSubBoard(board_name)),
                    }
                }
            },
            None => UiEventResult::NotHandled
        }
    }

    fn create_subboard(&mut self, parent_board: String, keyword: String) -> UiEventResult {
        let result = create_board(self.repository.as_ref(), parent_board, keyword);
        let board = match result {
            Ok(board) => {
                self.inner.move_to_end();
                success_board(format!("Created\n\"{}\"", board.name), self)
            }
            Err(err) => {
                error_board(format!("{}", err), self)
            }
        };
        return UiEventResult::PushState {
            board: Box::new(board),
            context: Box::new(()),
        };
    }

    fn create_collection(&mut self, board_name: String) -> UiEventResult {
        match create_new_chain_with_board(self.repository.as_ref(), &board_name) {
            Ok(collection) => {
                UiEventResult::PushState {
                    board: Box::new(success_board(format!("Created collection\n\"{}\"", collection.name), self)),
                    context: Box::new(CreateBoardListContext::CollectionCreated),
                }
            }
            Err(err) => {
                UiEventResult::PushState {
                    board: Box::new(error_board(format!("{}", err), self)),
                    context: Box::new(()),
                }
            }
        }
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> HasBoard for CreateBoardList<R> {
    fn board(&self) -> &dyn Board {
        &self.inner
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> HasHandler for CreateBoardList<R> {
    fn handler(&mut self) -> Option<&mut dyn UiEventHandler> {
        Some(&mut self.inner)
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingBoard for CreateBoardList<R> {
    fn delegate_title(&self) -> String {
        "Create".to_string()
    }
    fn delegate_tags(&self, modifier: Option<ModifierState>) -> Vec<Tag> {
        self.inner.tags(modifier).into_iter().chain(
            vec![ Tag {
                text: format!("New {}", if modifier.unwrap_or_default().ctrl { "Collection" } else { "Board" }),
                anchor: Anchor::SW, font_idx: Some(0), ..Default::default() }, ]
        ).collect()
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingHandler for CreateBoardList<R> {
    fn delegate_handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        let result = self.inner.handle_ui_event(event.clone());
        if let UiEventResult::PadSelected(pad_id) = result {
            return self.request_new(pad_id, event.modifiers());
        }
        result
    }

    fn delegate_handle_child_result(&mut self, context: Box<dyn std::any::Any>, result: Box<dyn std::any::Any>) -> UiEventResult {
        if let Some(context) = context.downcast_ref::<CreateBoardListContext>() {
            match context {
                CreateBoardListContext::NewSubBoard(parent_board) => {
                    if let Some(keyword) = result.downcast_ref::<String>() {
                        if !keyword.is_empty() {
                            return self.create_subboard(parent_board.clone(), keyword.clone());
                        }
                    }
                }
                CreateBoardListContext::NewCollection(board_name) => {
                    if let Some(confirmed) = result.downcast_ref::<bool>() {
                        if *confirmed {
                            return self.create_collection(board_name.clone());
                        }
                    }
                }
                CreateBoardListContext::CollectionCreated => {
                    return UiEventResult::ReplaceState {
                        board: Box::new(ChainBoardList::new(self.inner.board.clone(), self.repository.clone()))
                    };
                }
            }
        }
        UiEventResult::NotHandled
    }
}

impl_board_component_generic!(CreateBoardList<R>);



/// Convert boards screen

struct ConvertBoardList<R: SettingsRepository + SettingsRepositoryMut> {
    inner: BoardListBase<R>,
    repository: Rc<R>
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> ConvertBoardList<R> {
    pub fn new(board: core::Board, repository: Rc<R>) -> Self {
        Self {
            inner: BoardListBase::new(
                board,
                repository.clone(),
                |b| { !matches!(b.detection, Detection::None) }
            ),
            repository
        }
    }

    fn uc(&self, board_name: &String) -> ConvertToBoardChainUseCase<R> {
        ConvertToBoardChainUseCase::new(self.repository.clone(), board_name.clone())
    }

    fn request_convert_board(&mut self, board_name: String) -> UiEventResult {
        match self.repository.get_board(&board_name) {
            Ok(_) => {
                if let Err(err) = self.uc(&board_name).validate() {
                    return UiEventResult::PushState {
                        board: Box::new(error_board(format!("{}", err), self)),
                        context: Box::new(board_name),
                    };
                }
                return UiEventResult::PushState {
                    board: Box::new(yes_no_warning_board(format!("Convert\n\"{}\"\nto Collection?", board_name), self)),
                    context: Box::new(board_name),
                }
            },
            Err(_) => UiEventResult::NotHandled
        }

    }

    fn convert_to_board_chain(&mut self, target_board: String) -> UiEventResult {
        let result = self.uc(&target_board).convert();
        let board = match result {
            Ok(()) => success_board(format!("Converted\n\"{}\"", target_board), self),
            Err(err) => error_board(format!("{}", err), self),
        };
        return UiEventResult::PushState {
            board: Box::new(board),
            context: Box::new(()),
        };
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> HasBoard for ConvertBoardList<R> {
    fn board(&self) -> &dyn Board {
        &self.inner
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> HasHandler for ConvertBoardList<R> {
    fn handler(&mut self) -> Option<&mut dyn UiEventHandler> {
        Some(&mut self.inner)
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingBoard for ConvertBoardList<R> {
    fn delegate_title(&self) -> String {
        "Main Boards".to_string()
    }
    fn delegate_tags(&self, modifier: Option<ModifierState>) -> Vec<Tag> {
        self.inner.tags(modifier).into_iter().chain(
            vec![ Tag { text: "1-9: convert to Collection".to_string(), anchor: Anchor::SW, font_idx: Some(0), ..Default::default() }, ]
        ).collect()
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingHandler for ConvertBoardList<R> {
    fn delegate_handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        let result = self.inner.handle_ui_event(event.clone());
        if let UiEventResult::PadSelected(pad_id) = result {
            match self.padset(Some(event.modifiers())).pad(pad_id).board().clone() {
                Some(board_name) => {
                    return self.request_convert_board(board_name);
                },
                None => return UiEventResult::NotHandled
            }
        }
        result
    }

    fn delegate_handle_child_result(&mut self, context: Box<dyn std::any::Any>, result: Box<dyn std::any::Any>) -> UiEventResult {
        if let Some(board_name) = context.downcast_ref::<String>() {
            if let Some(confirmed) = result.downcast_ref::<bool>() {
                if *confirmed {
                    return self.convert_to_board_chain(board_name.clone());
                }
            }
        }
        UiEventResult::NotHandled
    }
}

impl_board_component_generic!(ConvertBoardList<R>);



/// Chain boards screen
struct ChainBoardList<R: SettingsRepository + SettingsRepositoryMut> {
    inner: BoardListBase<R>,
    repository: Rc<R>,
    selected_board: Option<String>
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> ChainBoardList<R> {
    pub fn new(board: core::Board, repository: Rc<R>) -> Self {
        Self {
            inner: BoardListBase::new(
                board,
                repository.clone(),
                |b| { matches!(b.board_type, BoardType::Chain(_))  }
            ),
            repository,
            selected_board: None
        }
    }

    fn all_non_chain_boards(&self) -> Vec<String> {
        self.repository.boards().iter()
            .filter_map(|name| self.repository.get_board(name).ok())
            .filter(|b| ! (self.inner.filter_function)(b))
            .map(|b| b.name)
            .collect()
    }

    fn get_selected_board(&self) -> Result<core::Board, Box<dyn std::error::Error>> {
        if let Some(board_name) = &self.selected_board {
            if let Ok(board) = self.repository.get_board(board_name) {
                return Ok(board);
            }
        }
        Err("No valid Collection board selected".into())
    }

    fn get_selected_board_params(&self) -> Result<ChainParams, Box<dyn std::error::Error>> {
        let board = self.get_selected_board()?;
        if let BoardType::Chain(params) = &board.board_type {
            return Ok(params.clone());
        }
        Err("Selected board is not a Collection board".into())
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> HasBoard for ChainBoardList<R> {
    fn board(&self) -> &dyn Board {
        &self.inner
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> HasHandler for ChainBoardList<R> {
    fn handler(&mut self) -> Option<&mut dyn UiEventHandler> {
        Some(&mut self.inner)
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingBoard for ChainBoardList<R> {
    fn delegate_title(&self) -> String {
        "Collections".to_string()
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingHandler for ChainBoardList<R> {
    fn delegate_handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        let result = self.inner.handle_ui_event(event.clone());
        if let UiEventResult::PadSelected(pad_id) = result {
            match self.padset(Some(event.modifiers())).pad(pad_id).board().clone() {
                Some(board_name) => {
                    self.selected_board = Some(board_name);
                    return UiEventResult::RequestChildWindow(ChildWindowRequest::ChainEditor)
                },
                None => return UiEventResult::NotHandled
            }
        }
        result
    }

    fn delegate_create_child_window(&mut self, request: ChildWindowRequest, parent_hwnd: windows::Win32::Foundation::HWND) -> UiEventResult {
        if let ChildWindowRequest::ChainEditor = request {
            if let Some(_) = &self.selected_board {
                if let Ok(params) = self.get_selected_board_params() {
                    if let Some((new_boards, new_initial)) = open_chain_editor(
                        params.boards(), params.initial_board.clone(),
                        self.all_non_chain_boards(),
                        Some(parent_hwnd)
                    ) {
                        let mut new_params = params.clone();
                        new_params.boards = new_boards.join(",");
                        new_params.initial_board = Some(new_initial);

                        if let Ok(mut board) = self.get_selected_board() {
                            board.board_type = BoardType::Chain(new_params);
                            if let Err(err) = self.repository.set_board(board) {
                                return UiEventResult::PushState {
                                    board: Box::new(error_board(format!("{}", err), self)),
                                    context: Box::new(()),
                                }
                            }
                        }
                    }
                }
            }
        }
        UiEventResult::NotHandled
    }
}

impl_board_component_generic!(ChainBoardList<R>);
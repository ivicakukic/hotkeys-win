
use std::rc::Rc;

use crate::core::{Resources, SettingsRepository, SettingsRepositoryMut, Param, Params};
use crate::impl_board_component_generic;
use crate::input::ModifierState;
use crate::model::{Board, Tag};
use crate::components::{BoardComponent, DelegatingBoard, HasBoard, MainBoard, StateMachineBoard, Tags, UiEvent, UiEventHandler, UiEventResult};
use crate::app::{BoardFactory, BoardRuntimeContext};

use windows::Win32::UI::Input::KeyboardAndMouse::*;


pub struct BoardChain<R: SettingsRepository + SettingsRepositoryMut> {
    boards: Vec<String>,
    current: String,
    resources: Resources,
    repository: Rc<R>,
    inner: MainBoard<R>
}

impl<R: SettingsRepository + SettingsRepositoryMut> HasBoard for BoardChain<R> {
    fn board(&self) -> &dyn Board {
        self.inner.board()
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> BoardChain<R> {

    pub fn new(boards: Vec<String>, initial_board: Option<String>, initial_params: Vec<Param>, resources: Resources, repository: Rc<R>) -> Self {
        assert!(!boards.is_empty(), "BoardChain requires at least one board");

        let mut current = initial_board.unwrap_or_else(|| boards.first().cloned().unwrap_or_default());

        if !boards.contains(&current) {
            log::warn!("Initial board '{}' not found in board chain, defaulting to first board", current);
            current = boards.first().cloned().unwrap_or_default();
        }

        let inner = MainBoard::new(
            current.clone(),
            initial_params,
            resources.clone(),
            repository.clone()
        );

        Self {
            boards,
            current,
            resources,
            repository,
            inner
        }
    }

    fn create_main_board(&self) -> MainBoard<R> {
        MainBoard::new(
            self.current.clone(),
            vec![],
            self.resources.clone(),
            self.repository.clone()
        )
    }

    fn page_string(&self) -> String {
        let current_index = self.boards.iter().position(|b| b == &self.current).unwrap_or(0);
        format!("{}/{}", current_index + 1, self.boards.len())
    }

    fn move_next(&mut self) -> UiEventResult {
        if let Some(current_index) = self.boards.iter().position(|b| b == &self.current) {
            let next_index = (current_index + 1) % self.boards.len();
            self.current = self.boards[next_index].clone();
            self.inner = self.create_main_board();
            return UiEventResult::RequiresRedraw
        }
        UiEventResult::NotHandled
    }

    fn move_previous(&mut self) -> UiEventResult {
        if let Some(current_index) = self.boards.iter().position(|b| b == &self.current) {
            let previous_index = if current_index == 0 {
                self.boards.len() - 1
            } else {
                current_index - 1
            };
            self.current = self.boards[previous_index].clone();
            self.inner = self.create_main_board();
            return UiEventResult::RequiresRedraw
        }
        UiEventResult::NotHandled
    }

    fn key_down(&mut self, key: u32, _modifiers: ModifierState) -> UiEventResult {
        let vk_code = VIRTUAL_KEY(key as u16);
        match vk_code {
            VK_RIGHT => self.move_next(),
            VK_LEFT => self.move_previous(),
            _ => UiEventResult::NotHandled
        }
    }


}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingBoard for BoardChain<R> {
    fn delegate_tags(&self, modifier: Option<crate::input::ModifierState>) -> Vec<Tag> {
        let mut tags = self.inner.tags(modifier);
        if modifier.is_none() || modifier.unwrap().is_none() {
            tags.push(Tags::LeftTextRight(self.page_string()).default());
        }
        tags
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for BoardChain<R> {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(ke) => {
                let result = self.key_down(ke.key, ke.modifiers);
                match result {
                    UiEventResult::NotHandled => {},
                    _ => return result
                }
            },
            _ => {}
        }
        match self.inner.handler() {
            Some(handler) => handler.handle_ui_event(event),
            None => UiEventResult::NotHandled
        }
    }

    fn activate(&mut self) -> UiEventResult {
        self.inner.activate()
    }

    fn create_child_window(&mut self, _request: super::ChildWindowRequest, _parent_hwnd: windows::Win32::Foundation::HWND) -> UiEventResult {
        self.inner.create_child_window(_request, _parent_hwnd)
    }

    fn handle_child_result(&mut self, _context: Box<dyn std::any::Any>, _result: Box<dyn std::any::Any>) -> UiEventResult {
        self.inner.handle_child_result(_context, _result)
    }
}

impl_board_component_generic!(BoardChain<R>);



#[allow(dead_code)]
pub struct BoardChainFactory {}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> BoardFactory<R> for BoardChainFactory {
    fn create_board(
        &self,
        context: &BoardRuntimeContext<R>,
        board: &crate::core::Board,
        params: Vec<Param>
    ) -> Result<Box<dyn BoardComponent>, Box<dyn std::error::Error>> {

        let board_params = match &board.board_type {
            crate::core::BoardType::Custom(params) => params,
            _ => return Err("BoardChainFactory can only create Custom boards".into()),
        };

        let params = board_params.merge_params(params);
        let boards: Vec<String> = params.get_param_as::<String>("boards")
            .ok_or("BoardChain requires 'boards' parameter")?
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        let initial_board: Option<String> = params.get_param_as::<String>("initial_board");

        let initial_params = params.into_iter()
            .filter(|p| p.name != "boards" && p.name != "initial_board")
            .collect::<Vec<Param>>();

        Ok(
            Box::new(
                StateMachineBoard::new(
                    Box::new(
                        BoardChain::new(
                            boards,
                            initial_board,
                            initial_params,
                            context.resources.clone(),
                            context.repository.clone()
                        )
                    )
                )
            )
        )
    }

}
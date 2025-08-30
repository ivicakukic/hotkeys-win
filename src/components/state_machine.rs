use std::any::Any;
use windows::Win32::Foundation::HWND;

use super::{BoardComponent, UiEvent, UiEventResult, ChildWindowRequest};

pub struct BoardStateMachine {
    stack: Vec<StateFrame>
}

struct StateFrame {
    board: Box<dyn BoardComponent>,
    #[allow(dead_code)]
    context: Option<Box<dyn Any>>, // What this state was spawned to fulfill
}

impl BoardStateMachine {
    pub fn new(initial_board: Box<dyn BoardComponent>) -> Self {
        let mut board_self = Self {
            stack: vec![StateFrame { board: initial_board, context: None }],
        };
        let activate_result = board_self.current_board()
            .handler()
            .map_or(UiEventResult::NotHandled, |h| h.activate());

        if let UiEventResult::PushState { board, context } = activate_result {
            board_self.process_state_result(UiEventResult::PushState { board, context });
        }
        board_self
    }

    pub fn current_board(&mut self) -> &mut dyn BoardComponent {
        self.stack.last_mut().unwrap().board.as_mut()
    }

    pub fn current_board_ref(&self) -> &dyn BoardComponent {
        self.stack.last().unwrap().board.as_ref()
    }

    pub fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        let result = if let Some(handler) = self.current_board().handler() {
            handler.handle_ui_event(event)
        } else {
            UiEventResult::NotHandled
        };

        self.process_state_result(result)
    }

    pub fn create_child_window(&mut self, request: ChildWindowRequest, parent_hwnd: HWND) -> UiEventResult {
        if let Some(handler) = self.current_board().handler() {
            let result = handler.create_child_window(request, parent_hwnd);
            self.process_state_result(result)
        } else {
            UiEventResult::NotHandled
        }
    }

    pub fn process_state_result(&mut self, result: UiEventResult) -> UiEventResult {
        match result {
            UiEventResult::PushState { board, context: contract } => {
                self.stack.push(StateFrame { board, context: Some(contract) });
                log::info!("Pushed new state, stack depth now {}", self.stack_depth());

                let activate_result = self.current_board()
                    .handler()
                    .map_or(UiEventResult::NotHandled, |h| h.activate());

                if let UiEventResult::PushState { board, context } = activate_result {
                    self.process_state_result(UiEventResult::PushState { board, context });
                }

                UiEventResult::RequiresRedraw
            }

            UiEventResult::PopState { result } => {
                if self.stack_depth() > 1 {
                    let popped_frame = self.stack.pop().unwrap();
                    log::info!("Popped state, stack depth now {}", self.stack_depth());

                    if let Some(handler) = self.current_board().handler() {
                        let _ = handler.handle_child_result(popped_frame.context.unwrap(), result);
                    }
                }
                UiEventResult::RequiresRedraw
            }

            UiEventResult::ReplaceState { board } => {
                if let Some(current) = self.stack.last_mut() {
                    current.board = board;
                }
                UiEventResult::RequiresRedraw
            }

            other => other,
        }
    }

    pub fn stack_depth(&self) -> usize {
        self.stack.len()
    }

}

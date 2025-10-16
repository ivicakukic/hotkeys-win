use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;

use crate::model::{Board, ModifierState, PadId};
use std::any::Any;

pub trait BoardComponent {
    fn data(&self) -> &dyn Board;
    fn handler(&mut self) -> Option<&mut dyn UiEventHandler> {
        None
    }
}

pub trait UiEventHandler {
    fn handle_ui_event(&mut self, _event: UiEvent) -> UiEventResult {
        UiEventResult::NotHandled
    }

    fn create_child_window(&mut self, _request: ChildWindowRequest, _parent_hwnd: HWND) -> UiEventResult {
        UiEventResult::NotHandled
    }

    fn handle_child_result(&mut self, _context: Box<dyn Any>, _result: Box<dyn Any>) -> UiEventResult {
        UiEventResult::NotHandled
    }

    fn activate(&mut self) -> UiEventResult {
        UiEventResult::NotHandled
    }
}


#[derive(Debug, Clone)]
pub enum UiEvent {
    KeyDown(KeyboardEvent),
    KeyUp(KeyboardEvent),
    RightMouseDown(MouseEvent),
}

impl UiEvent {
    pub fn modifiers(&self) -> ModifierState {
        match self {
            UiEvent::KeyDown(event) => event.modifiers,
            UiEvent::KeyUp(event) => event.modifiers,
            UiEvent::RightMouseDown(event) => event.modifiers,
        }
    }
}

pub enum Direction {
    Left, Right, Up, Down
}

pub enum SetWindowPosCommand {
    Move(Direction),
    Size(Direction),
}

pub enum UiEventResult {
    Handled,
    NotHandled,
    RequiresRedraw,
    PadSelected(PadId),
    CloseWindow,
    SetWindowPos(SetWindowPosCommand),
    RequestChildWindow(ChildWindowRequest),

    // State machine operations
    PushState {
        board: Box<dyn BoardComponent>,
        context: Box<dyn Any>, // Type-erased context
    },

    PopState {
        result: Box<dyn Any>, // Type-erased result
    },

    ReplaceState {
        board: Box<dyn BoardComponent>,
    },
}


#[derive(Copy, Clone, Debug)]
pub struct KeyboardEvent {
    pub key: u32,
    pub modifiers: ModifierState,
}

impl From<KeyboardEvent> for VIRTUAL_KEY {
    fn from(event: KeyboardEvent) -> Self {
        VIRTUAL_KEY(event.key as u16)
    }
}


#[derive(Copy, Clone, Debug)]
pub struct MouseEvent {
    pub target: MouseEventTarget,
    pub modifiers: ModifierState,
}


#[derive(Debug, Clone)]
pub enum ChildWindowRequest {
    PadEditor,
    ColorEditor,
    FontSelector,
    ChainEditor,
}

#[derive(Copy, Debug, Clone)]
pub enum MouseEventTarget {
    Header,
    Pad(PadId)
}


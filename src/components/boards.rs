use std::rc::Rc;

use windows::Win32::Foundation::{HWND, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::*;

use crate::core::{self, SettingsRepository, SettingsRepositoryMut};
use crate::input::{ModifierState, TextCapture};
use crate::model::{Anchor, Board, BoardHandle, ColorScheme, Pad, PadId, PadSet, Tag, TextStyle};

use super::{BoardComponent, UiEvent, UiEventHandler, UiEventResult, SetWindowPosCommand as Command, Direction, ChildWindowRequest, Tags, state_machine::BoardStateMachine};



#[macro_export]
macro_rules! impl_has_board {
    ($type:ty) => {
        impl<R: SettingsRepository + SettingsRepositoryMut> HasBoard for $type {
            fn board(&self) -> &dyn Board {
                &*self.inner
            }
        }
    };
}


#[macro_export]
macro_rules! impl_board_component {
    ($type:ty) => {
        impl BoardComponent for $type {
            fn data(&self) -> &dyn Board {
                self
            }
            fn handler(&mut self) -> Option<&mut dyn UiEventHandler> {
                Some(self)
            }
        }
    };
}

#[macro_export]
macro_rules! impl_board_component_generic {
    ($type:ty) => {
        impl<R: SettingsRepository + SettingsRepositoryMut + 'static> BoardComponent for $type {
            fn data(&self) -> &dyn Board {
                self
            }
            fn handler(&mut self) -> Option<&mut dyn UiEventHandler> {
                Some(self)
            }
        }
    };
}

/// Delegation traits for Board
pub trait HasBoard {
    fn board(&self) -> &dyn Board;
}

pub trait DelegatingBoard: HasBoard {
    fn delegate_name(&self) -> String {
        self.board().name()
    }
    fn delegate_title(&self) -> String {
        self.board().title()
    }
    fn delegate_icon(&self) -> Option<String> {
        self.board().icon()
    }
    fn delegate_color_scheme(&self) -> ColorScheme {
        self.board().color_scheme()
    }
    fn delegate_text_style(&self) -> TextStyle {
        self.board().text_style()
    }
    fn delegate_padset(&self, modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        self.board().padset(modifier)
    }
    fn delegate_tags(&self, modifier: Option<ModifierState>) -> Vec<Tag> {
        self.board().tags(modifier)
    }
}

impl<T: DelegatingBoard> Board for T {
    fn name(&self) -> String {
        self.delegate_name()
    }
    fn title(&self) -> String {
        self.delegate_title()
    }
    fn icon(&self) -> Option<String> {
        self.delegate_icon()
    }
    fn color_scheme(&self) -> ColorScheme {
        self.delegate_color_scheme()
    }
    fn text_style(&self) -> TextStyle {
        self.delegate_text_style()
    }
    fn padset(&self, modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        self.delegate_padset(modifier)
    }
    fn tags(&self, modifier: Option<ModifierState>) -> Vec<Tag> {
        self.delegate_tags(modifier)
    }
}


pub trait HasHandler {
    fn handler(&mut self) -> Option<&mut dyn UiEventHandler>;
}

pub trait DelegatingHandler: HasHandler {
    fn delegate_handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match self.handler() {
            Some(handler) => handler.handle_ui_event(event),
            None => UiEventResult::NotHandled
        }
    }
    fn delegate_activate(&mut self) -> UiEventResult {
        match self.handler() {
            Some(handler) => handler.activate(),
            None => UiEventResult::NotHandled
        }
    }
    fn delegate_create_child_window(&mut self, request: ChildWindowRequest, parent_hwnd: HWND) -> UiEventResult {
        match self.handler() {
            Some(handler) => handler.create_child_window(request, parent_hwnd),
            None => UiEventResult::NotHandled
        }
    }
    fn delegate_handle_child_result(&mut self, context: Box<dyn std::any::Any>, result: Box<dyn std::any::Any>) -> UiEventResult {
        match self.handler() {
            Some(handler) => handler.handle_child_result(context, result),
            None => UiEventResult::NotHandled
        }
    }
}

impl<T: DelegatingHandler> UiEventHandler for T {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        self.delegate_handle_ui_event(event)
    }
    fn activate(&mut self) -> UiEventResult {
        self.delegate_activate()
    }
    fn create_child_window(&mut self, request: ChildWindowRequest, parent_hwnd: HWND) -> UiEventResult {
        self.delegate_create_child_window(request, parent_hwnd)
    }
    fn handle_child_result(&mut self, context: Box<dyn std::any::Any>, result: Box<dyn std::any::Any>) -> UiEventResult {
        self.delegate_handle_child_result(context, result)
    }
}

/// StateMachineBoard - a BoardComponent implementation delegating to an internal BoardStateMachine

pub struct StateMachineBoard {
    state_machine: BoardStateMachine,
}

impl StateMachineBoard {
    pub fn new(board: Box<dyn BoardComponent>) -> Self {
        Self {
            state_machine: BoardStateMachine::new(board),
        }
    }

    fn main_key_down(&mut self, key: u32) -> UiEventResult {
        let vk_code = VIRTUAL_KEY(key as u16);
        match vk_code {
            VK_ESCAPE | VK_RETURN => {
                // Pop current state if we're not at the root
                if self.state_machine.stack_depth() > 1 {
                    let result = self.state_machine.process_state_result(UiEventResult::PopState {
                        result: Box::new(()),
                    });
                    return self.convert_state_result(result);
                }
            },
            _ => {}
        }
        UiEventResult::NotHandled
    }

    fn convert_state_result(&self, result: UiEventResult) -> UiEventResult {
        match result {
            // Convert state machine operations to UI-friendly results
            UiEventResult::PushState { .. } | UiEventResult::PopState { .. } | UiEventResult::ReplaceState { .. } => {
                UiEventResult::RequiresRedraw
            },
            other => other,
        }
    }
}

impl Board for StateMachineBoard {
    fn name(&self) -> String {
        self.state_machine.current_board_ref().data().name()
    }

    fn title(&self) -> String {
        self.state_machine.current_board_ref().data().title()
    }

    fn icon(&self) -> Option<String> {
        self.state_machine.current_board_ref().data().icon()
    }

    fn color_scheme(&self) -> ColorScheme {
        self.state_machine.current_board_ref().data().color_scheme()
    }

    fn text_style(&self) -> TextStyle {
        self.state_machine.current_board_ref().data().text_style()
    }

    fn padset(&self, modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        self.state_machine.current_board_ref().data().padset(modifier)
    }

    fn tags(&self, modifier: Option<ModifierState>) -> Vec<Tag> {
        self.state_machine.current_board_ref().data().tags(modifier)
    }
}

impl UiEventHandler for StateMachineBoard {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {

        let key_down = match event {
            UiEvent::KeyDown(ke) => Some(ke.key),
            _ => None,
        };

        match event {
            UiEvent::KeyDown(_) | UiEvent::KeyUp(_) | UiEvent::RightMouseDown(_) => {
                let result = self.state_machine.handle_ui_event(event);
                let converted_result = self.convert_state_result(result);

                match converted_result {
                    UiEventResult::NotHandled => {
                        match key_down {
                            Some(key) => self.main_key_down(key),
                            None => UiEventResult::NotHandled,
                        }
                    }
                    other => other,
                }
            }
        }
    }

    fn create_child_window(&mut self, request: ChildWindowRequest, parent_hwnd: HWND) -> UiEventResult {
        // Delegate to the current board in the state machine
        let result = self.state_machine.create_child_window(request, parent_hwnd);
        self.convert_state_result(result)
    }


}

impl_board_component!(StateMachineBoard);


/// SimpleBoard - data only BoardComponent implementation that uses BoardHandle to fetch data from the repository on demand


pub struct SimpleBoard<R: SettingsRepository + SettingsRepositoryMut> {
    repository: Rc<R>,
    board_name: String,
}

impl<R: SettingsRepository + SettingsRepositoryMut> SimpleBoard<R> {
    pub fn new(repository: Rc<R>, board_name: String) -> Self {
        Self {
            repository,
            board_name,
        }
    }

    fn get_handle(&self) -> BoardHandle<R> {
        BoardHandle::new(self.repository.clone(), self.board_name.clone())
    }

    pub fn new_box(repository: Rc<R>, board_name: String) -> Box<SimpleBoard<R>> {
        let simple_board = SimpleBoard::new(repository.clone(), board_name);
        Box::new(simple_board)
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut> Board for SimpleBoard<R> {

    fn name(&self) -> String {
        self.get_handle().name().to_string()
    }

    fn title(&self) -> String {
        self.get_handle().title().unwrap_or_else(|_| self.name().to_string())
    }

    fn icon(&self) -> Option<String> {
        self.get_handle().icon().ok().flatten().map(|s| s.to_string())
    }

    fn color_scheme(&self) -> ColorScheme {
        self.get_handle().color_scheme().unwrap_or_default()
    }

    fn text_style(&self) -> TextStyle {
        self.get_handle().text_style().unwrap_or_default()
    }

    fn padset(&self, modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        if let Ok(padset_handle) = self.get_handle().padset(modifier) {
            return padset_handle.pads().map(|p| Box::new(p) as Box<dyn PadSet>).unwrap_or_else(|_| Box::new(vec![] as Vec<Pad>));
        }
        Box::new(vec![] as Vec<Pad>)
    }

    fn tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        std::vec![]
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> BoardComponent for SimpleBoard<R> {
    fn data(&self) -> &dyn Board {
        self
    }
    fn handler(&mut self) -> Option<&mut dyn UiEventHandler> {
        None
    }
}


/// StringEditorBoard - a simple text editor board component

pub struct StringEditorBoard {
    pub text_capture: TextCapture,
    pub color_scheme: Option<ColorScheme>,
    pub text_style: Option<TextStyle>,
    pub tags: Vec<Tag>,
}



impl Board for StringEditorBoard {
    fn name(&self) -> String {
        "StringEditorBoard".to_string()
    }
    fn title(&self) -> String {
        format!("{}|", self.text_capture.text().unwrap_or_default())
    }
    fn icon(&self) -> Option<String> {
        None
    }
    fn color_scheme(&self) -> ColorScheme {
        self.color_scheme.clone().unwrap_or_default()
    }
    fn text_style(&self) -> TextStyle {
        self.text_style.clone().unwrap_or_default()
    }
    fn padset(&self, _: Option<ModifierState>) -> Box<dyn PadSet> {
        Box::new(vec![

        ] as Vec<Pad>)
    }
    fn tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        self.tags.clone()
    }
}

impl UiEventHandler for StringEditorBoard {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(ke) => {
                let wparam = WPARAM(ke.key as usize);
                let vk_code = VIRTUAL_KEY(ke.key as u16);
                match vk_code {
                    VK_ESCAPE => {
                        return UiEventResult::PopState { result: Box::new(()) };
                    }
                    VK_RETURN => {
                        let final_text = self.text_capture.text();
                        return UiEventResult::PopState { result: Box::new(final_text.unwrap_or_default()) };
                    }
                    _ => {}
                }
                self.text_capture.on_keydown(wparam, ke.modifiers);
                UiEventResult::RequiresRedraw
            }
            UiEvent::KeyUp(ke) => {
                self.text_capture.on_keyup(WPARAM(ke.key as usize), ke.modifiers);
                UiEventResult::RequiresRedraw
            }
            _ => UiEventResult::NotHandled,
        }
    }
}

impl_board_component!(StringEditorBoard);

pub fn string_editor_board(initial_text: String, board: &dyn BoardComponent, tag: String) -> StringEditorBoard {
    let data = board.data();
    StringEditorBoard {
        text_capture: TextCapture::new(Some(initial_text), false),
        color_scheme: Some(data.color_scheme()),
        text_style: Some(data.text_style()),
        tags: vec![
            Tag { text: tag, anchor: Anchor::NW, color_idx: Some(0), ..Default::default() },
            Tags::EscEnter.default(),
        ],
    }
}


/// YesNoBoard - a simple Yes/No confirmation board component
/// Returns true for Yes, false for No
pub struct YesNoBoard {
    pub message: String,
    pub color_scheme: Option<ColorScheme>,
    pub text_style: Option<TextStyle>,
    pub icon: Option<String>,
}

impl YesNoBoard {
    pub fn new(message: String, color_scheme: Option<ColorScheme>, text_style: Option<TextStyle>, icon: Option<String>) -> Self {
        Self {
            message,
            color_scheme,
            text_style,
            icon
        }
    }
}

impl Board for YesNoBoard {
    fn name(&self) -> String {
        "YesNoBoard".to_string()
    }
    fn title(&self) -> String {
        "Confirm".to_string()
    }
    fn icon(&self) -> Option<String> {
        self.icon.clone()
    }
    fn color_scheme(&self) -> ColorScheme {
        self.color_scheme.clone().unwrap_or_default()
    }
    fn text_style(&self) -> TextStyle {
        self.text_style.clone().unwrap_or_default()
    }
    fn padset(&self, _: Option<ModifierState>) -> Box<dyn PadSet> {
        Box::new(vec![
            PadId::Five.with_data(core::Pad {
                text: Some(self.message.clone()),
                ..Default::default()
            }).with_tags(vec![
                Tags::RightBlack.tag(Anchor::W),
                Tags::LeftBlack.tag(Anchor::E),
                Tag { text: "esc(N)".to_string(), anchor: Anchor::NW, font_idx: Some(0), ..Default::default() },
                Tag { text: "enter(Y)".to_string(), anchor: Anchor::NE, font_idx: Some(0), ..Default::default() }
            ])
        ])
    }
    fn tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        vec![]
    }
}

impl UiEventHandler for YesNoBoard {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(key_event) => {
                let vk_code = VIRTUAL_KEY(key_event.key as u16);
                if vk_code == VK_RETURN || vk_code == VK_Y {
                    UiEventResult::PopState { result: Box::new(true) }
                } else if vk_code == VK_ESCAPE || vk_code == VK_N {
                    UiEventResult::PopState { result: Box::new(false) }
                } else {
                    UiEventResult::NotHandled
                }
            }
            UiEvent::RightMouseDown(_) => UiEventResult::Handled, // Ignore mouse clicks here
            _ => UiEventResult::NotHandled,
        }
    }
}

impl_board_component!(YesNoBoard);

pub fn yes_no_question_board(message: String, board: &dyn BoardComponent) -> YesNoBoard {
    YesNoBoard::new(message, Some(board.data().color_scheme()), Some(board.data().text_style()), Some("question.svg".to_string()))
}

pub fn yes_no_warning_board(message: String, board: &dyn BoardComponent) -> YesNoBoard {
    YesNoBoard::new(message, Some(board.data().color_scheme()), Some(board.data().text_style()), Some("warning.svg".to_string()))
}

/// MessageBoard - a simple message display board component
/// Returns () on any key press
pub struct MessageBoard {
    pub title: Option<String>,
    pub message: String,
    pub color_scheme: Option<ColorScheme>,
    pub text_style: Option<TextStyle>,
    pub icon: Option<String>,
}

impl MessageBoard {
    pub fn new(title: Option<String>, message: String, color_scheme: Option<ColorScheme>, text_style: Option<TextStyle>, icon: Option<String>) -> Self {
        Self {
            title,
            message,
            color_scheme,
            text_style,
            icon,
        }
    }
}

impl Board for MessageBoard {
    fn name(&self) -> String {
        "MessageBoard".to_string()
    }
    fn title(&self) -> String {
        self.title.clone().unwrap_or_else(|| "Message".to_string())
    }
    fn icon(&self) -> Option<String> {
        self.icon.clone()
    }
    fn color_scheme(&self) -> ColorScheme {
        self.color_scheme.clone().unwrap_or_default()
    }
    fn text_style(&self) -> TextStyle {
        self.text_style.clone().unwrap_or_default()
    }
    fn padset(&self, _: Option<ModifierState>) -> Box<dyn PadSet> {
        Box::new(vec![
            PadId::Five.with_data(core::Pad {
                text: Some(self.message.clone()),
                ..Default::default()
            }).with_tags(vec![
                Tags::RightBlack.tag(Anchor::W),
                Tags::LeftBlack.tag(Anchor::E)
            ])
        ])
    }
    fn tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        vec![]
    }
}


impl UiEventHandler for MessageBoard {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(_) => {
                UiEventResult::PopState { result: Box::new(()) }
            }
            _ => UiEventResult::NotHandled,
        }
    }
}

impl_board_component!(MessageBoard);

pub fn error_board(message: String, board: &dyn BoardComponent) -> MessageBoard {
    MessageBoard::new(Some("Error".to_string()), message, Some(board.data().color_scheme()), Some(board.data().text_style()), Some("error.svg".to_string()))
}

pub fn success_board(message: String, board: &dyn BoardComponent) -> MessageBoard {
    MessageBoard::new(Some("Success".to_string()), message, Some(board.data().color_scheme()), Some(board.data().text_style()), Some("info.svg".to_string()))
}

/// LayoutBoard - a board for moving/resizing windows with keyboard

pub enum LayoutAction {
    Move,
    Resize,
}
impl Default for LayoutAction {
    fn default() -> Self {
        LayoutAction::Move
    }
}

impl LayoutAction {
    pub fn toggle(&self) -> Self {
        match self {
            LayoutAction::Move => LayoutAction::Resize,
            LayoutAction::Resize => LayoutAction::Move,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            LayoutAction::Move => "Move",
            LayoutAction::Resize => "Resize",
        }
    }
}

pub struct LayoutBoard{
    inner:Box<dyn Board>,
    mode:LayoutAction
}
impl LayoutBoard{
    pub fn new(inner:Box<dyn Board>, mode:LayoutAction)->Self{
        Self{
            inner,
            mode
        }
    }
}

impl HasBoard for LayoutBoard {
    fn board(&self) -> &dyn Board {
        &*self.inner
    }
}

impl LayoutBoard {
    fn key_down(&mut self, key: u32, _modifiers: ModifierState) -> UiEventResult {
        use windows::Win32::UI::Input::KeyboardAndMouse::*;
        let vk_code = VIRTUAL_KEY(key as u16);
        match vk_code {
            VK_LEFT => {
                match self.mode {
                    LayoutAction::Move => UiEventResult::SetWindowPos(Command::Move(Direction::Left)),
                    LayoutAction::Resize => UiEventResult::SetWindowPos(Command::Size(Direction::Left)),
                }
            }
            VK_RIGHT => {
                match self.mode {
                    LayoutAction::Move => UiEventResult::SetWindowPos(Command::Move(Direction::Right)),
                    LayoutAction::Resize => UiEventResult::SetWindowPos(Command::Size(Direction::Right)),
                }
            }
            VK_UP => {
                match self.mode {
                    LayoutAction::Move => UiEventResult::SetWindowPos(Command::Move(Direction::Up)),
                    LayoutAction::Resize => UiEventResult::SetWindowPos(Command::Size(Direction::Up)),
                }
            }
            VK_DOWN => {
                match self.mode {
                    LayoutAction::Move => UiEventResult::SetWindowPos(Command::Move(Direction::Down)),
                    LayoutAction::Resize => UiEventResult::SetWindowPos(Command::Size(Direction::Down)),
                }
            }
            VK_X => {
                self.mode = self.mode.toggle();
                UiEventResult::RequiresRedraw
            }
            VK_ESCAPE | VK_RETURN => {
                UiEventResult::PopState { result: Box::new(()) }
            }
            _ => UiEventResult::NotHandled,
        }
    }
}

impl DelegatingBoard for LayoutBoard {
    fn delegate_tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        let mut tags =vec![
            Tag{ text: format!("{} window", self.mode.as_str()), anchor: Anchor::NW, color_idx: Some(0), ..Default::default() },
            Tag{ text: format!("x: {}, esc/enter", self.mode.toggle().as_str().to_lowercase()), anchor: Anchor::SW, font_idx: Some(1), color_idx: None, ..Default::default() },
        ];
        tags.extend(vec![
            Tag{ text: " △ ".to_string(), anchor: Anchor::NE, font_idx: Some(3), ..Default::default() },
            Tag{ text: "◁ ▷".to_string(), anchor: Anchor::E, font_idx: Some(3), ..Default::default() },
            Tag{ text: " ▽ ".to_string(), anchor: Anchor::SE, font_idx: Some(3), ..Default::default() },
        ]);
        tags
    }
}

impl UiEventHandler for LayoutBoard {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(ke) => {
                self.key_down(ke.key, ke.modifiers)
            }
            UiEvent::RightMouseDown(_) => UiEventResult::Handled, // Ignore mouse clicks here
            _ => UiEventResult::NotHandled,
        }
    }
}


impl_board_component!(LayoutBoard);



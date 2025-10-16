use std::any::Any;
use std::cell::RefCell;
use std::rc::Rc;

use windows::Win32::Foundation::WPARAM;
use windows::Win32::UI::Input::KeyboardAndMouse::*;


use crate::components::PadMapping;
use crate::core::{ActionType, Param, Params, PathString, Resources, SettingsRepository, SettingsRepositoryMut};
use crate::model::{DeleteBoardUseCase, create_modifier_pad_set, delete_modifier_pad_set, Anchor, Board, BoardHandle, ColorScheme, ColorSchemeHandle, Pad, PadId, PadSet, Tag, TextStyle, TextStyleHandle};
use crate::input::{ModifierState, TextCapture, KeyCombinationCapture, capture::{DisplayFormats, DisplayFormatable}};
use crate::{impl_board_component, impl_board_component_generic, impl_has_board};
use crate::ui::dialogs::open_pad_editor;

use super::{
    BoardComponent, ChildWindowRequest, DelegatingBoard, HasBoard, KeyboardEvent, MouseEventTarget, LayoutAction, UiEvent, UiEventHandler, UiEventResult, SimpleBoard, LayoutBoard, SettingsBoard, EnumAll, EnumTraversal, Tags,
    apply_string, error_board, string_editor_board, success_board, yes_no_warning_board, INITIAL_PATH_PARAM
};

enum MainBoardContext {
    DeleteBoard,
    DeleteBoardSuccess,
}

/// MainBoard - main board (C->olorSchemeSelectorBoard, t->TextStyleSelectorBoard)
pub struct MainBoard<R: SettingsRepository + SettingsRepositoryMut> {
    inner: Box<dyn Board>,
    params: Vec<Param>,
    resources: Resources,
    repository: Rc<R>
}


impl_has_board!(MainBoard<R>);

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> MainBoard<R> {
    pub fn new(board_name: String, params: Vec<Param>, resources: Resources, repository: Rc<R>) -> Self {
        Self {
            inner: SimpleBoard::new_box(repository.clone(), board_name),
            params,
            resources,
            repository
        }
    }

    fn create_simple_board(&self) -> Box<SimpleBoard<R>> {
        SimpleBoard::new_box(self.repository.clone(), self.name())
    }

    fn request_edit_mode(&mut self, params: Vec<Param>) -> UiEventResult {
        let edit_board = Box::new(EditModeBoard::new(
            self.create_simple_board(),
            self.repository.clone(),
            params
        ));
        UiEventResult::PushState {
            board: edit_board,
            context: Box::new(()),
        }
    }

    fn request_settings_board(&self) -> UiEventResult {

        let mut board = self.repository.get_board("settings")
            .expect("Settings board should exist");
        board.color_scheme = Some(self.inner.color_scheme().name);
        board.text_style = Some(self.inner.text_style().name);

        let settings_board = Box::new(SettingsBoard::new(
            board,
            vec![],
            self.resources.clone(),
            self.repository.clone()
        ));
        UiEventResult::PushState {
            board: settings_board,
            context: Box::new(()),
        }

    }

    fn request_move_or_size(&self) -> UiEventResult {
        let move_or_size_board = Box::new(LayoutBoard::new(
            self.create_simple_board(),
            LayoutAction::Move
        ));
        UiEventResult::PushState {
            board: move_or_size_board,
            context: Box::new(()),
        }
    }

    fn request_confirm_delete(&self) -> UiEventResult {
        UiEventResult::PushState {
            board: Box::new(
                yes_no_warning_board(
                    format!("Delete board \"{}\"?", self.name()),
                    self
                )
            ),
            context: Box::new(MainBoardContext::DeleteBoard),
        }
    }

    fn key_down(&mut self, ke: KeyboardEvent) -> UiEventResult {
        let vk_code = VIRTUAL_KEY(ke.key as u16);
        match vk_code {
            VK_E => {
                self.request_edit_mode(vec![])
            },
            VK_S => {
                self.request_settings_board()
            }
            VK_X => {
                self.request_move_or_size()
            }
            VK_D => {
                self.request_confirm_delete()
            },
            VK_W => {
                if self.repository.is_dirty() && self.repository.flush().is_ok() {
                    return UiEventResult::RequiresRedraw
                }
                UiEventResult::NotHandled
            }
            _ => UiEventResult::NotHandled
        }
    }

    fn key_up(&mut self, _ke: KeyboardEvent) -> UiEventResult {
        UiEventResult::RequiresRedraw
    }

    fn uc(&self) -> DeleteBoardUseCase<R> {
        DeleteBoardUseCase::new(self.repository.clone(), self.name().to_string())
    }

    fn get_initial_path(&self) -> Option<Param> {
        self.params.iter()
            .find(|p| p.name == INITIAL_PATH_PARAM)
            .cloned()
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut> DelegatingBoard for MainBoard<R> {

    fn delegate_tags(&self, modifier: Option<ModifierState>) -> Vec<Tag> {
        let mut tags = Vec::<Tag>::new();

        if let Some(modifier) = modifier.filter(|m| m.is_any()) {
            tags.push(
                Tag { text: modifier.to_string(), anchor: Anchor::SE, font_idx: Some(0), ..Default::default() }
            );

            tags.push(Tag { text: "e: edit    x: layout\nd: delete  s: settings".to_string(), anchor: Anchor::SW, font_idx: Some(0), ..Default::default() });
            if self.repository.is_dirty() {
                tags.push(Tag { text: "w: save".to_string(), anchor: Anchor::NE, font_idx: Some(0), ..Default::default() });
            }
            return tags;
        }

        if self.repository.is_dirty() {
            tags.push(Tag { text: "(*)".to_string(), anchor: Anchor::NE, ..Default::default() });
        }

        tags
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for MainBoard<R> {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(ke) => self.key_down(ke),
            UiEvent::KeyUp(ke) => self.key_up(ke),
            UiEvent::RightMouseDown(me) => {
                match me.target {
                    MouseEventTarget::Header => self.request_edit_mode(vec![]),
                    _ => UiEventResult::NotHandled,
                }
            }
        }
    }

    fn handle_child_result(&mut self, context: Box<dyn Any>, result: Box<dyn Any>) -> UiEventResult {
        let operation = match context.downcast_ref::<MainBoardContext>() {
            Some(c) => c,
            None => return UiEventResult::NotHandled,
        };
        match operation {
            MainBoardContext::DeleteBoard => {
                if let Some(confirmed) = result.downcast_ref::<bool>() {
                    if *confirmed {
                        match self.uc().delete() {
                            Ok(()) => {
                                return UiEventResult::PushState {
                                    board: Box::new(success_board(format!("Deleted board:\n\n\"{}\"", self.name()),self)),
                                    context: Box::new(MainBoardContext::DeleteBoardSuccess),
                                }
                            }
                            Err(e) => {
                                return UiEventResult::PushState {
                                    board: Box::new(error_board(format!("{}", e), self)),
                                    context: Box::new(()),
                                }
                            }
                        }
                    }
                }
                UiEventResult::NotHandled
            },
            MainBoardContext::DeleteBoardSuccess => {
                UiEventResult::CloseWindow
            }
        }
    }

    fn activate(&mut self) -> UiEventResult {
        let initial_path = self.get_initial_path();
        if let Some(param) = initial_path {
            let path: Vec<String> = param.value.path();

            if let Some(first) = path.get(0) {
                if first == "edit" {
                    return self.request_edit_mode(
                        self.params.merge_params(vec![
                            param.with_sub_value(first.clone()),
                        ])
                    );
                }
            }
        }
        UiEventResult::NotHandled
    }
}

impl_board_component_generic!(MainBoard<R>);

macro_rules! impl_selector_board {
    ($board_type:ident, $item_type:ty) => {
        pub struct $board_type<R: SettingsRepository + SettingsRepositoryMut> {
            inner: Box<dyn Board>,
            item: RefCell<Option<$item_type>>,
            repository: Rc<R>,
        }

        impl<R: SettingsRepository + SettingsRepositoryMut> $board_type<R> {
            pub fn new(inner: Box<dyn Board>, repository: Rc<R>) -> Self {
                Self {
                    inner,
                    item: RefCell::new(None),
                    repository,
                }
            }

            fn get_tags(&self, text: &str) -> Vec<Tag> {
                vec![
                    Tags::LeftRight.default(),
                    Tags::EscEnter.default(),
                    Tag{ text: text.to_string(), anchor: Anchor::NW, ..Default::default() },
                ]
            }
        }


        impl<R: SettingsRepository + SettingsRepositoryMut + 'static> $board_type<R> {
            fn key_down(&mut self, key: u32, _: ModifierState) -> UiEventResult {
                let vk_code = VIRTUAL_KEY(key as u16);
                match vk_code {
                    VK_LEFT | VK_RIGHT => {
                        let mut handle = self.get_handle();

                        if vk_code == VK_RIGHT {
                            handle.move_next();
                        } else {
                            handle.move_prev();
                        }

                        self.item.replace(Some(handle.as_data().unwrap()));
                        UiEventResult::RequiresRedraw
                    }
                    VK_RETURN => {
                        let item_name = self
                            .item
                            .borrow()
                            .as_ref()
                            .map(|cs| cs.name.clone());

                        self.apply_selection(item_name);

                        // Pop state with completion signal
                        UiEventResult::PopState {
                            result: Box::new(()),
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
            }
        }


        impl<R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for $board_type<R> {
            fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
                match event {
                    UiEvent::KeyDown(ke) => {
                        self.key_down(ke.key, ke.modifiers)
                    }
                    _ => UiEventResult::NotHandled,
                }
            }
        }


        impl<R: SettingsRepository + SettingsRepositoryMut + 'static> BoardComponent for $board_type<R> {
            fn data(&self) -> &dyn Board {
                self
            }
            fn handler(&mut self) -> Option<&mut dyn UiEventHandler> {
                Some(self)
            }
        }
    };
}

impl_selector_board!(ColorSchemeSelectorBoard, ColorScheme);

impl_has_board!(ColorSchemeSelectorBoard<R>);

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> ColorSchemeSelectorBoard<R> {
    fn get_handle(&self) -> ColorSchemeHandle<R> {
        ColorSchemeHandle::<R>::new(
            self.repository.clone(),
            Some(self.delegate_color_scheme().name),
        )
    }

    fn apply_selection(&mut self, scheme_name: Option<String>) {
        // Save the change directly to the repository
        BoardHandle::<R>::new(self.repository.clone(), self.name())
            .set_color_scheme(scheme_name)
            .unwrap_or_else(|e| log::error!("Failed to update color scheme: {}", e));
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingBoard for ColorSchemeSelectorBoard<R> {
    fn delegate_title(&self) -> String {
        self.delegate_color_scheme().name
    }
    fn delegate_color_scheme(&self) -> ColorScheme {
        self.item.borrow().clone().unwrap_or_else(|| self.board().color_scheme())
    }

    fn delegate_tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        self.get_tags("Colors")
    }
}


impl_selector_board!(TextStyleSelectorBoard, TextStyle);

impl_has_board!(TextStyleSelectorBoard<R>);

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> TextStyleSelectorBoard<R> {
    fn get_handle(&self) -> TextStyleHandle<R> {
        TextStyleHandle::<R>::new(
            self.repository.clone(),
            Some(self.delegate_text_style().name),
        )
    }
    fn apply_selection(&mut self, style_name: Option<String>) {
        // Save the change directly to the repository
        BoardHandle::<R>::new(self.repository.clone(), self.name())
            .set_text_style(style_name)
            .unwrap_or_else(|e| log::error!("Failed to update text style: {}", e));
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingBoard for TextStyleSelectorBoard<R> {
    fn delegate_title(&self) -> String {
        self.delegate_text_style().name
    }
    fn delegate_text_style(&self) -> TextStyle {
        self.item.borrow().clone().unwrap_or_else(|| self.board().text_style())
    }

    fn delegate_tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        self.get_tags("Fonts")
    }
}




enum EditOperation {
    TitleEdit,
    DeletePadSet(ModifierState),
}

pub struct EditModeBoard<R: SettingsRepository + SettingsRepositoryMut> {
    inner: Box<dyn Board>,
    repository: Rc<R>,
    params: Vec<Param>
}

impl_has_board!(EditModeBoard<R>);

impl <R: SettingsRepository + SettingsRepositoryMut + 'static> EditModeBoard<R> {
    pub fn new(inner: Box<dyn Board>, repository: Rc<R>, params: Vec<Param>) -> Self {
        Self { inner, repository, params }
    }

    fn create_simple_board(&self) -> Box<SimpleBoard<R>> {
        SimpleBoard::new_box(self.repository.clone(), self.name())
    }

    fn request_title_editor(&mut self) -> UiEventResult {
        let board_box = Box::new(string_editor_board(
            self.title(),self, "Title".to_string()
        ));
        UiEventResult::PushState {
            board: board_box,
            context: Box::new(EditOperation::TitleEdit),
        }
    }

    fn request_color_scheme_selector(&self) -> UiEventResult {
        let selector_board = Box::new(ColorSchemeSelectorBoard::new(
            self.create_simple_board(),
            self.repository.clone()
        ));
        UiEventResult::PushState {
            board: selector_board,
            context: Box::new(()),
        }
    }

    fn request_text_style_selector(&self) -> UiEventResult {
        let selector_board = Box::new(TextStyleSelectorBoard::new(
            self.create_simple_board(),
            self.repository.clone()
        ));
        UiEventResult::PushState {
            board: selector_board,
            context: Box::new(()),
        }
    }

    fn request_pad_editor(&mut self, modifiers: ModifierState, pad_id: PadId) -> UiEventResult {
        let pad_editor_board = Box::new(PadEditorBoard::new(
            self.create_simple_board(),
            pad_id,
            modifiers,
            self.repository.clone()
        ));
        UiEventResult::PushState {
            board: pad_editor_board,
            context: Box::new(()),
        }
    }

    fn request_delete_modifier(&mut self, modifier: ModifierState) -> UiEventResult {
        UiEventResult::PushState {
            board: Box::new(yes_no_warning_board(format!("Delete padset?\n\n\"{}\"", modifier), self)),
            context: Box::new(EditOperation::DeletePadSet(modifier)),
        }
    }

    fn can_create_modifier_padset(&self, modifier: ModifierState) -> bool {
        if !modifier.is_any() {
            return false;
        }
        let board = self.repository.get_board(self.name().as_str()).unwrap();
        if !board.has_modifier(modifier.to_string().as_str()) {
            return true;
        }
        false
    }

    fn can_delete_modifier_padset(&self, modifier: ModifierState) -> bool {
        if !modifier.is_any() {
            return false;
        }
        let board = self.repository.get_board(self.name().as_str()).unwrap();
        if board.has_modifier(modifier.to_string().as_str()) {
            return true;
        }
        false
    }

    fn create_modifier_padset(&self, modifier: ModifierState) -> UiEventResult {
        UiEventResult::PushState {
            board: Box::new(
                match create_modifier_pad_set(self.repository.as_ref(), self.name(), modifier) {
                    Ok(padset) => success_board(format!("Created padset:\n\n\"{}\"", padset.name), self),
                    Err(e) => error_board(format!("{}", e), self)
                }
            ),
            context: Box::new(()),
        }
    }

    fn delete_modifier_padset(&self, modifier: ModifierState) -> UiEventResult {
        UiEventResult::PushState {
            board: Box::new(
                match delete_modifier_pad_set(self.repository.as_ref(), self.name(), modifier.to_string()) {
                    Ok(()) => success_board(format!("Deleted padset:\n\n\"{}\"", modifier), self),
                    Err(e) => error_board(format!("{}", e), self)
                }
            ),
            context: Box::new(()),
        }
    }

    fn key_down(&mut self, key: u32, modifiers: ModifierState) -> UiEventResult {
        use windows::Win32::UI::Input::KeyboardAndMouse::*;

        let vk_code = VIRTUAL_KEY(key as u16);

        // Handle 'f2' key for title editing
        if vk_code == VK_F2 {
            return self.request_title_editor()
        }

        // Handle 'c' key for color scheme selector
        if vk_code == VK_C {
            return self.request_color_scheme_selector()
        }

        // Handle 'f' key for text style selector
        if vk_code == VK_F || vk_code == VK_T {
            return self.request_text_style_selector()
        }

        if vk_code == VK_OEM_MINUS && self.can_delete_modifier_padset(modifiers) {
            return self.request_delete_modifier(modifiers);
        }

        if vk_code == VK_OEM_PLUS && self.can_create_modifier_padset(modifiers) {
            return self.create_modifier_padset(modifiers);
        }

        // Handle numpad keys 1-9 for pad editing
        let pad_id = match self.pad_mapping().map(vk_code) {
            Some(pad_id) => pad_id,
            _ => return UiEventResult::NotHandled,
        };

        self.request_pad_editor(modifiers, pad_id)
    }

    fn key_up(&mut self, _key: u32, _modifiers: ModifierState) -> UiEventResult {
        return UiEventResult::RequiresRedraw
    }

    fn right_mouse_down(&mut self, target: MouseEventTarget, modifiers: ModifierState) -> UiEventResult {
        match target {
            MouseEventTarget::Header => {
                return self.request_title_editor();
            }
            MouseEventTarget::Pad(pad_id) => {
                return self.request_pad_editor(modifiers, pad_id);
            }
        }
    }

    fn get_initial_path(&self) -> Option<Param> {
        self.params.iter()
            .find(|p| p.name == INITIAL_PATH_PARAM)
            .cloned()
    }

    fn pad_mapping(&self) -> PadMapping<R> {
        PadMapping { repository: self.repository.clone() }
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingBoard for EditModeBoard<R> {
    fn delegate_tags(&self, modifier: Option<ModifierState>) -> Vec<Tag> {
        let mut tags = vec![
            Tag { text: "Editing".to_string(), anchor: Anchor::NW, font_idx: None, color_idx: Some(0), ..Default::default() },
            Tags::EscEnter.default(),
            Tag { text: "c: colors, f: fonts".to_string(), anchor: Anchor::SE, font_idx: Some(0), ..Default::default() },
            Tag { text: "1-9: pad, F2: rename".to_string(), anchor: Anchor::SW, font_idx: Some(0), ..Default::default() }
        ];

        let modifier = modifier.unwrap_or_default();
        if modifier.is_none() {
            return tags;
        }

        let modifier_cmd = match self.can_create_modifier_padset(modifier) {
            true => format!("+: add ({})", modifier.to_string()),
            false => format!("-: delete ({})", modifier.to_string()),
        };
        // Replace the SW tag with the modifier command
        tags.pop();
        tags.push(Tag { text: modifier_cmd, anchor: Anchor::SW, font_idx: Some(0), ..Default::default() });
        tags
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for EditModeBoard<R> {

    fn activate(&mut self) -> UiEventResult {
        let initial_path = self.get_initial_path();
        if let Some(param) = initial_path {
            let path: Vec<String> = param.value.path();

            log::info!("Activating edit mode with initial path: {:?}", path);

            if let Some(first) = path.get(0) {
                match first.as_str() {
                    "title" => {
                        return self.request_title_editor();
                    }
                    "colors" => {
                        return self.request_color_scheme_selector();
                    }
                    "fonts" => {
                        return self.request_text_style_selector();
                    }
                    other if other.starts_with("pad") => {
                        if let Some(pad_num_str) = other.strip_prefix("pad") {
                            if let Ok(pad_num) = pad_num_str.parse::<i32>() {
                                if (1..=9).contains(&pad_num) {
                                    return self.request_pad_editor(ModifierState::default(), PadId::from_keypad_int(pad_num));
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        UiEventResult::NotHandled
    }

    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(ke) => {
                self.key_down(ke.key, ke.modifiers)
            }
            UiEvent::KeyUp(ke) => {
                self.key_up(ke.key, ke.modifiers)
            }
            UiEvent::RightMouseDown(me) => {
                self.right_mouse_down(me.target, me.modifiers)
            }
        }
    }

    fn handle_child_result(&mut self, context: Box<dyn Any>, result: Box<dyn Any>) -> UiEventResult {
        let operation = match context.downcast_ref::<EditOperation>() {
            Some(c) => c,
            None => return UiEventResult::NotHandled,
        };
        match operation {
            EditOperation::TitleEdit => {
                apply_string(result, |title| self.update_board_title(title.to_string()))
            }
            EditOperation::DeletePadSet(modifier) => {
                if let Some(confirmed) = result.downcast_ref::<bool>() {
                    if *confirmed {
                        return self.delete_modifier_padset(*modifier)
                    }
                }
                UiEventResult::NotHandled
            }
        }
    }
}

impl_board_component_generic!(EditModeBoard<R>);

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> EditModeBoard<R> {
    fn update_board_title(&mut self, new_title: String) -> Result<(), Box<dyn std::error::Error>> {
        let board_handle = BoardHandle::new(self.repository.clone(), self.name());
        board_handle.set_title(Some(new_title))?;
        Ok(())
    }
}



enum PadEditOperation {
    ShortcutEdit,
}

#[derive(PartialEq, Eq, Clone)]
enum PadEditorMode {
    Header,
    Text,
    Action,
    Board,
}

impl EnumAll<PadEditorMode> for PadEditorMode {
    fn all() -> Vec<PadEditorMode> {
        vec![
            PadEditorMode::Header,
            PadEditorMode::Text,
            PadEditorMode::Action,
            PadEditorMode::Board,
        ]
    }
}



impl Default for PadEditorMode {
    fn default() -> Self {
        PadEditorMode::Text
    }
}

pub struct PadEditorBoard<R:SettingsRepository+SettingsRepositoryMut>{
    inner:Box<dyn Board>,
    pad_id:PadId,
    modifier_state:ModifierState,
    edit:PadEditorMode,
    item:RefCell<Option<Pad>>,
    repository:Rc<R>,
}
impl <R:SettingsRepository+SettingsRepositoryMut> PadEditorBoard<R>{
    pub fn new(inner:Box<dyn Board>,pad_id:PadId,modifier_state:ModifierState,repository:Rc<R>)->Self{
        Self{
            inner,
            pad_id,
            modifier_state,
            edit: PadEditorMode::default(),
            item:RefCell::new(None),
            repository,
        }
    }
}

impl_has_board!(PadEditorBoard<R>);

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> PadEditorBoard<R> {
    fn get_pad(&self) -> Pad {
        if let Some(pad) = self.item.borrow().as_ref() {
            return pad.clone();
        }
        let padset = self.inner.padset(Some(self.modifier_state.clone()));
        self.set_pad(padset.pad(self.pad_id));
        self.get_pad()
    }

    fn set_pad(&self, pad: Pad) {
        self.item.replace(Some(pad));
    }

    fn request_pad_editor(&mut self) -> UiEventResult {
        UiEventResult::RequestChildWindow(ChildWindowRequest::PadEditor)
    }

    fn request_shortcut_editor(&mut self) -> UiEventResult {
        let mut text_style = self.repository.resolve_text_style(&None);
        text_style.header_font = "Consolas 30".to_string();
        text_style.pad_text_font = "Arial Italic 20".to_string();

        let board_box = Box::new(ShortcutEditorBoard {
            capture: KeyCombinationCapture::new(),
            text_style: Some(text_style),
            color_scheme: Some(self.color_scheme()),
            is_finished: false,
        });
        UiEventResult::PushState {
            board: board_box,
            context: Box::new(PadEditOperation::ShortcutEdit),
        }
    }

    #[allow(dead_code)]
    fn request_default_editor(&mut self) -> UiEventResult {
        fn is_shortcut_assumed(pad: &Pad) -> bool {
            if pad.actions().is_empty() {
                return true;
            } else if pad.actions().len() == 1 {
                if let ActionType::Shortcut(_) = &pad.actions()[0] {
                    return true;
                }
            }
            false
        }

        if is_shortcut_assumed(&self.get_pad()) {
            self.request_shortcut_editor()
        } else {
            self.request_pad_editor()
        }
    }

    fn set_first_action_shortcut(&self, value: String) -> Result<(), Box<dyn std::error::Error>> {
        let value = value.trim().to_string();
        if value.is_empty() {
            return Ok(());
        }
        let mut pad = self.get_pad();
        pad.data.actions = vec![ActionType::Shortcut(value.clone())];
        pad.data.header = Some(value);
        self.set_pad(pad);
        Ok(())
    }

}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> DelegatingBoard for PadEditorBoard<R> {
    fn delegate_title(&self) -> String {
        match self.edit {
            PadEditorMode::Header => "Pad header".to_string(),
            PadEditorMode::Text => "Pad text".to_string(),
            PadEditorMode::Action => "Pad actions".to_string(),
            PadEditorMode::Board => "Navigation".to_string(),
        }
    }
    fn delegate_padset(&self, _modifier: Option<ModifierState>) -> Box<dyn PadSet> {
        let mut pad = self.get_pad();

        match self.edit {
            PadEditorMode::Header => pad.data.header = Some(pad.data.header.clone().unwrap_or_default() + "|"),
            PadEditorMode::Text => pad.data.text = Some(pad.data.text.clone().unwrap_or_default() + "|"),
            PadEditorMode::Action => {},
            PadEditorMode::Board => {},
        }

        let cnt_actions = pad.actions().len();
        let action_str = match cnt_actions {
            0 => "-".to_string(),
            1 => "1 action".to_string(),
            n => format!("{} actions", n),
        };

        let (anchor, tag_text) = match self.edit {
            PadEditorMode::Header => (Anchor::NW, Tags::RightBlack.to_string()),
            PadEditorMode::Text => (Anchor::W, Tags::RightBlack.to_string()),
            PadEditorMode::Action => (Anchor::SW, format!("{} {}", Tags::RightBlack.to_string(), action_str)),
            PadEditorMode::Board => (Anchor::SW, format!("{} {}", Tags::RightBlack.to_string(), pad.board().unwrap_or("-".to_string()))),
        };
        pad.tags.extend(vec![Tag{ text: tag_text, anchor, color_idx: Some(0), ..Default::default() }]);

        Box::new(vec![pad])
    }
    fn delegate_tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        let mut tags = vec![
            Tag{ text: format!("Pad {}", self.pad_id.to_string()), anchor: Anchor::NW, font_idx: None, color_idx: Some(0), ..Default::default() },
            Tags::EscEnter.default()
        ];
        if self.edit == PadEditorMode::Action || self.edit == PadEditorMode::Board {
            tags.push(Tag{ text: "c: clear pad, e: edit, s: shortcut".to_string(), anchor: Anchor::SW, font_idx: Some(1), color_idx: None, ..Default::default() });
            tags.push(Tag{ text: "▷   ".to_string(), anchor: Anchor::SE, font_idx: Some(2), color_idx: Some(0), ..Default::default() });
        }
        tags.push(Tags::DownUp.default());

        tags
    }
}

impl<R: SettingsRepository + SettingsRepositoryMut + 'static> UiEventHandler for PadEditorBoard<R> {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(ke) => {
                let vk_code = VIRTUAL_KEY(ke.key as u16);
                match vk_code {
                    VK_UP => {
                        self.edit = self.edit.previous();
                        UiEventResult::RequiresRedraw
                    }
                    VK_DOWN => {
                        self.edit = self.edit.next();
                        UiEventResult::RequiresRedraw
                    }
                    VK_ESCAPE => {
                        UiEventResult::PopState { result: Box::new(()) }
                    }
                    other => {
                        if other == VK_RETURN && ke.modifiers.is_none() {
                            if let Some(pad) = self.item.borrow().as_ref() {
                                // Save the updated pad
                                let board_handle = BoardHandle::<R>::new(self.repository.clone(), self.name());
                                if let Ok(padset_handle) = board_handle.padset(Some(self.modifier_state.clone())) {
                                    if let Err(e) = padset_handle.set_pad(pad.clone()) {
                                        log::error!("Failed to update pad: {}", e);
                                    }
                                }
                                return UiEventResult::PopState { result: Box::new(()) };
                            }
                            return UiEventResult::NotHandled
                        }

                        if self.edit == PadEditorMode::Action || self.edit == PadEditorMode::Board {
                            return match other {
                                VK_E => self.request_pad_editor(),
                                VK_C => {
                                    let mut pad = self.get_pad();
                                    pad.data = Default::default();
                                    self.set_pad(pad);
                                    UiEventResult::RequiresRedraw
                                },
                                VK_S => self.request_shortcut_editor(),
                                VK_RIGHT => if self.edit == PadEditorMode::Action {
                                    self.request_default_editor()
                                } else {
                                    self.request_pad_editor()
                                },
                                _ => UiEventResult::NotHandled
                            }
                        }


                        let mut pad = self.get_pad();
                        let mut text_capture = TextCapture::new(match self.edit {
                            PadEditorMode::Header => pad.data.header.clone(),
                            PadEditorMode::Text => pad.data.text.clone(),
                            _  => unreachable!()
                        }, true);
                        text_capture.on_keydown(WPARAM(ke.key as usize), ke.modifiers);
                        let final_text = text_capture.text();
                        match self.edit {
                            PadEditorMode::Header => pad.data.header = final_text,
                            PadEditorMode::Text => pad.data.text = final_text,
                            _ => unreachable!(),
                        }

                        self.set_pad(pad);
                        UiEventResult::RequiresRedraw
                    },
                }
            }
            _ => UiEventResult::NotHandled,
        }
    }

    fn create_child_window(&mut self, request: ChildWindowRequest, parent_hwnd: windows::Win32::Foundation::HWND) -> UiEventResult {
        match request {
            ChildWindowRequest::PadEditor => {
                if let Some(pad) = open_pad_editor(self.get_pad(), Some(parent_hwnd), self.repository.boards(), self.edit == PadEditorMode::Board) {
                    self.set_pad(pad);
                    UiEventResult::RequiresRedraw
                } else {
                    UiEventResult::NotHandled
                }
            }
            _ => UiEventResult::NotHandled,
        }
    }

    fn handle_child_result(&mut self, context: Box<dyn Any>, result: Box<dyn Any>) -> UiEventResult {
        let operation = match context.downcast_ref::<PadEditOperation>() {
            Some(c) => c,
            None => return UiEventResult::NotHandled,
        };
        match operation {
            PadEditOperation::ShortcutEdit => {
                apply_string(result, |title| self.set_first_action_shortcut(title.to_string()))
            }
        }
    }
}

impl_board_component_generic!(PadEditorBoard<R>);



struct ShortcutEditorBoard {
    capture: KeyCombinationCapture,
    color_scheme: Option<ColorScheme>,
    text_style: Option<TextStyle>,
    is_finished: bool,
}

impl Board for ShortcutEditorBoard {
    fn name(&self) -> String {
        "ShortcutEditorBoard".to_string()
    }
    fn title(&self) -> String {
        let display_format = DisplayFormats::InverseSpaced;
        let current_capture = self.capture.get_current_capture();
        let display_text = current_capture.display_format(&display_format.get_format());
        format!("{}|", display_text)
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
        Box::new(
            if self.is_finished { vec![] }
            else { vec![
                PadId::Five.with(|p| {
                    p.data.text = Some("Enter shortcut and press\n'Esc' to finish".to_string());
                })]
            }
        )
    }
    fn tags(&self, _modifier: Option<ModifierState>) -> Vec<Tag> {
        let mut tags = vec![
            Tag { text: "Shortcut".to_string(), anchor: Anchor::NW, color_idx: Some(0), ..Default::default() },
        ];
        if !self.is_finished {
            tags.push(Tags::EscEnter.default());
        } else {
            tags.push(Tag { text: "Cancel (esc)".to_string(), anchor: Anchor::SW, ..Default::default()});
            tags.push(Tag { text: "Confirm (enter)".to_string(), anchor: Anchor::SE, ..Default::default()});
        }
        tags
    }
}

impl UiEventHandler for ShortcutEditorBoard {
    fn handle_ui_event(&mut self, event: UiEvent) -> UiEventResult {
        match event {
            UiEvent::KeyDown(ke) => {
                let vk_code = VIRTUAL_KEY(ke.key as u16);
                if self.is_finished {
                    if vk_code == VK_RETURN {
                        let final_capture = self.capture.get_current_capture();

                        if final_capture.is_empty() {
                            return UiEventResult::PopState { result: Box::new(String::new()) };
                        } else {
                            let display_format = DisplayFormats::InverseSpaced;
                            let display_text = final_capture.display_format(&display_format.get_format());
                            return UiEventResult::PopState { result: Box::new(display_text) };
                        }

                    }
                    if vk_code == VK_ESCAPE {
                        return UiEventResult::PopState { result: Box::new(()) };
                    }
                    return UiEventResult::NotHandled;
                }

                if vk_code == VK_ESCAPE && ke.modifiers.is_none() {
                    self.is_finished = true;
                    return UiEventResult::RequiresRedraw;
                }

                let wparam = WPARAM(ke.key as usize);
                self.capture.on_keydown(wparam, ke.modifiers);
                UiEventResult::RequiresRedraw
            }
            UiEvent::KeyUp(ke) => {
                let wparam = WPARAM(ke.key as usize);
                self.capture.on_keyup(wparam, ke.modifiers);
                UiEventResult::Handled
            }
            _ => UiEventResult::NotHandled,
        }
    }
}

impl_board_component!(ShortcutEditorBoard);
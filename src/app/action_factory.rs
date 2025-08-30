use std::{collections::HashMap, rc::Rc};

use clipboard_win::{Clipboard, Setter, Unicode};

use crate::core::{ActionType, ActionParams, SettingsRepository, SettingsRepositoryMut};
use crate::input::{script, script::InputScript};

#[derive(Debug, Clone, PartialEq)]
pub enum ActionResult {
    Success,
    Error(String),
}

pub struct ActionRuntimeContext<R: SettingsRepository + SettingsRepositoryMut> {
    #[allow(dead_code)]
    pub repository: Rc<R>,
}

pub trait Action {
    fn run(&self) -> ActionResult;
    fn requires_reload(&self) -> bool { false }
    fn requires_restart(&self) -> bool { false }
}

pub trait ActionFactory<R: SettingsRepository + SettingsRepositoryMut> {
    fn create_action(&self, context: &ActionRuntimeContext<R>, action: &ActionParams) -> Option<Box<dyn Action>>;
}

pub struct ActionFactoryRegistry<R: SettingsRepository + SettingsRepositoryMut> {
    factories: HashMap<String, Box<dyn ActionFactory<R>>>,
}

impl<R: SettingsRepository + SettingsRepositoryMut> ActionFactoryRegistry<R> {
    pub fn new() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn register_factory(&mut self, name: &str, factory: Box<dyn ActionFactory<R>>) {
        self.factories.insert(name.to_string(), factory);
    }

    pub fn get_factory(&self, name: &str) -> Option<&Box<dyn ActionFactory<R>>> {
        self.factories.get(name)
    }
}



pub struct ActionFactoryImpl<'a, R: SettingsRepository + SettingsRepositoryMut> {
    repository: Rc<R>,
    registry: &'a ActionFactoryRegistry<R>,
}

impl<'a, R: SettingsRepository + SettingsRepositoryMut> ActionFactoryImpl<'a, R> {
    pub fn new(repository: Rc<R>, registry: &'a ActionFactoryRegistry<R>) -> Self {
        Self { repository, registry }
    }

    // fn keyboard_mappings(&self) -> HashMap<String, String> {
    //     let layout = self.repository.get_keyboard_layout();
    //     layout.mappings.clone()
    // }

    fn runtime_context(&self) -> ActionRuntimeContext<R> {
        ActionRuntimeContext { repository: self.repository.clone() }
    }

    pub fn create_action(&self, action_type: &ActionType) -> Box<dyn Action + '_> {
        match action_type {
            ActionType::Shortcut(text) => {
                let script = script::for_shortcut(text.clone());
                Box::new(InputScriptAction { script })
            },
            ActionType::Text(text) => {
                let script = script::for_text(text.clone());
                Box::new(InputScriptAction { script })
            },
            ActionType::Line(text) => {
                let script = script::for_line(text.clone());
                Box::new(InputScriptAction { script })
            },
            ActionType::Paste(text) => {
                Box::new(PasteAction { text: text.clone(), enter: false })
            },
            ActionType::PasteEnter(text) => {
                Box::new(PasteAction { text: text.clone(), enter: true })
            },
            ActionType::Pause(duration) => {
                let script = script::for_pause(*duration);
                Box::new(InputScriptAction { script })
            },
            ActionType::OpenUrl(url) => {
                Box::new(OpenUrlAction { url: url.clone() })
            },
            ActionType::Custom(custom_action) => {
                self.registry
                    .get_factory(&custom_action.action_type)
                    .and_then(|factory| factory.create_action(&self.runtime_context(), custom_action))
                    .unwrap_or_else(|| Box::new(NoOpAction))
            }
        }
    }
}

// Actions runners
struct NoOpAction;
impl Action for NoOpAction {
    fn run(&self) -> ActionResult {
        ActionResult::Success
    }
}


struct InputScriptAction {
    script: InputScript,
}

impl Action for InputScriptAction {
    fn run(&self) -> ActionResult {
        self.script.play();
        ActionResult::Success
    }
}

struct OpenUrlAction {
    url: String,
}

impl Action for OpenUrlAction {
    fn run(&self) -> ActionResult {
        match open::that(&self.url) {
            Ok(()) => ActionResult::Success,
            Err(e) => {
                log::error!("Failed to open URL: {}", e);
                ActionResult::Error(format!("Failed to open URL: {}", e))
            }
        }
    }
}

struct PasteAction {
    text: String,
    enter: bool,
}

impl Action for PasteAction {
    fn run(&self) -> ActionResult {
        let text = self.text.as_str();
        if let Ok(_clip) = Clipboard::new_attempts(10) {
            match Unicode.write_clipboard(&text) {
                Err(e) => {
                    log::error!("Failed to set clipboard text: {}", e);
                    return ActionResult::Error(format!("Failed to set clipboard text: {}", e))
                }
                Ok(_) => { /* Clipboard set successfully */ }
            }
        } else {
            log::error!("Failed to open clipboard");
            return ActionResult::Error("Failed to open clipboard".to_string());
        }

        script::for_shortcut(format!("Ctrl V{}", if self.enter { " + Enter" } else { "" })).play();
        ActionResult::Success
    }
}

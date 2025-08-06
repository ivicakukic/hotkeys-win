use crate::{
    app::settings::{Settings, Pad, Action, ProfileMatch::NameEquals},
    input::script,
};

pub struct ActionExecutor;

impl ActionExecutor {
    pub fn execute_actions<F>(pad: &Pad, settings: &Settings, mut show_board_callback: F)
    where
        F: FnMut(crate::app::settings::Profile),
    {
        for action in pad.actions.as_slice() {
            match action {
                Action::Shortcut(text) => script::for_shortcut(text.clone()).play(),
                Action::Text(text) => script::for_text(text.clone(), settings.keyboard_layout()).play(),
                Action::Line(text) => script::for_line(text.clone(), settings.keyboard_layout()).play(),
                Action::Pause(pause) => script::for_pause(*pause).play(),
                Action::Board(keyword) => {
                    if let Some(profile) = settings.try_match(&NameEquals(keyword.clone())) {
                        show_board_callback(profile.clone());
                    }
                }
            }
        }
    }
}
mod traits;
mod boards;
mod main_board;
mod home_board;
mod controls;
mod colors_board;
mod fonts_board;
mod settings_board;
mod state_machine;
mod board_chain;
mod result_helpers;

pub struct PadMapping<R: SettingsRepository> {
    repository: Rc<R>
}

impl<R: SettingsRepository> PadMapping<R> {
    pub fn new(repository: Rc<R>) -> Self {
        Self { repository }
    }
}


use windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY;

impl <R: SettingsRepository> PadMapping<R> {
    pub fn map(&self, vk_code: VIRTUAL_KEY) -> Option<PadId> {
        map_pad_id(vk_code, self.repository.natural_key_order())
    }
}

pub fn map_pad_id(vk_code: VIRTUAL_KEY, natural_key_order: bool) -> Option<PadId> {
    use windows::Win32::UI::Input::KeyboardAndMouse::*;
    if ! natural_key_order {
        match vk_code {
            VK_NUMPAD1 | VK_1 => Some(PadId::One),
            VK_NUMPAD2 | VK_2 => Some(PadId::Two),
            VK_NUMPAD3 | VK_3 => Some(PadId::Three),
            VK_NUMPAD4 | VK_4 => Some(PadId::Four),
            VK_NUMPAD5 | VK_5 => Some(PadId::Five),
            VK_NUMPAD6 | VK_6 => Some(PadId::Six),
            VK_NUMPAD7 | VK_7 => Some(PadId::Seven),
            VK_NUMPAD8 | VK_8 => Some(PadId::Eight),
            VK_NUMPAD9 | VK_9 => Some(PadId::Nine),
            _ => None,
        }
    } else {
        match vk_code {
            VK_NUMPAD1 | VK_7 => Some(PadId::One),
            VK_NUMPAD2 | VK_8 => Some(PadId::Two),
            VK_NUMPAD3 | VK_9 => Some(PadId::Three),
            VK_NUMPAD4 | VK_4 => Some(PadId::Four),
            VK_NUMPAD5 | VK_5 => Some(PadId::Five),
            VK_NUMPAD6 | VK_6 => Some(PadId::Six),
            VK_NUMPAD7 | VK_1 => Some(PadId::Seven),
            VK_NUMPAD8 | VK_2 => Some(PadId::Eight),
            VK_NUMPAD9 | VK_3 => Some(PadId::Nine),
            _ => None,
        }
    }
}

trait EnumAll<T: Sized + Eq + PartialEq + Clone> {
    fn all() -> Vec<T>;
}

trait EnumTraversal<T: Sized + Eq + PartialEq + Clone> {
    fn next(&self) -> T;
    fn previous(&self) -> T;
    fn index(&self) -> usize;
}

impl<T: EnumAll<T> + Sized + Eq + PartialEq + Clone> EnumTraversal<T> for T {
    fn next(&self) -> T {
        let all = T::all();
        let idx = all.iter().position(|x| x == self).unwrap();
        all[(idx + 1) % all.len()].clone()
    }
    fn previous(&self) -> T {
        let all = T::all();
        let idx = all.iter().position(|x| x == self).unwrap();
        if idx == 0 {
            all[all.len() - 1].clone()
        } else {
            all[idx - 1].clone()
        }
    }

    fn index(&self) -> usize {
        let all = T::all();
        all.iter().position(|x| x == self).unwrap()
    }
}

const INITIAL_PATH_PARAM: &str = "initial_path";

use std::rc::Rc;

use result_helpers::*;

pub use traits::*;
pub use boards::*;
pub use controls::*;
pub use board_chain::*;
pub use main_board::MainBoard;
pub use home_board::HomeBoard;
pub use settings_board::SettingsBoard;

use crate::{core::SettingsRepository, model::PadId};



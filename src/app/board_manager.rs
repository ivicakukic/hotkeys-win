use crate::{
    app::settings::{Settings, Profile},
    ui::{
        windows::board::BoardWindow,
        shared::layout::WindowLayout,
    },
};

pub struct BoardManager {
    pub board: Option<Box<BoardWindow>>,
    pub layout: WindowLayout,
}

impl BoardManager {
    pub fn new() -> Self {
        Self {
            board: None,
            layout: WindowLayout::default(),
        }
    }

    pub fn new_with_layout(layout: WindowLayout) -> Self {
        Self {
            board: None,
            layout,
        }
    }

    pub fn show_board(&mut self, settings: &Settings, profile: Profile, is_timed: bool) {
        if let Some(ref mut _board) = self.board {
            panic!("Board already exists, cannot create a new one");
        }

        self.board = Some(BoardWindow::new(
            "HotKeys",
            self.layout.clone(),
            settings.find_scheme(&profile.color_scheme).unwrap_or_default(),
            profile,
            if is_timed { settings.timeout() as u32 } else { 0 },
            settings.feedback()
        ).unwrap());
    }

    pub fn hide_board(&mut self) {
        if let Some(ref mut board) = self.board {
            self.layout = board.layout.clone();
            board.hide();
            self.board = None;
        }
    }

    pub fn get_board_profile(&self) -> Option<Profile> {
        if let Some(ref board) = self.board {
            Some(board.profile.clone())
        } else {
            None
        }
    }
}
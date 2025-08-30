use std::rc::Rc;

use crate::{
    components::BoardComponent,
    settings::{LayoutSettings, Settings},
    ui::shared::layout::{Rect, WindowLayout, WindowStyle}
};

use super::windows::BoardWindow;

pub struct BoardManager {
    pub board: Option<Box<BoardWindow>>,
    pub settings: Rc<Settings>,
}

impl BoardManager {

    pub fn new(settings: Rc<Settings>) -> Self {
        Self {
            board: None,
            settings,
        }
    }

    fn layout(&self) -> WindowLayout {
        self.settings.get_layout_settings().map(|ls| ls.into()).unwrap_or_default()
    }

    pub fn show_board(&mut self, board: Box<dyn BoardComponent>, timeout: u32, feedback: u64) {
        if let Some(ref mut _board) = self.board {
            log::warn!("Board already displayed, cannot create a new one");
            return;
        }

        self.board = Some(BoardWindow::new(
            "HotKeys",
            self.layout(),
            board,
            timeout,
            feedback
        ).unwrap());
    }

    pub fn hide_board(&mut self) {
        if let Some(ref mut board) = self.board {
            board.hide();
            self.board = None;
        }
    }

    pub fn save_layout(&mut self) {
        if let Some(ref mut board) = self.board {
            self.settings.set_layout_settings(board.layout().clone().into());
        }
    }

    pub fn redraw_board(&self) {
        if let Some(ref board) = self.board {
            board.redraw();
        }
    }

}



// Mapping between LayoutSettings and WindowLayout
impl Into<LayoutSettings> for WindowLayout {
    fn into(self) -> LayoutSettings {
        LayoutSettings {
            x: self.rect.left,
            y: self.rect.top,
            width: self.rect.right - self.rect.left,
            height: self.rect.bottom - self.rect.top,
            window_style: self.style.to_string(),
        }
    }
}

impl From<LayoutSettings> for WindowLayout {
    fn from(layout: LayoutSettings) -> Self {
        WindowLayout {
            rect: Rect {
                left: layout.x,
                top: layout.y,
                right: layout.x + layout.width,
                bottom: layout.y + layout.height,
            },
            style: WindowStyle::from_string(&layout.window_style),
        }
    }
}
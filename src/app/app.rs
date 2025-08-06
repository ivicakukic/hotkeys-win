use std::{ sync::mpsc::{channel, Receiver}, thread, process::Command };
use std::ffi::c_void;
use windows::{
    core::{Result, HSTRING},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::{WindowsAndMessaging::{
            DefWindowProcW, DispatchMessageW, GetMessageW, MessageBoxW, PostMessageW, PostQuitMessage, TranslateMessage, IDOK, MB_ICONERROR, MB_OK, MB_OKCANCEL, MSG, WM_CLOSE
        }},
    },
};
use crate::{
    app ::{
        action_executor::ActionExecutor, board_manager::BoardManager, hook, message::Message::{self, *}, settings::{
            LayoutSettings, Pad, Profile, ProfileMatch::*, Settings, WM_HOOK_TRIGGER, WM_OPEN_SETTINGS, WM_RELOAD_SETTINGS
        }
    },
    ui::{
        components::tray, shared::layout::{Rect, WindowLayout, WindowStyle}, windows::{
            board::{WM_BOARD_COMMAND, WM_BOARD_FINISHED}, main::MainWindow
        }
    },
};

pub struct Application {
    settings: Settings,
    default_profile: Profile,
    board_manager: BoardManager,
}

impl Application {

    pub fn create(settings: Settings) -> Self {
        let board_manager = if let Some(layout) = settings.layout() {
            BoardManager::new_with_layout(layout.clone().into())
        } else {
            BoardManager::new()
        };

        Self {  settings,
                default_profile: Profile::new(
                    "Welcome 😊", "_Hello_", "", vec![
                        Pad::new("", "", vec![]),
                        Pad::new("", "Press any key to hide ...", vec![]),
                        Pad::new("", "", vec![]),
                        Pad::new("", "What's", vec![]),
                        Pad::new("", "", vec![]),
                        Pad::new("", "rustaman!", vec![]),
                        Pad::new("", "", vec![]),
                        Pad::new("", "up", vec![]),
                        Pad::new("", "", vec![])
                    ],-1
                ),
                board_manager,
            }
    }

    pub fn run(&mut self) -> Result<()> {
        let (tx, rx) = channel::<Message>();
        let join_handle = Self::event_proc(rx, &self.settings);

        hook::install(tx.clone());
        {
            let main_window = MainWindow::new("HotKeys", 20, 20)?; // , self as _)?;

            tx.send(WinCreated(main_window.hwnd())).unwrap_or_default();
            let _tray = tray::create(main_window.hwnd());

            self.board_manager.show_board(&self.settings, self.default_profile.clone(), false);

            let mut message = MSG::default();
            unsafe {
                while GetMessageW(&mut message, None, 0, 0).into() {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }
        hook::uninstall();

        tx.send(Quit).unwrap_or_default();
        join_handle.join().unwrap();
        Ok(())
    }



    fn app_wnd_proc(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            match msg {
                WM_CLOSE => {
                    if IDOK == MessageBoxW(Some(hwnd), &HSTRING::from("Really?"), &HSTRING::from("Exit HotKeys"), MB_OKCANCEL) {
                        PostQuitMessage(0);
                    }
                },
                WM_OPEN_SETTINGS => {
                    self.open_settings_editor();
                },
                WM_RELOAD_SETTINGS => {
                    self.settings.reload();
                },
                WM_HOOK_TRIGGER => {
                    if let Some(profile) = self.settings.try_match(&IdEquals(lparam.0)) {
                        self.board_manager.show_board(&self.settings, profile.clone(), true);
                    }
                },
                WM_BOARD_COMMAND => {
                    self.handle_board_command(wparam.0);
                }
                WM_BOARD_FINISHED => {
                    self.board_manager.hide_board();
                },
                _ => return DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            LRESULT(0)
        }
    }

    fn event_proc(rx: Receiver<Message>, settings:&Settings) -> thread::JoinHandle<()>  {
        let settings = settings.clone();
        let mut main_hwnd: Option<isize> = None;

        thread::spawn(move || {

            while let Ok(msg) = rx.recv() {
                match msg {
                    WinCreated(hwnd) => {
                        main_hwnd = Some(hwnd);
                    },
                    HookEvt(pinfo) => {
                        if let (Some(hwnd), Some(profile)) = (main_hwnd, settings.try_match(&NameContains(pinfo.name))) {
                            unsafe {
                                PostMessageW(
                                    Some(HWND(hwnd as *mut c_void)),
                                    WM_HOOK_TRIGGER,
                                    WPARAM(0),
                                    LPARAM(profile.id)
                                ).unwrap_or_default();
                            }
                        }

                    },
                    Quit => { break; }
                }
            }
            log::info!("Thread: Exiting");

        })

    }


    fn handle_board_command(&mut self, pad_id: usize) {
        let profile = self.board_manager.get_board_profile();

        self.board_manager.hide_board();

        if let Some(profile) = profile {
            if pad_id < profile.pads.len() {
                let pad = &profile.pads[pad_id];
                let settings = &self.settings;
                let board_manager = &mut self.board_manager;
                ActionExecutor::execute_actions(pad, settings, |profile| {
                    board_manager.show_board(settings, profile, false);
                });
            }
        }

    }


    fn open_settings_editor(&self) {
        let editor_path = self.settings.editor();
        let file_path = self.settings.file_path();
        if let Err(e) = Command::new(editor_path).args([file_path]).spawn() {
            unsafe {
                MessageBoxW(None,
                    &HSTRING::from(format!("An issue occured while opening the file: {}", e).as_str()),
                    &HSTRING::from("Error"), MB_OK | MB_ICONERROR);
            }
        }
    }

    pub fn save_layout_to_settings(&mut self) {
        let layout = self.board_manager.layout.clone();
        self.settings.set_layout(layout.clone().into());
        self.settings.save();
    }

}

// Implementation of the framework::AppHandler trait for Application
use crate::framework::AppHandler;
impl AppHandler for Application {
    fn app_wnd_proc(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        self.app_wnd_proc(hwnd, msg, wparam, lparam)
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
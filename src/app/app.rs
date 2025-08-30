use std::{ process::Command, rc::Rc, sync::mpsc::{channel, Receiver}, thread };
use std::ffi::c_void;
use windows::{
    core::{Result, HSTRING},
    Win32::{
        Foundation::{HWND, LPARAM, LRESULT, WPARAM},
        UI::WindowsAndMessaging::{
            DefWindowProcW, DispatchMessageW, GetMessageW, MessageBoxW, PostQuitMessage, TranslateMessage, IDOK, MB_ICONERROR, MB_OK, MB_OKCANCEL, MSG, WM_CLOSE, WM_USER
        },
    },
};

use super::{
    BoardManager, ActionFactoryRegistry, BoardFactoryRegistry, ActionFactoryImpl, BoardFactoryImpl,
    hook, hook::win_icon, message, message::Message,
    windows::{ MainWindow, tray_item, WM_BOARD_COMMAND, WM_BOARD_FINISHED, WM_UPDATE_LAYOUT, WM_OPEN_SETTINGS, WM_RELOAD_SETTINGS, WM_SAVE_SETTINGS }
};

use crate::{
    app::windows::WM_SHOW_APPLICATION, core::{data::Detection, resources::DetectedIcon, Param, Resources, SettingsRepository, SettingsRepositoryMut}, model::{PadId, PadSet}, settings::*, ui::shared::utils
};

pub const WM_HOOK_TRIGGER:u32 = WM_USER + 1;

#[repr(C)]
struct ProcessInfo {
    pub pid: u32,
    pub name: [u8; 260], // MAX_PATH
    pub title: [u8; 260], // MAX_TITLE
}

impl ProcessInfo {
    pub fn new(pinfo: message::ProcessInfo) -> Self {
        let mut info = ProcessInfo {
            pid: pinfo.pid,
            name: [0; 260],
            title: [0; 260],
        };
        utils::copy_string_to_array(&mut info.name, &pinfo.name);
        utils::copy_string_to_array(&mut info.title, &pinfo.title);
        info
    }

    pub fn get_name(&self) -> &str {
        utils::get_string_from_array(&self.name)
    }
    pub fn get_title(&self) -> &str {
        utils::get_string_from_array(&self.title)
    }
}




pub struct Application {
    settings: Rc<Settings>,
    action_factory_registry: ActionFactoryRegistry<Settings>,
    board_factory_registry: BoardFactoryRegistry<Settings>,
    board_manager: BoardManager,
    restart_info: Option<Option<String>>,
}

impl Application {

    pub fn create(
        settings: Rc<Settings>,
        action_factory_registry: ActionFactoryRegistry<Settings>,
        board_factory_registry: BoardFactoryRegistry<Settings>
    ) -> Self {
        let board_manager = BoardManager::new(settings.clone());

        Self { settings, action_factory_registry, board_factory_registry, board_manager, restart_info: None }
    }

    fn show_board(&mut self, board_name: String, params: Vec<Param>, timeout: u32) ->  core::result::Result<(), Box<dyn std::error::Error>> {
        let board_factory_registry = &self.board_factory_registry;
        let board_factory = BoardFactoryImpl::new(self.settings.clone(), board_factory_registry, self.settings.get_resources().clone());
        let board_trait = board_factory.create_board(&board_name, params);

        match board_trait {
            Ok(board_trait) => {
                self.board_manager.show_board(board_trait, timeout, self.settings.feedback());
                Ok(())
            },
            Err(err) => {
                log::error!("Failed to create board '{}': {}", board_name, err);
                Err(err)
            }
        }
    }


    pub fn run(&mut self, board_name: Option<String>, params: Vec<Param>) -> Result<()> {
        let (tx, rx) = channel::<Message>();
        let join_handle = Self::event_proc(rx,
            self.settings.get_resources().clone(),
            self.settings.detections().to_vec()
        );

        hook::install(tx.clone());
        {
            let main_window = MainWindow::new("HotKeys", 20, 20)?; // , self as _)?;

            tx.send(Message::WinCreated(main_window.hwnd())).unwrap_or_default();
            let _tray = tray_item(main_window.hwnd());

            let board_name = board_name
            .and_then(|name| self.settings
                .get_board(&name)
                .ok()
                .map(|b| b.name.clone())
            ).unwrap_or_else(|| self.settings.home_board_name());

            self.show_board(board_name, params, 0).unwrap_or_default();

            let mut message = MSG::default();
            unsafe {
                while GetMessageW(&mut message, None, 0, 0).into() {
                    let _ = TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
            }
        }
        hook::uninstall();

        tx.send(Message::Quit).unwrap_or_default();
        join_handle.join().unwrap();
        Ok(())
    }


    fn app_wnd_proc(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            match msg {
                WM_CLOSE => {
                    let question = if self.settings.is_dirty() {
                        "You have unsaved changes. Close application without saving?"
                    } else {
                        "Close application?"
                    };
                    if IDOK == MessageBoxW(Some(hwnd), &HSTRING::from(question), &HSTRING::from("HotKeys"), MB_OKCANCEL) {
                        PostQuitMessage(0);
                    }
                },
                WM_OPEN_SETTINGS => {
                    self.open_settings_editor();
                },
                WM_RELOAD_SETTINGS => {
                    match self.settings.reload() {
                        Err(e) => {
                            MessageBoxW(Some(hwnd), &HSTRING::from(format!("Failed to reload settings: {}", e)), &HSTRING::from("Error"), MB_OK | MB_ICONERROR);
                        }
                        Ok(_) => {
                            log::info!("Settings reloaded");
                            self.board_manager.redraw_board();
                        }
                    }
                },
                WM_SAVE_SETTINGS => {
                    match self.settings.flush() {
                        Err(e) => {
                            MessageBoxW(Some(hwnd), &HSTRING::from(format!("Failed to save settings: {}", e)), &HSTRING::from("Error"), MB_OK | MB_ICONERROR);
                        }
                        Ok(_) => {
                            log::info!("Settings saved");
                            // self.post_setting_changed(hwnd);
                        }
                    }
                },
                WM_SHOW_APPLICATION => {
                    self.show_board(self.settings.home_board_name(), vec![], 0).unwrap_or_default();
                },
                WM_HOOK_TRIGGER => {
                    let process_info = utils::receive_window_message::<ProcessInfo>(wparam);
                    let board_name = self.settings.detect(process_info.get_name());
                    let params = if board_name.is_some() { vec![] } else { vec![
                        Param { name: "process_name".to_string(), value: process_info.get_name().to_string() },
                        Param { name: "window_title".to_string(), value: process_info.get_title().to_string() },
                    ]};
                    let board_name = board_name.unwrap_or_else(|| self.settings.home_board_name());
                    self.show_board(board_name, params, self.settings.timeout() as u32).unwrap_or_default();

                },
                WM_BOARD_COMMAND => {
                    self.handle_board_command(wparam.0);
                }
                WM_BOARD_FINISHED => {
                    self.board_manager.hide_board();
                },
                WM_UPDATE_LAYOUT => {
                    self.board_manager.save_layout();
                }
                _ => return DefWindowProcW(hwnd, msg, wparam, lparam)
            }
            LRESULT(0)
        }
    }

    fn event_proc(rx: Receiver<Message>, resources: Resources, detections: Vec<Detection>) -> thread::JoinHandle<()>  {
        let mut main_hwnd: Option<isize> = None;
        let mut saved_icons: Vec<DetectedIcon> = vec![];

        thread::spawn(move || {

            while let Ok(msg) = rx.recv() {
                match msg {
                    Message::WinCreated(hwnd) => {
                        main_hwnd = Some(hwnd);
                    },
                    Message::HookEvt(pinfo) => {
                        if let Some(hwnd) = main_hwnd {
                            // Skip icon fetching if we have this process pre-configured
                            let detection = detections.iter().find(|d| d.is_match(&pinfo.name));
                            if detection.is_none() {

                                // Skip if we already saved the icon for this process in this session
                                let png_path = resources.detected_icon(pinfo.name.clone());
                                if !saved_icons.iter().any(|icon| icon.name() == png_path.name()) {

                                    // Try to save the icon
                                    if win_icon::save_first_window_icon(HWND(pinfo.hwnd as *mut c_void), &png_path.path()).is_ok() {
                                        log::info!("Saved detected icon for process '{}' to '{}'", pinfo.name, png_path.path().display());
                                        saved_icons.push(png_path);
                                    }
                                }
                            }
                            utils::send_window_message(HWND(hwnd as *mut c_void), WM_HOOK_TRIGGER, ProcessInfo::new(pinfo));
                        }
                    },
                    Message::Quit => { break; }
                }
            }
            log::info!("Thread: Exiting");

        })

    }

    fn handle_board_command(&mut self, pad_id: usize) {

        // Get selected pad and close window
        let pad = self.board_manager.board.as_ref()
            .map(|bw| bw.board().data()
                .padset(Some(bw.modifier_state().clone()))
                .flatten()
                .pad(PadId::from_keypad_int(pad_id as i32)))
            .unwrap_or_else(|| PadId::One.into());

        self.board_manager.hide_board();

        // Execute actions first
        let mut needs_reload = false;
        let mut needs_restart = false;


        for action_type in pad.actions() {
            let action_factory_registry = &self.action_factory_registry;
            let action_factory = ActionFactoryImpl::new(self.settings.clone(), action_factory_registry);
            let action = action_factory.create_action(action_type);

            match action.run() {
                crate::app::action_factory::ActionResult::Success => {
                    if action.requires_reload() {
                        needs_reload = true;
                    }
                    if action.requires_restart() {
                        needs_restart = true;
                    }
                },
                crate::app::action_factory::ActionResult::Error(err) => {
                    log::error!("Action execution failed: {}", err);
                }
            }
        }

        // Handle reload if any action requested it
        if needs_reload {
            self.settings.reload().unwrap_or_default();
        }

        // Handle restart if any action requested it
        if needs_restart {
            // If restart is needed, use pad.board as the board to restart to
            self.initiate_restart(pad.board());
            return; // Exit early since we're restarting
        }

        // Handle board navigation (only if not restarting)
        if let Some(ref board_name) = pad.board() {
            if let Ok(board) = self.settings.get_board(board_name) {
                self.show_board(board.name, pad.board_params().to_vec(), 0).unwrap_or_default();
            }
        }
    }

    fn open_settings_editor(&self) {
        let editor_path = self.settings.editor();
        if let Some(settings_path) = self.settings.get_resources().settings_json() {
            if let Some(file_path) = settings_path.to_str() {
                if let Err(e) = Command::new(editor_path).args([file_path]).spawn() {
                    unsafe {
                        MessageBoxW(None,
                            &HSTRING::from(format!("An issue occured while opening the file: {}", e).as_str()),
                            &HSTRING::from("Error"), MB_OK | MB_ICONERROR);
                    }
                }
            }
        }
    }

    pub fn restart_info(&self) -> &Option<Option<String>> {
        &self.restart_info
    }

    pub fn initiate_restart(&mut self, initial_board: Option<String>) {
        self.restart_info = Some(initial_board);
        unsafe {
            PostQuitMessage(0);
        }
    }

}

// Implementation of the framework::AppHandler trait for Application
use crate::framework::AppHandler;
impl AppHandler for Application {
    fn app_wnd_proc(&mut self, hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        self.app_wnd_proc(hwnd, msg, wparam, lparam)
    }
}

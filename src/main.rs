#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// #![allow(dead_code)]

mod framework;
mod app;
mod ui;
mod core;
mod input;
mod model;
mod components;
mod settings;

use crate::app::{Application, ActionFactoryRegistry, BoardFactoryRegistry};
use crate::settings::Settings;
use crate::framework::{set_app_handler};
use crate::ui::components::{svg::ICON_CACHE, png::PNG_CACHE};
use crate::core::{Param, Resources};

use windows::core::{Result, Error};
use std::{env, path::PathBuf, process::Command};

#[derive(Debug)]
struct Args {
    config_dir: Option<String>,
    board: Option<String>,
    params: Vec<Param>,
}

fn parse_args() -> Args {
    let args: Vec<String> = env::args().collect();

    let mut config_dir: Option<String> = None;
    let mut board: Option<String> = None;
    let mut params: Vec<Param> = Vec::new();
    let mut i = 1;
    let mut parsing_params = false;

    // Parse options
    // After "--" everything is treated as a parameter, and we expect the form --key <value>
    while i < args.len() {
        match args[i].as_str() {
            "--config_dir" => {
                if i + 1 < args.len() {
                    config_dir = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("ERROR: --config_dir requires a value");
                    std::process::exit(1);
                }
            },
            "--board" => {
                if i + 1 < args.len() {
                    board = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    eprintln!("ERROR: --board requires a value");
                    std::process::exit(1);
                }
            },
            "--" => {
                parsing_params = true;
                i += 1;
            },
            _ if parsing_params => {
                // Expecting --key value pairs
                if args[i].starts_with("--") {
                    let key = args[i][2..].to_string();
                    if i + 1 < args.len() && !args[i + 1].starts_with("--") {
                        let value = args[i + 1].clone();
                        params.push(Param::new(key, value));
                        i += 2;
                    } else {
                        eprintln!("ERROR: Parameter {} requires a value", key);
                        std::process::exit(1);
                    }
                } else {
                    eprintln!("ERROR: Expected parameter starting with --, found {}", args[i]);
                    std::process::exit(1);
                }
            },
            _ => {
                eprintln!("ERROR: Unknown argument {}", args[i]);
                std::process::exit(1);
            }
        }
    }
    Args { config_dir, board, params }
}



pub fn get_resource_path(config_dir: Option<PathBuf>) -> PathBuf {
    fn get_current_exe_dir() -> Option<PathBuf> {
        if let Ok(exe_path) = std::env::current_exe() {
            return exe_path.parent().map(|p| p.to_path_buf());
        }
        None
    }

    fn get_dev_resources_dir() -> Option<PathBuf> {
        // Dynamic resolution instead of using #[cfg(debug_assertions)]
        // This allows using development resources even with release builds

        if let Some(exe_dir) = get_current_exe_dir() {
            let dev_resources = exe_dir.join("..\\..\\resources");
            if dev_resources.exists() {
                return Some(dev_resources);
            }
        }
        None
    }

    // PRODUCTION:
    // use either config_dir OR "<exe_dir>/resources"

    let config_dir = if let Some(config_dir) = config_dir {
        Some(config_dir)
    } else {
        get_current_exe_dir().map(|exe_dir| exe_dir.join("resources"))
    };

    if let Some(user_config_dir) = config_dir {
        if user_config_dir.exists() {
            // Use user config directory
            return user_config_dir;
        }
    }

    // DEVELOPMENT:
    // try to fallback to development resources

    get_dev_resources_dir().unwrap_or_else(|| {
        eprintln!("ERROR: Resources directory not found");
        std::process::exit(1);
    })
}

fn initialize_icon_caches(resources: &Resources) {
    ICON_CACHE.with(|cache| {
        cache.borrow_mut().initialize(resources.clone());
    });
    PNG_CACHE.with(|cache| {
        cache.borrow_mut().initialize(resources.clone());
    });
}

fn run() -> Result<()> {
    let args = parse_args();
    let resources = Resources::new(vec![get_resource_path(args.config_dir.clone().map(PathBuf::from))]);

    // Initialize icon caches with resources
    initialize_icon_caches(&resources);

    log4rs::init_file(resources.log_toml().unwrap(), Default::default()).expect("Log init error");
    log::warn!("Starting HotKeys");
    log::info!("Args: {:?}", args);

    let settings = Settings::load(resources.clone())
        .map_err(|e| {
            log::error!("Failed to load settings: {}", e);
            eprintln!("Error: Failed to load settings: {}", e);
            Error::from_hresult(windows::Win32::Foundation::E_FAIL)
        })?;
    let action_factory_registry = ActionFactoryRegistry::<Settings>::new();
    let board_factory_registry = BoardFactoryRegistry::<Settings>::new();

    // Register custom factories
    // action_factory_registry.register_factory(...);
    // board_factory_registry.register_factory(...);

    let mut app = Application::create(settings, action_factory_registry, board_factory_registry);
    set_app_handler::<Application>(&mut app);
    app.run(args.board.clone(), args.params.clone())?;

    // Check if restart was requested
    if let Some(restart_board) = app.restart_info().clone() {
        restart_with_board(restart_board, &args);
    }

    // Clear the icon caches here rather than leaving it to overlap with async appender destruction
    ICON_CACHE.with(|cache| cache.borrow_mut().clear());
    PNG_CACHE.with(|cache| cache.borrow().clear());

    log::warn!("Exiting HotKeys");
    Ok(())
}

fn restart_with_board(restart_board: Option<String>, original_args: &Args) {
    let current_exe = env::current_exe().expect("Failed to get current executable path");

    let mut new_args = Vec::new();

    // Add config_dir argument if it was specified
    if let Some(ref config_dir) = original_args.config_dir {
        new_args.push("--config_dir".to_string());
        new_args.push(config_dir.clone());
    }

    // Add new --board argument if specified
    if let Some(board_name) = restart_board {
        new_args.push("--board".to_string());
        new_args.push(board_name);
    }

    log::info!("Restarting with args: {:?}", new_args);

    // Start new process and exit immediately
    if let Err(e) = Command::new(current_exe).args(new_args).spawn() {
        log::error!("Failed to restart application: {}", e);
        eprintln!("Failed to restart application: {}", e);
    }

    std::process::exit(0);
}

fn main() {
    if let Err(error) = run() {
        error.code().unwrap();
    }
}

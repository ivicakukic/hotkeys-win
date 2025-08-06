#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![feature(unboxed_closures)]
#![feature(fn_traits)]
// #![allow(dead_code)]

mod framework;
mod app;
mod ui;
mod input;

use crate::app::{settings, app::Application};
use crate::framework::{set_app_handler};
use windows::core::Result;

fn run() -> Result<()> {
    log4rs::init_file("resources/log.toml", Default::default()).expect("Log init error");
    log::info!("Starting HotKeys");

    let mut app = Application::create(settings::load("resources/settings.json"));
    set_app_handler::<Application>(&mut app);
    app.run()?;

    // Save the current layout to settings before exiting
    app.save_layout_to_settings();

    log::info!("Exiting HotKeys");
    Ok(())
}

fn main() {
    let result = run();

    // We do this for nicer HRESULT printing when errors occur.
    if let Err(error) = result {
        error.code().unwrap();
    }
}

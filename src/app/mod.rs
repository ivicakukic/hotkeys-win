// pub mod settings;
mod message;
mod app;
mod hook;
mod board_manager;
mod action_factory;
mod board_factory;
mod windows;

use action_factory::ActionFactoryImpl;
use board_factory::BoardFactoryImpl;
use board_manager::BoardManager;

pub use app::Application;
pub use action_factory::{ ActionFactoryRegistry };
pub use board_factory::{ BoardFactoryRegistry, BoardFactory, BoardRuntimeContext };
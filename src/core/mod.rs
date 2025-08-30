pub mod resources;
pub mod data;
pub mod repository;
pub mod integration;


// #[cfg(test)]

pub use data::{TextStyle, ColorScheme, Board, PadSet, Pad, Detection};
pub use repository::{SettingsRepository, SettingsRepositoryMut};
pub use integration::{ActionType, ActionParams, BoardType, Param, Params, PathString};
// pub use integration::*;

pub use resources::{Resources, DetectedIcon, slugify_process_name};
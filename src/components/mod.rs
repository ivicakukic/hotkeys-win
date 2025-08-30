mod traits;
mod boards;
mod main_board;
mod home_board;
mod controls;
mod colors_board;
mod fonts_board;
mod settings_board;
mod state_machine;
mod result_helpers;

const INITIAL_PATH_PARAM: &str = "initial_path";

use result_helpers::*;
use boards::*;

pub use traits::*;
pub use boards::StateMachineBoard;
pub use main_board::MainBoard;
pub use home_board::HomeBoard;
pub use settings_board::SettingsBoard;



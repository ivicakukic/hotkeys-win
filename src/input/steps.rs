use std::time::Duration;

use crate::input::api;

use std::any::Any;



#[derive(Debug, PartialEq)]
pub struct KeyInput {
    pub vk_code : u16,
    pub key_down : bool
}

pub struct KeyInputs {
    pub inputs: Vec<KeyInput>
}

#[derive(Debug, PartialEq)]
pub struct NoInput {
    pub pause: u16
}

pub trait InputStep {
    fn play(&self);
    #[allow(dead_code)]
    fn as_any(&self) -> &dyn Any;
}


impl InputStep for NoInput {
    fn play(&self) {
        if self.pause > 0 {
            std::thread::sleep(Duration::from_millis(self.pause as u64));
        }
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl InputStep for KeyInput {
    fn play(&self) {
        api::send_input(
            map_api_input(self)
        );
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl InputStep for KeyInputs {
    fn play(&self) {
        api::send_inputs(
            self.inputs.iter().map(
                |input| map_api_input(input)
            ).collect()
        );
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

fn map_api_input(input: &KeyInput) -> api::KeyboardInput {
    api::KeyboardInput {
        vk_code: input.vk_code,
        key_down: input.key_down
    }
}
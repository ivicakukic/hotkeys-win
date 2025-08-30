use std::any::Any;
use super::UiEventResult;
use crate::model::Pad;

/// Apply a String result using the provided handler function
pub fn apply_string<F>(result: Box<dyn Any>, handler: F) -> UiEventResult
where F: FnOnce(&str) -> Result<(), Box<dyn std::error::Error>>
{
    if let Some(string_result) = result.downcast_ref::<String>() {
        match handler(string_result) {
            Ok(()) => UiEventResult::RequiresRedraw,
            Err(e) => {
                log::error!("String handler failed: {}", e);
                UiEventResult::RequiresRedraw
            }
        }
    } else {
        log::warn!("Expected String result but got different type");
        UiEventResult::NotHandled
    }
}

/// Apply a Pad result using the provided handler function
#[allow(dead_code)]
pub fn apply_pad<F>(result: Box<dyn Any>, handler: F) -> UiEventResult
where F: FnOnce(&Pad) -> Result<(), Box<dyn std::error::Error>>
{
    if let Some(pad_result) = result.downcast_ref::<Pad>() {
        match handler(pad_result) {
            Ok(()) => UiEventResult::RequiresRedraw,
            Err(e) => {
                log::error!("Pad handler failed: {}", e);
                UiEventResult::RequiresRedraw
            }
        }
    } else {
        log::warn!("Expected Pad result but got different type");
        UiEventResult::NotHandled
    }
}
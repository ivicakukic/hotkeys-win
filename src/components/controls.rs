use num_traits::ToPrimitive;
use windows::Win32::UI::Input::KeyboardAndMouse::{VIRTUAL_KEY, VK_DOWN, VK_LEFT, VK_RIGHT, VK_UP};

use crate::core;
use crate::model::{Anchor, Pad, PadId, Tag};
use super::{KeyboardEvent, UiEventResult};

use std::ops::{Add, Div, Mul, Sub};

pub struct NumericSpinnerPad<T> {
    pad_id: PadId,
    value: T,
    min: T,
    max: T,
    step: T,
    direction: i32,
    format: fn(T) -> String,
}

impl<T> NumericSpinnerPad<T>
where
    T: Copy + PartialOrd + Add<Output = T> + Sub<Output = T> + std::fmt::Display,
{
    pub fn new(pad_id: PadId, value: T, min: T, max: T, step: T, format: Option<fn(T) -> String>) -> Self {
        Self {
            pad_id,
            value,
            min,
            max,
            step,
            format : format.unwrap_or(|v| format!("{}", v)),
            direction: 0,
        }
    }

    pub fn value(&self) -> T {
        self.value
    }

    pub fn parsed_formatted_value(&self) -> Result<T, Box<dyn std::error::Error>>
    where
        T: std::str::FromStr,
        T::Err: std::error::Error + 'static,
    {
        let formatted_string = (self.format)(self.value);
        let parsed_value = formatted_string.parse::<T>()?;
        Ok(parsed_value)
    }
}

impl<T> NumericSpinnerPad<T>
where
    T: Copy + PartialOrd + Add<Output = T> + Sub<Output = T> + std::fmt::Display,
{
    pub fn key_down(&mut self, event: KeyboardEvent) -> UiEventResult {
        match event.into() {
            VK_UP | VK_DOWN => {
                self.direction = if VIRTUAL_KEY::from(event) == VK_UP { 1 } else { -1 };
                let inc = if event.modifiers.shift {
                    // If shift is held, increase step by 10x
                    let mut inc = self.step;
                    for _ in 1..10 {
                        inc = inc + self.step;
                    }
                    inc
                } else {
                    self.step
                };
                let new_val = if self.direction == 1 {
                    self.value + inc
                } else {
                    self.value - inc
                };
                if new_val > self.max {
                    self.value = self.max;
                } else if new_val < self.min {
                    self.value = self.min;
                } else {
                    self.value = new_val;
                }
                UiEventResult::RequiresRedraw
            }
            _ => UiEventResult::NotHandled,
        }
    }

    pub fn key_up(&mut self, event: KeyboardEvent) -> UiEventResult {
        match event.into() {
            VK_UP | VK_DOWN => {
                self.direction = 0;
                UiEventResult::RequiresRedraw
            }
            _ => UiEventResult::NotHandled,
        }
    }


    pub fn get_pad(&self) -> Pad {

        let text = (self.format)(self.value);

        Pad::from(self.pad_id).with_data(core::Pad {
            text: Some(text),
            ..Default::default()
        })
        .with_tags(vec![
            Tags::UpWhite.with(Anchor::N, if self.direction == 1 { Some(0) } else { None }, Some(2)),
            Tags::DownWhite.with(Anchor::S, if self.direction == -1 { Some(0) } else { None }, Some(2)),
            Tags::RightBlack.tag(Anchor::W),
        ])
    }
}


pub struct HSlider<T> {
    label: String,
    value: T,
    min: T,
    max: T,
    step: T,
    format: fn(T) -> String,
}

impl<T> HSlider<T>
where
    T: Copy + PartialOrd + Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T> + ToPrimitive + std::fmt::Display,
{
    pub fn new(label: String, value: T, min: T, max: T, step: T, format: Option<fn(T) -> String>) -> Self {
        Self {
            label,
            value,
            min,
            max,
            step,
            format: format.unwrap_or(|v| format!("{}", v)),
        }
    }

    pub fn value(&self) -> T {
        self.value
    }
}

impl<T> HSlider<T>
where
    T: Copy + PartialOrd + Add<Output = T> + Sub<Output = T> + Mul<Output = T> + Div<Output = T> + ToPrimitive + std::fmt::Display,
{
    pub fn key_down(&mut self, event: KeyboardEvent) -> UiEventResult {
        match event.into() {
            VK_LEFT | VK_RIGHT => {
                let inc = if event.modifiers.shift {
                    // If shift is held, increase step by 10x
                    let mut inc = self.step;
                    for _ in 1..10 {
                        inc = inc + self.step;
                    }
                    inc
                } else {
                    self.step
                };
                let new_val = if VIRTUAL_KEY::from(event) == VK_RIGHT {
                    self.value + inc
                } else {
                    self.value - inc
                };
                if new_val > self.max {
                    self.value = self.max;
                } else if new_val < self.min {
                    self.value = self.min;
                } else {
                    self.value = new_val;
                }
                UiEventResult::RequiresRedraw
            }
            _ => UiEventResult::NotHandled,
        }
    }

    // Creates a Tag representing the current value and ascii representation of the slider (10 segments): "<label> |■■■■■□□□□□| <formatted_value>"
    // Segments are filled based on the value's position between min and max (use f32 for internal calculation, then convert back to T for display)
    pub fn get_tag(&self, anchor: Anchor) -> Tag {
       let segments = 16;

        // Convert to f32 safely
        let min = self.min.to_f32().unwrap();
        let max = self.max.to_f32().unwrap();
        let value = self.value.to_f32().unwrap();

        let ratio = (value - min) / (max - min);
        let filled_segments = (ratio * segments as f32).round() as usize;
        let empty_segments = segments - filled_segments;

        let filled_bar = "■".repeat(filled_segments);
        let empty_bar = "□".repeat(empty_segments);

        let text = format!(
            "{} |{}{}| {}",
            self.label,
            filled_bar,
            empty_bar,
            (self.format)(self.value)
        );

        Tag {
            text,
            anchor,
            ..Default::default()
        }
    }

}


pub enum Tags {
    RightBlack,
    LeftBlack,
    UpBlack,
    DownBlack,
    #[allow(dead_code)]
    RightWhite,
    #[allow(dead_code)]
    LeftWhite,
    UpWhite,
    DownWhite,
    DownUp,
    LeftRight,
    LeftTextRight(String),
    EscEnter,
}

impl Tags {
    pub fn to_string(&self) -> String {
        match self {
            Tags::RightBlack => "▶".to_string(),
            Tags::LeftBlack => "◀".to_string(),
            Tags::UpBlack => "▲".to_string(),
            Tags::DownBlack => "▼".to_string(),
            Tags::RightWhite => "▷".to_string(),
            Tags::LeftWhite => "◁".to_string(),
            Tags::UpWhite => "△".to_string(),
            Tags::DownWhite => "▽".to_string(),
            Tags::DownUp => "▽△".to_string(),
            Tags::LeftRight => "◁▷".to_string(),
            Tags::LeftTextRight(text) => format!("◁ {} ▷", text),
            Tags::EscEnter => "esc/enter".to_string(),
        }
    }

    pub fn default(&self) -> Tag {
        match self {
            Tags::RightBlack => Tag { text: self.to_string(), anchor: Anchor::W, font_idx: None, color_idx: Some(0), ..Default::default() },
            Tags::LeftBlack => Tag { text: self.to_string(), anchor: Anchor::E, font_idx: None, color_idx: Some(0), ..Default::default() },
            Tags::UpBlack => Tag { text: self.to_string(), anchor: Anchor::S, font_idx: None, color_idx: Some(0), ..Default::default() },
            Tags::DownBlack => Tag { text: self.to_string(), anchor: Anchor::N, font_idx: None, color_idx: Some(0), ..Default::default() },
            Tags::RightWhite => Tag { text: self.to_string(), anchor: Anchor::W, font_idx: Some(2), color_idx: None, ..Default::default() },
            Tags::LeftWhite => Tag { text: self.to_string(), anchor: Anchor::E, font_idx: Some(2), color_idx: None, ..Default::default() },
            Tags::UpWhite => Tag { text: self.to_string(), anchor: Anchor::S, font_idx: Some(2), color_idx: None, ..Default::default() },
            Tags::DownWhite => Tag { text: self.to_string(), anchor: Anchor::N, font_idx: Some(2), color_idx: None, ..Default::default() },
            Tags::DownUp => Tag { text: self.to_string(), anchor: Anchor::SE, font_idx: Some(2), color_idx: None, ..Default::default() },
            Tags::LeftRight => Tag { text: self.to_string(), anchor: Anchor::SE, font_idx: Some(2), color_idx: None, ..Default::default() },
            Tags::LeftTextRight(_) => Tag { text: self.to_string(), anchor: Anchor::SE, font_idx: None, color_idx: None, ..Default::default() },
            Tags::EscEnter => Tag { text: self.to_string(), anchor: Anchor::NE, font_idx: Some(0), color_idx: None, ..Default::default() }
        }
    }

    pub fn tag(&self, anchor: Anchor) -> Tag {
        let mut tag = self.default();
        tag.anchor = anchor;
        tag
    }

    pub fn with(&self, anchor: Anchor, color_idx: Option<usize>, font_idx: Option<usize>) -> Tag {
        let mut tag = self.default();
        tag.anchor = anchor;
        tag.color_idx = color_idx;
        tag.font_idx = font_idx;
        tag
    }
}

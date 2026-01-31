use embedded_graphics::{
    mono_font::{MonoTextStyleBuilder, ascii::FONT_6X10},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text, TextStyleBuilder},
};

use crate::hardware::SSD1306Hardware;

pub struct Display<'a> {
    hardware: SSD1306Hardware<'a>,
}

impl<'a> Display<'a> {
    pub fn new(hardware: SSD1306Hardware<'a>) -> Self {
        Self { hardware }
    }

    pub fn clear(&mut self) -> Result<(), &'static str> {
        self.hardware.clear()
    }

    pub fn draw_text(&mut self, text: &str, x: i32, y: i32) -> Result<(), &'static str> {
        let text_style = MonoTextStyleBuilder::new()
            .font(&FONT_6X10)
            .text_color(BinaryColor::On)
            .build();

        let baseline_style = TextStyleBuilder::new().baseline(Baseline::Top).build();

        let text_obj = Text::with_text_style(text, Point::new(x, y), text_style, baseline_style);

        self.hardware.draw_text(text_obj)
    }

    pub fn draw_line(&mut self, start: Point, end: Point) -> Result<(), &'static str> {
        let line = embedded_graphics::primitives::Line::new(start, end);

        self.hardware.draw_line(line)
    }
}

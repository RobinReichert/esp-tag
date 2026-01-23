use crate::hardware::{bmp, error::DisplayError};
use embedded_graphics::{
    image::{Image, ImageRaw},
    mono_font::{MonoTextStyle, MonoTextStyleBuilder, ascii::FONT_6X10, iso_8859_2::FONT_9X18},
    pixelcolor::BinaryColor,
    prelude::{Point, *},
    text::{Alignment, Baseline, Text},
};
use embedded_hal_async::i2c::I2c;
use ssd1306::{I2CDisplayInterface, Ssd1306Async, mode::BufferedGraphicsModeAsync, prelude::*};

const WIDTH: usize = 128;
const HEIGHT: usize = 32;
const BUFFER_SIZE: usize = WIDTH * HEIGHT / 8;

pub struct Display<I2C> {
    display: Ssd1306Async<
        I2CInterface<I2C>,
        DisplaySize128x32,
        BufferedGraphicsModeAsync<DisplaySize128x32>,
    >,
}

impl<I2C> Display<I2C>
where
    I2C: I2c,
{
    pub fn new(i2c: I2C) -> Self {
        let interface = I2CDisplayInterface::new(i2c);
        let display = Ssd1306Async::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        return Display { display };
    }

    pub async fn init(&mut self) -> Result<(), DisplayError> {
        self.display
            .init()
            .await
            .map_err(|_| DisplayError::InitError)
    }

    pub async fn show_logo(&mut self) -> Result<(), DisplayError> {
        let raw_image = ImageRaw::<BinaryColor>::new(&bmp::LOGO_BITMAP, 120);
        let image = Image::new(&raw_image, Point::new(8, 0));
        image
            .draw(&mut self.display)
            .map_err(|_| DisplayError::DrawError)?;
        self.display
            .flush()
            .await
            .map_err(|_| DisplayError::FlushError)
    }

    pub async fn show_center_text(&mut self, text: &str) -> Result<(), DisplayError> {
        self.display
            .clear(BinaryColor::Off)
            .map_err(|_| DisplayError::ClearError)?;
        Text::with_alignment(
            text,
            Point::new(64, 20),
            MonoTextStyle::new(&FONT_9X18, BinaryColor::On),
            Alignment::Center,
        )
        .draw(&mut self.display)
        .map_err(|_| DisplayError::DrawError)?;
        self.display
            .flush()
            .await
            .map_err(|_| DisplayError::FlushError)
    }

    pub async fn show_text_at(&mut self, text: &str, x: i32, y: i32) -> Result<(), DisplayError> {
        self.display
            .clear(BinaryColor::Off)
            .map_err(|_| DisplayError::ClearError)?;
        Text::with_baseline(
            text,
            Point::new(x, y),
            MonoTextStyle::new(&FONT_9X18, BinaryColor::On),
            Baseline::Top,
        )
        .draw(&mut self.display)
        .map_err(|_| DisplayError::DrawError)?;
        self.display
            .flush()
            .await
            .map_err(|_| DisplayError::FlushError)
    }

    pub async fn clear(&mut self) -> Result<(), DisplayError> {
        self.display
            .clear(BinaryColor::Off)
            .map_err(|_| DisplayError::ClearError)?;
        self.display
            .flush()
            .await
            .map_err(|_| DisplayError::FlushError)
    }
}

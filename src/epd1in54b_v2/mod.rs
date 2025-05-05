//! A simple Driver for the Waveshare 1.54" (B) v2 E-Ink Display via SPI

use embedded_hal::{delay::*, digital::*, spi::SpiDevice};

use crate::interface::DisplayInterface;
use crate::traits::{
    InternalWiAdditions, RefreshLut, WaveshareDisplay, WaveshareThreeColorDisplay,
};

//The Lookup Tables for the Display
mod constants;
use crate::epd1in54b_v2::constants::*;

/// Width of epd1in54 in pixels
pub const WIDTH: u32 = 200;
/// Height of epd1in54 in pixels
pub const HEIGHT: u32 = 200;
/// Default Background Color (white)
pub const DEFAULT_BACKGROUND_COLOR: TriColor = TriColor::White;
const IS_BUSY_LOW: bool = false;
const SINGLE_BYTE_WRITE: bool = true;

use crate::color::TriColor;

use crate::type_a::command::Command;
use crate::buffer_len;

/// Full size buffer for use with the 1in54b EPD
#[cfg(feature = "graphics")]
pub type Display1in54b = crate::graphics::Display<
    WIDTH,
    HEIGHT,
    false,
    { buffer_len(WIDTH as usize, 2 * HEIGHT as usize) },
    TriColor,
>;

/// Epd1in54b driver
pub struct Epd1in54b<SPI, BUSY, DC, RST, DELAY> {
    interface: DisplayInterface<SPI, BUSY, DC, RST, DELAY, SINGLE_BYTE_WRITE>,
    background_color: TriColor,
}

impl<SPI, BUSY, DC, RST, DELAY> InternalWiAdditions<SPI, BUSY, DC, RST, DELAY>
    for Epd1in54b<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    fn init(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {

        #[cfg(feature = "log")]
        log::debug!("Init Epd1in54b");

        self.interface.reset(delay, 10_000, 10_000);
        #[cfg(feature = "log")]
        log::debug!("Reset done");
        self.wait_until_idle(spi, delay)?;

        #[cfg(feature = "log")]
        log::debug!("Wait until idle done");
        self.command(spi, Command::SwReset)?;

        #[cfg(feature = "log")]
        log::debug!("SwReset done");
        self.wait_until_idle(spi, delay)?;

        // 3 Databytes:
        // A[7:0]
        // 0.. A[8]
        // 0.. B[2:0]
        // Default Values: A = Height of Screen (0x127), B = 0x00 (GD, SM and TB=0?)
        self.cmd_with_data(
            spi,
            Command::DriverOutputControl,
            &[(HEIGHT - 1) as u8, 0x00, 0x00],
        )?; 

        self
            .cmd_with_data(spi, Command::DataEntryModeSetting, &[0x3])?;

        self.set_ram_area(spi, delay, 0, 0, WIDTH - 1, HEIGHT - 1)?;

        self.cmd_with_data(spi, Command::BorderWaveformControl, &[0x05])?;

        self.cmd_with_data(
            spi,
            Command::TemperatureSensorSelection,
            &[0x80], // 0x80: internal temperature sensor
        )?;

        self
            .cmd_with_data(spi, Command::TemperatureSensorControl, &[0xB1, 0x20])?;

        // load waveform LUT


        self.set_ram_counter(spi, delay, 0, 0)?;

        self.wait_until_idle(spi, delay)?;


        Ok(())
    }
}

impl<SPI, BUSY, DC, RST, DELAY> WaveshareThreeColorDisplay<SPI, BUSY, DC, RST, DELAY>
    for Epd1in54b<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    fn update_color_frame(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        black: &[u8],
        chromatic: &[u8],
    ) -> Result<(), SPI::Error> {
        self.update_achromatic_frame(spi, delay, black)?;
        self.update_chromatic_frame(spi, delay, chromatic)
    }

    fn update_achromatic_frame(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        black: &[u8],
    ) -> Result<(), SPI::Error> {

        self.wait_until_idle(spi, delay)?;
        self.use_full_frame(spi, delay)?;
        self.cmd_with_data(spi, Command::WriteRam, black)?;

        Ok(())
    }

    fn update_chromatic_frame(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        chromatic: &[u8],
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.use_full_frame(spi, delay)?;
        self.cmd_with_data(spi, Command::WriteRam2, chromatic)?;

        Ok(())
    }
}

impl<SPI, BUSY, DC, RST, DELAY> WaveshareDisplay<SPI, BUSY, DC, RST, DELAY>
    for Epd1in54b<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    type DisplayColor = TriColor;
    fn new(
        spi: &mut SPI,
        busy: BUSY,
        dc: DC,
        rst: RST,
        delay: &mut DELAY,
        delay_us: Option<u32>,
    ) -> Result<Self, SPI::Error> {
        #[cfg(feature = "log")]
        log::debug!("Create epd1in54b_v2 instance");

        let interface = DisplayInterface::new(busy, dc, rst, delay_us);
        let color = DEFAULT_BACKGROUND_COLOR;

        let mut epd = Epd1in54b { interface, background_color: color };

        epd.init(spi, delay)?;

        Ok(epd)
    }

    fn sleep(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.interface
            .cmd_with_data(spi, Command::DeepSleepMode, &[0x01])?;
        Ok(())
    }

    fn wake_up(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.init(spi, delay)
    }

    fn set_background_color(&mut self, color: TriColor) {
        self.background_color = color;
    }

    fn background_color(&self) -> &TriColor {
        &self.background_color
    }

    fn width(&self) -> u32 {
        WIDTH
    }

    fn height(&self) -> u32 {
        HEIGHT
    }

    fn update_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.use_full_frame(spi, delay)?;
        self.cmd_with_data(spi, Command::WriteRam, buffer)?;
        Ok(())
    }

    #[allow(unused)]
    fn update_partial_frame(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        buffer: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), SPI::Error> {
        unimplemented!()
    }

    fn display_frame(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {

        self.wait_until_idle(spi, delay)?;

        //F7 which is also TEMP is used by the arduino version, C7 was used by the rust version
        self.cmd_with_data(spi, Command::DisplayUpdateControl2, &[0xF7])?;

        self.command(spi, Command::MasterActivation)?;
        // MASTER Activation should not be interupted to avoid corruption of panel images
        // therefore a terminate command is send
        self.command(spi, Command::Nop)?;
        Ok(())
    }

    fn update_and_display_frame(
        &mut self,
        spi: &mut SPI,
        buffer: &[u8],
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        self.update_frame(spi, buffer, delay)?;
        self.display_frame(spi, delay)?;
        Ok(())
    }

    fn clear_frame(&mut self, spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        self.use_full_frame(spi, delay)?;

        // clear the ram with the background color
        let color: u8 = self.background_color().get_byte_value();

        if self.background_color() == &TriColor::Chromatic {
            // black and red
            self.command(spi, Command::WriteRam)?;
            self.interface
                .data_x_times(spi, 0xff, 2* (WIDTH / 8 * HEIGHT))?;
            // red and white
            self.command(spi, Command::WriteRam2)?;
            self.interface
                .data_x_times(spi, 0x00, WIDTH / 8 * HEIGHT)?;
        } else {
            // black and white
            self.command(spi, Command::WriteRam)?;
            self.interface
                .data_x_times(spi, color, WIDTH / 8 * HEIGHT)?;
            // red and white
            self.command(spi, Command::WriteRam2)?;
            self.interface
                .data_x_times(spi, 0x00, WIDTH / 8 * HEIGHT)?;
        }

        Ok(())
    }


    fn set_lut(
        &mut self,
        _spi: &mut SPI,
        _delay: &mut DELAY,
        _refresh_rate: Option<RefreshLut>,
    ) -> Result<(), SPI::Error> {
        unimplemented!();
    }

    fn wait_until_idle(&mut self, _spi: &mut SPI, delay: &mut DELAY) -> Result<(), SPI::Error> {
        self.interface.wait_until_idle(delay, IS_BUSY_LOW);
        Ok(())
    }
}

impl<SPI, BUSY, DC, RST, DELAY> Epd1in54b<SPI, BUSY, DC, RST, DELAY>
where
    SPI: SpiDevice,
    BUSY: InputPin,
    DC: OutputPin,
    RST: OutputPin,
    DELAY: DelayNs,
{
    fn command(&mut self, spi: &mut SPI, command: Command) -> Result<(), SPI::Error> {
        self.interface.cmd(spi, command)
    }

    fn send_data(&mut self, spi: &mut SPI, data: &[u8]) -> Result<(), SPI::Error> {
        self.interface.data(spi, data)
    }

    fn cmd_with_data(
        &mut self,
        spi: &mut SPI,
        command: Command,
        data: &[u8],
    ) -> Result<(), SPI::Error> {
        self.interface.cmd_with_data(spi, command, data)
    }

    pub(crate) fn use_full_frame(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
    ) -> Result<(), SPI::Error> {
        // choose full frame/ram
        self.set_ram_area(spi, delay, 0, 0, WIDTH - 1, HEIGHT - 1)?;

        // start from the beginning
        self.set_ram_counter(spi, delay, 0, 0)
    }

    pub(crate) fn set_ram_area(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        start_x: u32,
        start_y: u32,
        end_x: u32,
        end_y: u32,
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        assert!(start_x < end_x);
        assert!(start_y < end_y);

        // x is positioned in bytes, so the last 3 bits which show the position inside a byte in the ram
        // aren't relevant
        self.interface.cmd_with_data(
            spi,
            Command::SetRamXAddressStartEndPosition,
            &[(start_x >> 3) as u8, (end_x >> 3) as u8],
        )?;

        // 2 Databytes: A[7:0] & 0..A[8] for each - start and end
        self.interface.cmd_with_data(
            spi,
            Command::SetRamYAddressStartEndPosition,
            &[
                start_y as u8,
                (start_y >> 8) as u8,
                end_y as u8,
                (end_y >> 8) as u8,
            ],
        )?;
        Ok(())
    }

    pub(crate) fn set_ram_counter(
        &mut self,
        spi: &mut SPI,
        delay: &mut DELAY,
        x: u32,
        y: u32,
    ) -> Result<(), SPI::Error> {
        self.wait_until_idle(spi, delay)?;
        // x is positioned in bytes, so the last 3 bits which show the position inside a byte in the ram
        // aren't relevant
        self
            .cmd_with_data(spi, Command::SetRamXAddressCounter, &[(x >> 3) as u8])?;

        // 2 Databytes: A[7:0] & 0..A[8]
        self.cmd_with_data(
            spi,
            Command::SetRamYAddressCounter,
            &[y as u8, (y >> 8) as u8],
        )?;
        Ok(())
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epd_size() {
        assert_eq!(WIDTH, 200);
        assert_eq!(HEIGHT, 200);
        assert_eq!(DEFAULT_BACKGROUND_COLOR, TriColor::White);
    }
}

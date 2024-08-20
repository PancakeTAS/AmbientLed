use std::{error::Error, path::PathBuf};

use serial2::SerialPort;

///
/// Serial device
///
pub struct Device {
    port: SerialPort,
    buffer: Vec<u8>,
    lengths: Vec<u16>
}

impl Device {

    ///
    /// Create a new serial device
    ///
    /// # Arguments
    ///
    /// * `port` - Path to the serial port
    /// * `baud_rate` - Baud rate
    /// * `lengths` - Amount of leds per strip connected to this device
    ///
    pub fn new(port: &PathBuf, baud_rate: u32, lengths: Vec<u16>) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            port: SerialPort::open(port, baud_rate)?,
            buffer: vec![0; lengths.iter().sum::<u16>() as usize * 3],
            lengths,
        })
    }

    ///
    /// Get a mutable reference to a subarray of the buffer
    ///
    /// # Arguments
    ///
    /// * `strip` - Strip index
    /// * `offset` - Offset in the strip
    /// * `length - Length of the subarray
    ///
    pub(super) fn get_mut(&mut self, strip: u8, offset: u16, length: u16) -> &mut [u8] {
        let start = self.lengths.iter().take(strip as usize).sum::<u16>() as usize * 3 + offset as usize * 3;
        &mut self.buffer[start..start + length as usize * 3]
    }

    ///
    /// Write the data to the serial port
    ///
    pub(super) fn write(&self) -> Result<(), Box<dyn Error>> {
        self.port.write_all(&self.buffer)?;
        self.port.flush()?;
        Ok(())
    }

}

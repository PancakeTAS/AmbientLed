use std::collections::HashMap;

use anyhow::Context;
use device::Device;
use log::trace;
use strip::Strip;

pub mod device;
pub mod strip;

///
/// Connector between multiple devices and virtual led strips
pub struct Connector {
    devices: HashMap<u64, Device>,
    strips: HashMap<u64, Strip>,
}

impl Connector {

    ///
    /// Create a new connector
    ///
    pub fn new() -> Self {
        Self { devices: HashMap::new(), strips: HashMap::new() }
    }

    ///
    /// Add a new device to the connector
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the device
    /// * `device` - Device
    ///
    pub fn set_device(&mut self, id: u64, device: Device) {
        self.devices.insert(id, device);
    }

    ///
    /// Add a new strip to the connector
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the strip
    /// * `strip` - Strip
    ///
    pub fn set_strip(&mut self, id: u64, strip: Strip) {
        self.strips.insert(id, strip);
    }

    ///
    /// Get a mutable reference to a strip
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the strip
    ///
    /// # Errors
    ///
    /// This function returns an error if the strip is not found
    ///
    pub fn mutate_strip(&mut self, id: u64) -> Result<&mut [u8], anyhow::Error> {
        Ok(self.strips.get_mut(&id).context("strip not found")?.get_mut())
    }

    ///
    /// Write all strips to the devices
    ///
    /// # Errors
    ///
    /// This function returns an error if any of the strips or devices fail to write
    ///
    pub fn write(&mut self) -> Result<(), anyhow::Error> {
        // copy all virtual led strips to the devices
        for (id, strip) in &self.strips {
            strip.write(&mut self.devices).context("failed to write strip")?;
            trace!("copied virtual strip {} to physical strips", id);
        }

        // write all devices
        for (id, device) in &mut self.devices {
            device.write().context("failed to write device")?;
            trace!("wrote device {}", id);
        }

        trace!("finished writing all devices");
        Ok(())
    }

    ///
    /// Reset the connector
    ///
    pub fn reset(&mut self) {
        self.devices.clear();
        self.strips.clear();
    }

}
use std::collections::HashMap;

use device::Device;
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
    pub fn add_device(&mut self, id: u64, device: Device) {
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
    pub fn add_strip(&mut self, id: u64, strip: Strip) {
        self.strips.insert(id, strip);
    }

    ///
    /// Get a mutable reference to a strip
    ///
    /// # Arguments
    ///
    /// * `id` - Id of the strip
    ///
    pub fn mutate_strip(&mut self, id: u64) -> Result<&mut [u8], Box<dyn std::error::Error>> {
        Ok(self.strips.get_mut(&id).ok_or("strip not found")?.get_mut())
    }

    ///
    /// Write all strips to the devices
    ///
    pub fn write(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // copy all virtual led strips to the devices
        for strip in self.strips.values_mut() {
            strip.write(&mut self.devices)?;
        }

        // write all devices
        for device in self.devices.values_mut() {
            device.write()?;
        }

        Ok(())
    }

}
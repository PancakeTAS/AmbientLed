use std::collections::HashMap;

use anyhow::Context;
use log::debug;

use super::device::Device;

///
/// LED strip mapping
///
pub struct Mapping {
    device_id: u64,
    strip_id: u8,
    offset: u16, // where on the physical strip this segment starts
    length: u16
}

impl Mapping {
    pub fn new(device_id: u64, strip_id: u8, offset: u16, length: u16) -> Self {
        Self { device_id, strip_id, offset, length }
    }
}

///
/// Virtual LED strip consisting of multiple physical LED strips
///
pub struct Strip {
    mappings: Vec<Mapping>,
    buffer: Vec<u8>
}

impl Strip {

    ///
    /// Create a new virtual LED strip
    ///
    /// # Arguments
    ///
    /// * `length` - Length of the strip
    ///
    pub fn new(length: u16) -> Self {
        Self {
            mappings: Vec::new(),
            buffer: vec![0; length as usize * 3]
        }
    }

    ///
    /// Map a part of the group to a strip (call this method in order)
    ///
    /// Inverting the direction of a strip is not supported and should be done in the renderer
    ///
    /// # Arguments
    ///
    /// * `mapping` - Mapping of the strip
    ///
    pub fn map(&mut self, mapping: Mapping) {
        self.mappings.push(mapping);
    }

    ///
    /// Get a mutable reference to the buffer
    ///
    pub fn get_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }

    ///
    /// Copy the buffer to the devices (this does NOT write the buffer to the devices)
    ///
    /// # Arguments
    ///
    /// * `device_map` - Mapping of device IDs to devices
    ///
    /// # Errors
    ///
    /// This function returns an error if a device could not be found in the device map
    ///
    pub(super) fn write(&self, device_map: &mut HashMap<u64, Device>) -> Result<(), anyhow::Error> {
        let mut offset = 0;
        for mapping in &self.mappings {
            let device = device_map.get_mut(&mapping.device_id).context("device not found in device id map")?;
            let buffer_slice = device.get_mut(mapping.strip_id, mapping.offset, mapping.length);
            buffer_slice.copy_from_slice(&self.buffer[offset..offset + mapping.length as usize * 3]);
            offset += mapping.length as usize * 3;
        }

        Ok(())
    }

}

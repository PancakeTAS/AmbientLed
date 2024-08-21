use std::{fs::File, io::BufReader, path::PathBuf};

use anyhow::Context;
use serde::Deserialize;

///
/// The configuration of the application
///
#[derive(Deserialize)]
pub struct Configuration {
    /// Settings for the connector
    pub connector: Connector,
    /// Settings for screencopy
    pub screencopy: Screencopy,
    /// Settings for the render pipeline
    pub render_pipeline: RenderPipeline
}

impl Configuration {

    ///
    /// Loads the configuration from the specified file
    ///
    /// # Arguments
    ///
    /// * `path` - The path to the configuration file
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be opened or the configuration cannot be parsed
    ///
    pub fn new(path: &PathBuf) -> Result<Self, anyhow::Error> {
        let file = File::open(path).context("failed to open the configuration file")?;
        let config = serde_yml::from_reader(BufReader::new(file)).context("failed to parse the configuration file")?;
        Ok(config)
    }

}

// ====== Connector ======

///
/// The configuration of the connector
///
#[derive(Deserialize)]
pub struct Connector {
    /// List of physical devices
    pub devices: Vec<Device>,
    /// List of virtual strips
    pub strips: Vec<Strip>
}

///
/// The configuration of a physical device
///
/// A physical device is an arduino or similar microcontroller that controls a set of physical led strips
///
#[derive(Deserialize)]
pub struct Device {
    /// The unique identifier of the device
    pub id: u64,
    /// The port the device is connected to, e.g. COM3 or /dev/ttyACM0
    pub port: String,
    /// The baud rate of the serial connection (ensure that the device is configured to use the same baud rate)
    pub baud_rate: u32,
    /// The list of physical strips connected to the device
    pub physical_strips: Vec<PhysicalStrip>
}

///
/// The configuration of a physical led strip
///
/// A physical led strip is a strip of leds connected to a physical device. Any number of virtual strips can be mapped this physical strip.
///
#[derive(Deserialize)]
pub struct PhysicalStrip {
    /// The amount of leds on the strip, this is required to calculate the buffer size of the serial connection
    pub leds: u16
}

///
/// The configuration of a virtual led strip
///
/// A virtual led strip is a strip of leds that is mapped to one or multiple physical led strips. The mapping specifies the offset and length of the physical strips and is applied in the order of the mappings.
///
#[derive(Deserialize)]
pub struct Strip {
    /// The unique identifier of the strip
    pub id: u64,
    /// The amount of leds on the virtual strip
    pub leds: u16,
    /// The list of mappings to physical strips, ensure that the total length of the mappings equals the amount of leds
    pub mappings: Vec<Mapping>
}

///
/// The mapping of a virtual strip to a physical strip
///
/// The mapping specifies the offset and length of a physical strip on a specific device.
///
#[derive(Deserialize)]
pub struct Mapping {
    /// The unique identifier of the device the physical strip is connected to
    pub device_id: u64,
    /// The unique identifier of the physical strip on the device
    pub physical_strip_idx: u8,
    /// The offset of the physical strip on the virtual strip
    pub offset: u16,
    /// The length of the physical strip on the virtual strip
    pub length: u16
}

// ====== Screencopy ======

///
/// The configuration of screencopy
///
#[derive(Deserialize)]
pub struct Screencopy {
    /// The path to the gbm device that is used for rendering
    pub gbm_device: String,
    /// The list of capture sessions
    pub capture_sessions: Vec<CaptureSession>
}

///
/// The configuration of a capture session
///
/// A capture session specifies the region of an output that is captured and imported into the render pipeline as a texture
///
#[derive(Deserialize)]
pub struct CaptureSession {
    /// The unique identifier of the capture session
    pub id: u64,
    /// The output that is captured (this will first attempt to find a matching output by name, if no output is found it will attempt to find a matching output by description if it is provided)
    pub output: String,
    /// The region of the output that is captured
    pub region: Region
}

///
/// The region of an output that is captured.
///
/// Please keep in mind that these are local to the output and also virtual coordinates, so scaling applies.
///
#[derive(Deserialize)]
pub struct Region {
    /// The left coordinate of the region
    pub left: i32,
    /// The top coordinate of the region
    pub top: i32,
    /// The width of the region
    pub width: i32,
    /// The height of the region
    pub height: i32
}

// ====== Render Pipeline ======

///
/// The configuration of the render pipeline
///
#[derive(Deserialize)]
pub struct RenderPipeline {
    /// The list of programs
    pub programs: Vec<Program>
}

///
/// The configuration of a program
///
/// A program specifies the shaders that are used to render a specific strip. You can specify multiple capture sessions that are used as textures in the shaders.
///
#[derive(Deserialize)]
pub struct Program {
    /// The unique identifier of the program
    pub id: u64,
    /// Path to the fragment shader
    pub fragment_shader: String,
    /// Path to the vertex shader
    pub vertex_shader: String,
    /// List of capture sessions that are used as textures in the shaders
    pub capture_sessions: Vec<u64>,
    /// The unique identifier of the strip that is rendered by this program
    pub strip_id: u64
}

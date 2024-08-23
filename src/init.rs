use std::{path::PathBuf, str::FromStr};

use anyhow::{anyhow, Context};
use log::LevelFilter;
use log::{info, warn};

use crate::configuration;
use crate::connector;
use crate::renderer;
use crate::screencopy;

///
/// (Re)creates a capture session.
///
/// # Arguments
///
/// * `session` - The configuration of the capture session.
/// * `screencopy` - The screencopy instance.
/// * `render_pipeline` - The render pipeline instance.
///
/// # Errors
///
/// Returns an error if the output is not found, the capture session cannot be created or the texture cannot be set.
///
fn recreate_capture_session(session: &configuration::CaptureSession, screencopy: &mut screencopy::Screencopy, render_pipeline: &mut renderer::RenderPipeline) -> Result<(), anyhow::Error> {
    // try to find the output by name, then by description
    let output = screencopy.outputs.iter().find(
        |output| output.1.name.as_deref().unwrap_or("") == session.output
    ).or_else(|| {
        screencopy.outputs.iter().find(
            |output| session.output.contains(output.1.description.as_deref().unwrap_or(""))
        )
    }).context("output not found")?;

    // create the capture session
    let capture_session = screencopy::CaptureSession::new(
        output.0.clone(),
        session.region.left,
        session.region.top,
        session.region.width,
        session.region.height
    );
    let bo = screencopy.set_capture_session(session.id, capture_session).context("failed to create capture session")?;

    // set the render texture
    render_pipeline.set_texture(session.id, bo).context("failed to set texture")?;

    Ok(())
}

///
/// (Re)creates a device.
///
/// # Arguments
///
/// * `device` - The configuration of the device.
/// * `connector` - The connector instance.
///
/// # Errors
///
/// Returns an error if the device cannot be created.
///
fn recreate_devices(device: &configuration::Device, connector: &mut connector::Connector) -> Result<(), anyhow::Error> {
    connector.set_device(
        device.id,
        connector::device::Device::new(
            &PathBuf::from(device.port.clone()),
            device.baud_rate,
            device.physical_strips.iter().map(|strip| strip.leds).collect()
        ).context("failed to create device")?
    );

    Ok(())
}

///
/// (Re)creates a program.
///
/// # Arguments
///
/// * `program` - The configuration of the program.
/// * `config` - The configuration.
///
/// # Errors
///
/// Returns an error if the program cannot be created.
///
fn recreate_program(config_dir: &PathBuf, program: &configuration::Program, config: &configuration::Configuration, render_pipeline: &mut renderer::RenderPipeline) -> Result<(), anyhow::Error> {
    let vertex_shader = config_dir.join(&program.vertex_shader);
    let fragment_shader = config_dir.join(&program.fragment_shader);
    render_pipeline.set_shader(
        program.id,
        &program.capture_sessions,
        config.connector.strips.iter().find(|strip| strip.id == program.strip_id).context("strip not found")?.leds as u32, 1,
        &vertex_shader,
        &fragment_shader
    ).context("failed to set shader")?;

    Ok(())
}

///
/// (Re)creates a strip.
///
/// # Arguments
///
/// * `strip` - The configuration of the strip.
/// * `connector` - The connector instance.
///
/// # Errors
///
/// Returns an error if the strip cannot be created.
///
fn recreate_strip(strip: &configuration::Strip, connector: &mut connector::Connector) {
    let mut connector_strip = connector::strip::Strip::new(strip.leds);
    for mapping in &strip.mappings {
        info!("mapping physical strip {} on device {} at offset {} with length {}", mapping.physical_strip_idx, mapping.device_id, mapping.offset, mapping.length);
        connector_strip.map(connector::strip::Mapping::new(mapping.device_id, mapping.physical_strip_idx, mapping.offset, mapping.length));
    }
    connector.set_strip(strip.id, connector_strip);
}

pub fn init(verbose: bool, frames: Option<&u32>, config: Option<&PathBuf>) -> Result<(), anyhow::Error> {
    // find .config directory
    let config_dir = dirs::config_dir().unwrap().join("ambient-led");
    let default_config = config_dir.join("config.yml");
    let config_file = config.unwrap_or(&default_config);
    if !config_file.exists() {
        return Err(anyhow!("specified configuration file does not exist"));
    }

    // parse the configuration file
    let config = configuration::Configuration::new(config_file).context("failed to parse configuration file")?;

    // initialize the logger
    let level = if verbose { LevelFilter::Trace } else { log::LevelFilter::from_str(&config.log_level).context("invalid log level")? };
    colog::default_builder()
        .filter_level(level)
        .init();

    // initialize constructs
    let mut screencopy = screencopy::Screencopy::new(config.screencopy.gbm_device.clone())?;
    let mut connector = connector::Connector::new();
    let mut render_pipeline = renderer::RenderPipeline::new(screencopy.get_display_id())?;

    // add devices
    info!("adding devices to connector");
    for device in &config.connector.devices {
        info!("adding device {} on {} @{}Bd with {} physical strips", device.id, device.port, device.baud_rate, device.physical_strips.len());
        recreate_devices(device, &mut connector).context("failed to create device, panicking")?;
    }

    // add strips
    info!("adding strips to connector");
    for strip in &config.connector.strips {
        info!("adding strip {} with {} leds consisting of {} mapping(s)", strip.id, strip.leds, strip.mappings.len());
        recreate_strip(strip, &mut connector);
    }

    // add capture sessions
    info!("creating capture sessions");
    for session in &config.screencopy.capture_sessions {
        info!("creating capture session {} for output {} at {}, {} with size {}x{}", session.id, session.output, session.region.left, session.region.top, session.region.width, session.region.height);
        recreate_capture_session(session, &mut screencopy, &mut render_pipeline).context("failed to recreate capture session, panicking")?;
    }

    // add programs
    info!("adding programs to render pipeline");
    for program in &config.render_pipeline.programs {
        info!("adding program {} with {} textures from {} and {} to {}", program.id, program.capture_sessions.len(), program.vertex_shader, program.fragment_shader, program.strip_id);
        recreate_program(&config_dir, program, &config, &mut render_pipeline).context("failed to recreate program, panicking")?;
    }

    // prepare optional frame limit
    let mut captured_frames = 0;

    // start the render loop
    let frame_time = std::time::Duration::from_secs_f32(1.0 / (config.fps as f32));
    info!("starting render loop with {} fps", config.fps);
    loop {
        let start = std::time::Instant::now();

        // capture the screens
        for session in &config.screencopy.capture_sessions {
            let status = screencopy.capture(session.id);
            if status.is_err() {
                warn!("failed to capture session {}: {:?}", session.id, status);
                recreate_capture_session(session, &mut screencopy, &mut render_pipeline).context("failed to recreate capture session, panicking")?;
                info!("recreated capture session {}", session.id);
            }
        }

        // render the strips
        for program in &config.render_pipeline.programs {
            render_pipeline.render(program.id, connector.mutate_strip(program.strip_id).unwrap());
        }

        // send the data to the devices
        let status = connector.write();
        if status.is_err() {
            warn!("failed to write to devices: {:?}", status);
            for device in &config.connector.devices {
                recreate_devices(device, &mut connector).context("failed to recreate device, panicking")?;
            }
            info!("recreated devices");
        }

        let elapsed = start.elapsed();
        if elapsed < frame_time {
            std::thread::sleep(frame_time - elapsed);
        }

        // check if the frame limit is reached
        if let Some(frames) = frames {
            captured_frames += 1;
            if captured_frames >= *frames {
                info!("captured {} frames, exiting", frames);
                return Ok(());
            }
        }
    }
}
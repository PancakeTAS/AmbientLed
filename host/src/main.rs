use std::{collections::HashMap, path::PathBuf};

use anyhow::Context;
use log::info;

mod connector;
mod renderer;
mod screencopy;
mod configuration;

fn main() -> Result<(), anyhow::Error> {
    // initialize the logger
    colog::default_builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    // parse the configuration file
    let config = configuration::Configuration::new(&PathBuf::from("config.example.yml"))?;

    // initialize constructs
    let mut screencopy = screencopy::Screencopy::new()?; // TODO: add gbm device
    let mut connector = connector::Connector::new();
    let mut render_pipeline = renderer::RenderPipeline::new(screencopy.get_display_id())?;

    // add devices
    info!("adding devices to connector");
    for device in &config.connector.devices {
        info!("adding device {} on {} @{}Bd with {} physical strips", device.id, device.port, device.baud_rate, device.physical_strips.len());
        connector.add_device(
            device.id,
            connector::device::Device::new(
                &PathBuf::from(device.port.clone()),
                device.baud_rate,
                device.physical_strips.iter().map(|strip| strip.leds).collect()
            ).context("failed to create device")?
        );
    }

    // add strips
    info!("adding strips to connector");
    for strip in &config.connector.strips {
        info!("adding strip {} with {} leds consisting of {} mapping(s)", strip.id, strip.leds, strip.mappings.len());
        let mut connector_strip = connector::strip::Strip::new(strip.leds);
        for mapping in &strip.mappings {
            info!("mapping physical strip {} on device {} at offset {} with length {}", mapping.physical_strip_idx, mapping.device_id, mapping.offset, mapping.length);
            connector_strip.map(connector::strip::Mapping::new(mapping.device_id, mapping.physical_strip_idx, mapping.offset, mapping.length));
        }
        connector.add_strip(strip.id, connector_strip);
    }

    // add capture sessions
    info!("creating capture sessions");
    let mut sessions = HashMap::<u64, screencopy::CaptureSession>::new(); // FIXME: would be nice to store this in screencopy
    for session in &config.screencopy.capture_sessions {
        info!("creating capture session {} for output {} at {}, {} with size {}x{}", session.id, session.output, session.region.left, session.region.top, session.region.width, session.region.height);

        // try to find the output by name, then by description
        let output = screencopy.outputs.iter().find(
            |output| output.1.name.as_deref().unwrap_or("") == session.output
        ).or_else(|| {
            screencopy.outputs.iter().find(
                |output| session.output.contains(output.1.description.as_deref().unwrap_or(""))
            )
        }).context("output not found")?;

        // create the capture session
        let mut capture_session = screencopy::CaptureSession::new(
            output.0.clone(),
            session.region.left,
            session.region.top,
            session.region.width,
            session.region.height
        );
        screencopy.capture(&mut capture_session)?;

        // set the render texture
        render_pipeline.set_texture(session.id, capture_session.get_dmabuf().unwrap())?;

        sessions.insert(session.id, capture_session);
    }

    // add programs
    info!("adding programs to render pipeline");
    for program in &config.render_pipeline.programs {
        info!("adding program {} with {} textures from {} and {} to {}", program.id, program.capture_sessions.len(), program.vertex_shader, program.fragment_shader, program.strip_id);
        render_pipeline.set_shader(
            program.id,
            &program.capture_sessions,
            config.connector.strips.iter().find(|strip| strip.id == program.strip_id).context("strip not found")?.leds as u32, 1,
            &PathBuf::from(program.vertex_shader.clone()),
            &PathBuf::from(program.fragment_shader.clone())
        )?;
    }

    // start the render loop
    // FIXME: don't hardcode 30 fps
    let frame_time = std::time::Duration::from_secs_f32(1.0 / 30.0);
    loop {
        let start = std::time::Instant::now();

        // capture the screen
        for session in sessions.values_mut() {
            screencopy.capture(session)?;
        }

        // render the strips
        for program in &config.render_pipeline.programs {
            render_pipeline.render(program.id, connector.mutate_strip(program.strip_id)?);
        }

        // send the data to the devices
        connector.write()?;

        let elapsed = start.elapsed();
        if elapsed < frame_time {
            std::thread::sleep(frame_time - elapsed);
        }
    }
}
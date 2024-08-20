use std::{path::PathBuf, time::Duration};

mod screencopy;
mod renderer;
mod connector;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    colog::init();

    // initialize the connector
    let mut connector = connector::Connector::new();

    // create a device with two physical led strips with lengths 88 and 91
    let leonardo = connector::device::Device::new(&PathBuf::from("/dev/ttyACM0"), 1000000, vec![88, 91])?;
    connector.add_device(1, leonardo);

    // create a virtual led strip for the top strip
    let strip1_len = 88;
    let mut strip1 = connector::strip::Strip::new(strip1_len);
    strip1.map(connector::strip::Mapping::new(1, 0, 0, strip1_len));
    connector.add_strip(1, strip1);

    // create the virtual led strip for the bottom strip
    let strip2_len = 17 + 48 + 16;
    let mut strip2 = connector::strip::Strip::new(strip2_len);
    strip2.map(connector::strip::Mapping::new(1, 1, 0, 17));
    strip2.map(connector::strip::Mapping::new(1, 1, 22, 48));
    strip2.map(connector::strip::Mapping::new(1, 1, 75, 16));
    connector.add_strip(2, strip2);

    // initialize the screencopy
    let mut client = screencopy::Client::new()?;
    let primary_output = client.outputs.iter().find(|output| output.1.name.as_ref().unwrap() == "DP-3").ok_or("No primary output found")?;

    // create the capture sessions
    let scaling_factor = 1.25;
    let mut session1 = screencopy::CaptureSession::new(primary_output.0.clone(), 0, 0, (2560.0 / scaling_factor) as i32, (150.0 / scaling_factor) as i32);
    let mut session2 = screencopy::CaptureSession::new(primary_output.0.clone(), 0, ((1440.0 - 150.0) / scaling_factor) as i32, (2560.0 / scaling_factor) as i32, (150.0 / scaling_factor) as i32);
    client.capture(&mut session1)?;
    client.capture(&mut session2)?;

    // initialize the renderer
    let mut renderer = renderer::RenderPipeline::new(client.get_display_id())?;

    // set the render textures
    renderer.set_texture(1, session1.get_dmabuf().unwrap())?;
    renderer.set_texture(2, session2.get_dmabuf().unwrap())?;

    // set the shader
    renderer.set_shader(1, &[1], strip1_len as u32, 1, &PathBuf::from("shaders/basic_vertex.glsl"), &PathBuf::from("shaders/basic_fragment.glsl"))?;
    renderer.set_shader(2, &[2], strip2_len as u32, 1, &PathBuf::from("shaders/basic_vertex.glsl"), &PathBuf::from("shaders/basic_fragment.glsl"))?;

    // start the render loop
    let frame_time = Duration::from_secs_f32(1.0 / 30.0);
    loop {
        let start = std::time::Instant::now();

        // capture the screen
        client.capture(&mut session1)?;
        client.capture(&mut session2)?;

        // render the strips
        renderer.render(1, connector.mutate_strip(1)?);
        renderer.render(2, connector.mutate_strip(2)?);

        // send the data to the devices
        connector.write()?;

        // sleep for the remaining time
        let elapsed = start.elapsed();
        if elapsed < frame_time {
            std::thread::sleep(frame_time - elapsed);
        }
    }
}

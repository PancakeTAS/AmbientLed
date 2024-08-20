use std::path::PathBuf;

mod screencopy;
mod renderer;
mod connector;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // initialize the connector
    let mut connector = connector::Connector::new();

    // create a device with two physical led strips with lengths 88 and 91
    let leonardo = connector::device::Device::new(&PathBuf::from("/dev/ttyACM0"), 1000000, vec![88, 91])?;
    connector.add_device(1, leonardo);

    // create a virtual led strip for the top strip
    let mut strip1 = connector::strip::Strip::new(88);
    strip1.map(connector::strip::Mapping::new(1, 0, 0, 88));
    connector.add_strip(1, strip1);

    // create the remaining virtual led strip
    let mut strip2 = connector::strip::Strip::new(91);
    strip2.map(connector::strip::Mapping::new(1, 1, 0, 91));
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
    renderer.set_texture(1, session1.get_dmabuf()?)?;
    renderer.set_texture(2, session2.get_dmabuf()?)?;

    // set the shader
    renderer.set_shader(1, &[1], 88, 1, &PathBuf::from("shaders/vertex.glsl"), &PathBuf::from("shaders/fragment.glsl"))?;
    renderer.set_shader(2, &[2], 91, 1, &PathBuf::from("shaders/vertex.glsl"), &PathBuf::from("shaders/fragment.glsl"))?;

    // start the render loop
    loop {
        // capture the screen
        client.capture(&mut session1)?;
        client.capture(&mut session2)?;

        // render the strips
        renderer.render(1, connector.mutate_strip(1)?);
        renderer.render(2, connector.mutate_strip(2)?);

        // send the data to the devices
        connector.write()?;
    }
}

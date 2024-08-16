use std::{path::PathBuf, thread::sleep, time::Instant};

mod screencopy;
mod renderer;
mod pi;
mod util;

fn main() {
    // initialize screencopy
    let mut client = screencopy::Client::new().unwrap();
    let primary = client.outputs.iter().find(|o| o.1.name.as_ref().unwrap() == "DP-1").unwrap().0;
    let piano = client.outputs.iter().find(|o| o.1.name.as_ref().unwrap() == "DP-3").unwrap().0;
    let mut tophalf_session: screencopy::CaptureSession = screencopy::CaptureSession::new(primary.clone(), 0, 0, 1920, 150);
    let mut bottomhalf_session: screencopy::CaptureSession = screencopy::CaptureSession::new(primary.clone(), 0, 1080 - 150, 1920, 150);
    let mut pianoright_session: screencopy::CaptureSession = screencopy::CaptureSession::new(piano.clone(), 3540, 0, 300, 2160);
    let mut pianotop_session: screencopy::CaptureSession = screencopy::CaptureSession::new(piano.clone(), 0, 0, 3840, 180);
    let mut pianoleft_session: screencopy::CaptureSession = screencopy::CaptureSession::new(piano.clone(), 0, 0, 300, 2160);
    client.capture(&mut tophalf_session).unwrap();
    client.capture(&mut bottomhalf_session).unwrap();
    client.capture(&mut pianoright_session).unwrap();
    client.capture(&mut pianotop_session).unwrap();
    client.capture(&mut pianoleft_session).unwrap();

    // initialize render pipeline
    let mut pipeline = renderer::RenderPipeline::new(client.get_display_id(), 144, 1).unwrap();
    pipeline.set_texture(0, tophalf_session.get_dmabuf().unwrap()).unwrap();
    pipeline.set_texture(1, bottomhalf_session.get_dmabuf().unwrap()).unwrap();
    pipeline.set_texture(2, pianoright_session.get_dmabuf().unwrap()).unwrap();
    pipeline.set_texture(3, pianotop_session.get_dmabuf().unwrap()).unwrap();
    pipeline.set_texture(4, pianoleft_session.get_dmabuf().unwrap()).unwrap();
    pipeline.set_shader(0, &[0], &PathBuf::from("shaders/vertex.glsl"), &PathBuf::from("shaders/fragment.glsl")).unwrap();
    pipeline.set_shader(1, &[1], &PathBuf::from("shaders/vertex.glsl"), &PathBuf::from("shaders/fragment.glsl")).unwrap();
    pipeline.set_shader(2, &[2], &PathBuf::from("shaders/vertex.glsl"), &PathBuf::from("shaders/fragment.glsl")).unwrap();
    pipeline.set_shader(3, &[3], &PathBuf::from("shaders/vertex.glsl"), &PathBuf::from("shaders/fragment.glsl")).unwrap();
    pipeline.set_shader(4, &[4], &PathBuf::from("shaders/vertex.glsl"), &PathBuf::from("shaders/fragment.glsl")).unwrap();

    // connect to raspberry pi
    let mut tophalf_conn = pi::connect("10.0.2.10:5163", &pi::Params {
        max_b: 230,
        num_l: 144,
        mul_r: 1.0, mul_g: 1.0, mul_b: 1.0,
        lerp: 0.5, rate: 60
    }).unwrap();
    let mut bottomhalf_conn = pi::connect("10.0.2.12:5164", &pi::Params {
        max_b: 192,
        num_l: 144,
        mul_r: 1.0, mul_g: 1.0, mul_b: 1.0,
        lerp: 0.5, rate: 60
    }).unwrap();
    let mut piano_conn = pi::connect("10.0.2.11:5163", &pi::Params {
        max_b: 136,
        num_l: 180,
        mul_r: 1.0, mul_g: 1.0, mul_b: 1.0,
        lerp: 0.5, rate: 60
    }).unwrap();

    // main loop
    let mut buf = vec![0u8; 144 * 3];
    let frame_duration = std::time::Duration::from_millis(1000 / 30);

    loop {
        let start_time = Instant::now();

        // capture screen
        client.capture(&mut tophalf_session).unwrap();
        client.capture(&mut bottomhalf_session).unwrap();
        client.capture(&mut pianoright_session).unwrap();
        client.capture(&mut pianotop_session).unwrap();
        client.capture(&mut pianoleft_session).unwrap();

        // top half
        pipeline.render(0, &mut buf);
        for i in 0..144 {
            let (r, g, b) = (buf[i * 3], buf[i * 3 + 1], buf[i * 3 + 2]);

            tophalf_conn.colors[i].r = r;
            tophalf_conn.colors[i].g = g;
            tophalf_conn.colors[i].b = b;
        }
        pi::update(&mut tophalf_conn).unwrap();

        // bottom half
        pipeline.render(1, &mut buf);
        for i in 0..144 {
            let (r, g, b) = (buf[i * 3], buf[i * 3 + 1], buf[i * 3 + 2]);

            bottomhalf_conn.colors[i].r = r;
            bottomhalf_conn.colors[i].g = g;
            bottomhalf_conn.colors[i].b = b;
        }
        pi::update(&mut bottomhalf_conn).unwrap();

        // piano
        pipeline.render(2, &mut buf);
        for i in 0..47 {
            let (r, g, b) = (buf[i * 3], buf[i * 3 + 1], buf[i * 3 + 2]);

            piano_conn.colors[i].r = r;
            piano_conn.colors[i].g = g;
            piano_conn.colors[i].b = b;
        }
        pipeline.render(3, &mut buf);
        for i in 0..76 {
            let (r, g, b) = (buf[i * 3], buf[i * 3 + 1], buf[i * 3 + 2]);

            piano_conn.colors[i + 47].r = r;
            piano_conn.colors[i + 47].g = g;
            piano_conn.colors[i + 47].b = b;
        }
        pipeline.render(4, &mut buf);
        for i in 0..57 {
            let (r, g, b) = (buf[i * 3], buf[i * 3 + 1], buf[i * 3 + 2]);

            piano_conn.colors[i + 47 + 76].r = r;
            piano_conn.colors[i + 47 + 76].g = g;
            piano_conn.colors[i + 47 + 76].b = b;
        }
        pi::update(&mut piano_conn).unwrap();

        let elapsed = start_time.elapsed();
        if elapsed < frame_duration {
            sleep(frame_duration - elapsed);
        }
    }
}

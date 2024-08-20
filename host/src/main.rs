use std::path::PathBuf;

mod connector;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // create the connector struct
    let mut connector = connector::Connector::new();

    // create a device with two physical led strips with lengths 88 and 91
    let leonardo = connector::device::Device::new(&PathBuf::from("/dev/ttyACM1"), 1000000, vec![88, 91])?;
    connector.add_device(1, leonardo);

    // create a virtual led strip with length (88+91)-40 containing the center of the two physical led strips
    let mut strip = connector::strip::Strip::new(88+91-40);
    strip.map(connector::strip::Mapping::new(1, 0, 10, 88 - 20));
    strip.map(connector::strip::Mapping::new(1, 1, 10, 91 - 20));
    connector.add_strip(0, strip);

    // create the remaining virtual led strip
    let mut strip = connector::strip::Strip::new(40);
    strip.map(connector::strip::Mapping::new(1, 0, 0, 10));
    strip.map(connector::strip::Mapping::new(1, 0, 88 - 10, 10));
    strip.map(connector::strip::Mapping::new(1, 1, 0, 10));
    strip.map(connector::strip::Mapping::new(1, 1, 91 - 10, 10));
    connector.add_strip(1, strip);

    // fill the first virtual led strip with red
    let data = connector.mutate_strip(0)?;
    for i in (0..data.len()).step_by(3) {
        data[i] = 255;     // Red
        data[i + 1] = 0;   // Green
        data[i + 2] = 0;   // Blue
    }

    // fill the second virtual led strip with green
    let data = connector.mutate_strip(1)?;
    for i in (0..data.len()).step_by(3) {
        data[i] = 0;       // Red
        data[i + 1] = 255; // Green
        data[i + 2] = 0;   // Blue
    }

    // write the data to the devices
    loop {
        connector.write()?;

        std::thread::sleep(std::time::Duration::from_secs(1));
    }

    // create a virtual led strip with length 88+91 mapped to the device
    /*let mut strip = connector::strip::Strip::new(88+91);
    strip.map(connector::strip::Mapping::new(1, 0, 0, 88));
    strip.map(connector::strip::Mapping::new(1, 1, 0, 91));
    connector.add_strip(0, strip);

    // write all white to the virtual led strip
    let data = connector.mutate_strip(0)?;
    for i in (0..data.len()).step_by(3) {
        data[i] = 255;     // Red
        data[i + 1] = 0;   // Green
        data[i + 2] = 0;   // Blue
    }
    connector.write()?;*/

    Ok(())
}

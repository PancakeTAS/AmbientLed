# ambient-led
ambient-led is a small but powerful rust program, that turns your WS2812B LED strip into a monitor backlight.
## Features
By default, ambient-led will continuously sample the screen and pick colors to display on the LED strip. Additionally, it will change the brightness of the LEDs to a logarithmic scale, in order to compensate for darker applications. Each LED strip is connected to an Arduino (or similar) microcontroller, which is connected to the computer via USB.

In addition to these defaults, ambient-led is also capable of:
- Connecting multiple physical LED strips to a single microcontroller, or multiple microcontrollers to a single computer
- Combining or splitting multiple physical LED strips into one or more virtual strips
- Customizing the rendering pipeline with vertex and fragment shaders
- Capturing and combining as many screens as desired
## Compatibility
As of right now, `ambient-led` relies on EGL, which is a linux-only API. It also relies on the ZwlrScreencopyManagerV1 protocol, which is only available on wlroots-based compositors on Wayland. Ensure it's compatible with your compositor by checking [this page](https://wayland.app/protocols/wlr-screencopy-unstable-v1).

Support for general Wayland compositors is planned, however the screencopy protocol was only merged into the main Wayland repository a few weeks ago and hasn't been implemented in any other compositor yet.

Support for X11 is tricky, due to X11's lack of a fast and efficient way to capture the screen. The options are XSHM, which is incredibly slow and Xcomposite, which can only capture individual windows. NvFBC however support is planned, which will allow for fast and efficient screen capture on X11 when using NVIDIA GPUs (using a [patched driver](https://github.com/keylase/nvidia-patch)).

Support for Windows is also planned, however it's not a priority at the moment.

Support for Raspberry Pis as LED controllers was dropped in version 3.x due to lack of consistent and reliable connection to the host however might be re-added in the future.
## Hardware
ambient-led was primarily designed to be used with an Arduino Leonardo, a few cut 1m/144 WS2812B LED strips and a USB serial connection.

If you want to recreate my exact setup, you'll need:
- 2x 1m/144 WS2812B LED strips (IP30 and IP65 protected strips will have adhesive tape on the back)
- 5V 15A power supply (although in my experience, 10A is enough)
- Arduino Leonardo (additionally a case)
- Universal DC Adapters (seperates the round plug from the power supply into a screw terminal)
- AWG22 or better rated wires
- Jumper wires
- Anything to connect two wires together (I forgot about this at first ._.)

If your LED strip has 5 wires, the outer two being longer and thicker, connect those to your power supply. The middle wire is the data wire, which should be connected to the Arduino (in my case on pins 2 and 4). Ensure the ground wire is also connected to the Arduino, note that the ground wire from the inner 3 wires is directly connected to the ground wire from the outer 2 wires and can be used for this purpose safely. Finally, put the Arduino in a case and mount it as well as the LED strips to the back of your monitor.
## Installing
ambient-led consists of two parts: the host program and the microcontroller program. The host program is written in Rust, and the microcontroller program is written in C++. The host program is responsible for capturing the screen and sending the data to the microcontroller, which is responsible for controlling the LEDs.
### Host program
Ensure rust is installed alongside cargo, then run:
```sh
cargo build --release
```
The executable will be located at `target/release/ambient-led`.
### Microcontroller program
The microcontroller program is written for the Arduino, but can be easily adapted to other microcontrollers. The program is located at `arduino/arduino.ino`. You first have to edit the file to match your LED strip configuration.

Change `MAX_BRIGHTNESS` to adjust the maximum brightness of all LEDs. This value should be between 0 and 255. Add `STRIPX_LENGTH` and `STRIPX_DATA` corresponding to the number of LEDs and the data pin of the LED strip. You can add up to 4 LED strips (1m/144) to an Arduino Leonardo, but it's really not recommended to add more than 2. Finally, ensure the baud rate specified in `SERIAL_BAUD` is large enough to handle the data sent by the host program. Generally `1000000` is a good value. The baud rate is the amount of bits per second that can be sent over the serial connection. For example, with 200 LEDs updating 30 times a second, the baud rate needs to be above `144000`. The nearest standard baud rate is `250000`, which is more than enough. However, increasing the baud rate to `500000` or `1000000` will allow for the LEDs to update quicker and more smoothly.

After editing the file, upload it to the Arduino either through the Arduino IDE or through the command line. When using `arduino-cli`, simply run:
```sh
arduino-cli lib install FastLED
arduino-cli compile -b arduino:avr:leonardo -p /dev/ttyACMX -u --warnings all arduino.ino
```
Replace `/dev/ttyACMX` with the port your Arduino is connected to.
## Configuration
Before being able to use ambient-led, you need to setup the configuration.
For starters, copy `config.example.yml` into `~/.config/ambient-led/config.yml` and edit it to match your setup.

Quick explanations for each field are in the example file, but for a more detailed explanation, check out `src/configuration.rs`

After setting up the configuration, copy the example shaders from `shaders/*` or write your own and place them in `~/.config/ambient-led/shaders/`.

Finally, run the host program with:
```sh
./target/release/ambient-led
```
... or copy it to wherever you want and run it from there.
## Launch options
Use `--help` to see all available options.
- `-v` overrides the log level to `trace`
- `-t <frames>` will run the program for a set amount of frames before exiting. This is useful combined with `-v` for debugging.
- `-c <path>` will override the path to the configuration file. By default, it's `~/.config/ambient-led/config.yml`.
## Auto-start
While you can simply add an autostart desktop entry, the program occasionally crashes and I haven't figured out why yet. I recommend setting up a systemd service instead.
Put the following in `~/.config/systemd/user/ambient-led.service`:
```ini
[Unit]
Description=ambient-led service

[Service]
Type=simple
Environment="WAYLAND_DISPLAY=wayland-1"
ExecStart=<path to ambient-led>
Restart=always
RestartSec=60s
StartLimitIntervalSec=0
StartLimitBurst=999999999

[Install]
WantedBy=default.target
```
Ensure the path to `ambient-led` is correct. Then run:
```sh
systemctl --user enable ambient-led
```

This will automatically start the program when you log into your user account.
To launch the program manually, run:
```sh
systemctl --user start ambient-led
```

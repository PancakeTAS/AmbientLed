The Art of over-engineering a backlight.

# Introduction
Why did I make this?

Being in a dark room with a bright screen is not good for your eyes. The term "flashbang" is a pretty good word to describe the feeling of accidentally opening wikipedia in the middle of the night and tearing your eyes out.
How can we fix this?
Well, we *could* turn down the brightness of the screen, but that kinda ruins the experience for dark scenes in movies and games. Perhaps some black stabilizer? Or lower contrast? No. Those are all terrible ideas.
Then how about we turn on a fucking lamp? Hey that's a great idea! If we just turn on the roomlight then there's no problem anymore! Though.. I do wish to sleep at night and having the lights on hours before going to bed doesn't help falling asleep... and also the immersion is ruined!! You can't play horror games in a lit room!
My solution for the longest time was a small lamp behind the monitor, which made bright websites not pain to look at and was really nice (I have 3 monitors in front of me, so there's plenty of light already).

Eventually the yellow light got on my nerves: I wanted something white! I grabbed the led strip from my piano, put it on the back of my monitor and hours later ambient-led was born.
Originally, it was written in very poor java, was able to interface with an arduino and a raspberry pi (via wifi) and could set the color to whatever I wanted it to be. Such as, for example, a nice rainbow gradient! (i- it was just white.. on the wall... but we don't talk about that)
Later I added GDI support to capture my desktop and set the color of the led strip to the average color of the screen. Extremely CPU intensive, because GDI sucks and averaging every pixel on the screen is not a good idea.

Fast forward a bit, I now had 3 led strips, top and bottom of my primary monitor and one above my piano monitor. With the help of very strong usb-c power supplies, I used the 5v pin on the pi 4, pi 3 and arduino to power the led strips.. which is a terrible idea because these leds are POWER HUNGRY. Either way, this was my solution for the longest time. I also switched to NvFBC and linux after some time and it was a lot better, but still not perfect.

## Problems with the old setup
- The led strips are not bright enough with internal power
- Holy shit I just wasted 2 pis and an arduino for fucking backlights
- The desktop capture does not work on wayland
- Support for backwards led strips, top to bottom and other led strips is incredibly janky
- I wrote this shit in C, am I insane or something?!

# Introducing ambient-led-rs
*no, the -rs suffix does not indicate that I merely switched to rust*
Okay, to fix this mess let's start at the top and work our way down.

## WS2812B LED Strips
In order to individually address each LED on an LED strip, you need an LED strip, that has individually addressable LEDs... *ahem*.
As you're probably already familiar with, WS2812B LED strips do exactly that! They feature a tiny controller that is capable of controlling a bunch of leds in a strip of varying lengths and densities. For my monitor I decided to pick 2x 1m/144 strips. WS2812B LED strips are HUNGRY, like, really hungry, insanely hungry. A quick google search tells you that each LED eats 20mA. WS2812B, being a RGB led strip, has 3 of those per pixel. 144 pixels... 1 meter... 3 leds... 20 mA, that's.. uh..
*punches 144\*3\*20 into the calculator*
8640mA or just under 9A. That's.. a lot, the silly LED strip consumes up to 45 watts of power! That's like.. 3 entire steam decks at full load! Okay it's not actually *that* much, I won't be running them at max brightness anyways.

## Arduino Leonardo
The Arduino Leonardo is a microcomputer (do you even call them microcomputers?) that I happened to have lying around. It's.. extremely slow, and cheap, which makes it perfect for a usecase like this. It interacts with the computer via usb serial and has a bunch of GPIO pins that can drive WS2812B LED strips. Sounds great to me, let's use it! Oh and while we're at it, let's add an extneral power supply to power the LED strips. Can one arduino handle 2 strips? Eh, let's find out...
I measured my monitors width and figured I'd have around 180 leds in total, which would draw 10.8A at full brightness, so I bought a 5V 15A brick. Did I bother checking if my leds even use that much power? No, I did not..
I was even gonna go for a 20A power supply and a longer strip, but sadly going above 20A only comes with those big bulky power supplies that I'm way too scared to even touch. I'm not gonna burn my house down for some pretty lights.
A- anyways, do I need a capacitor? I don't know, I'm not an electrician, I'm just a software engineer. I'll ~~just add one to be safe.~~ just not buy one I'm sure it'll be fine. Oh, a resistor is also required? Nah I'll just pretend I didn't see that. (jokes aside, I did research and apparently modern strips have resistors and capacitors built in, so I'm good). One thing I forgot to buy.. was wires that can handle 10A.
Thankfully Ben Eaters 6502 computer series has this nice kit I own and it has wires rated.. 22AWG. What does that mean anyways? Oh, can handle 7A? That's great considering each strip only consumes 5A at full brightness!
I also used to own a Märklin train set, which had these nice screw in connectors, so I used those to connect the wires to the LED strips.

Let's get to programming the Arduino. I was considering actually doing it in rust, but setting that up was too much of a hassle sadly.. so I went with the good old C (idk, is it c++? i genuinely don't know lmao). Previously my Arduino code did a lot more than just grab Serial data and set leds, it actually had it's own timer, did linear interpolation between values to double the framerate and.. yeah okay that's it, it only did lerping. Anyways, turns out, 2.5kb memory is very little! I wasn't able to reserve enough space for actually implementing anything fancy, because I wanted the Arduino to run up to 4 or 2 long led strips. The arduino code ended up.. very dry, as you can see. I was so limited that I had to rely on preprocessor statements to setup the led strip..

## Rust
Enough distraction, let's get into the fun part of this entire project, the software. So far nothing has really been overkill... I mean this is all pretty simple stuff, so let's go above and beyond with the software.

### Capturing the screen
On linux, the operating system I use, there are two programs you can use for getting graphics on your screen: X11 and Wayland. You can compare X11 to Windows without explorer.exe. It handles windows, rendering, input and some more stuff. Ontop of X11 you install a desktop environment or window manager, which.. manages stuff like windows, actually implements keybinds, it does all the fancy stuff you see on linux. Luckily, X11 has a way to take a screenshot! It's called XSHM.

#### XSHM
Holy fuck, XSHM is probably the single worst thing that could ever happen to you. Let's understand why. Which PC component renders to the screen.. that would be the GPU. That means the GPU holds the framebuffer and renders ontop of it. If you want to copy the current framebuffer into an integer array in your rust code, you will have to copy it from the GPU, to the CPU. XSHM, X11 Shared Memory, does just that. It copies the framebuffer to the CPU memory and shares us the memory handle so that we can access the framebuffer without copying it *again*. Sounds amazing, doesn't it? Well... turns out, XSHM actually *pauses* rendering while copying.. or something. I still don't know exactly what it does, but while XSHM is running, your entire PC turns into a potato, no matter how good it is. (except if you are on integrated graphics).

#### NvFBC
NVIDIA Frame Buffer Capture comes to the rescue. Using the Capture SDK, I can capture the framebuffer directly at the last step without losing any frames. NvFBC can even scale down the texture, so the transfer rate isn't too high. NvFBC is locked on consumer cards, but I was able to find a patch that unlocks it (thank you keylase). NvFBC is.. amazing. It's fast, it's efficient, it's everything you could ever want. It's also not available on wayland, which sucks because I just switched to wayland... *sigh*.

#### What about Wayland?
Let's understand what wayland is. Wayland itself is just a protocol.. in fact it's a single XML file that isn't very long either. Wayland describes how a client (firefox, discord, vscode) interacts with the server (also known as compositor). Wayland is relatively small, but the protocol contains everything you need to get a full desktop experience. The compositor is completely free in how it implements anything in the back as long as it provides a set of wayland protocols.
Where X11 is the full server implemented exactly like the people over at the X.Org Foundation wanted it to be, Wayland is merely a protocol that ensures every implementation of it's protocol is compatible with every client. It's great! Buuut it doesn't have a way to take a screenshot, at all. Actually, only recently, they merged a protocol that does exactly that, but it's not implemented in any compositor yet. So.. what do we do?
Thankfully, I am using Hyprland, which is based on WLRoots, which is a wayland compositor with it's own protocol extensions. And guess what? It has a screenshot protocol! It actually has two! Let's use that!

#### The Rust Code
First, we have to implement a wayland client, to connect to the compositor. Thankfully the documentation is absolutely flawless.. although it takes a bit to wrap your head around it. You don't have simple function calls or variables you can read, instead, when you call a method you actually queue a message, which the server then responds to with a Dispatch that you can handle in another method. It's a bit weird, but super powerful.
Initializing a wayland client is fairly straight forward: After connecting to the compositor, you create a registry object and ask the server to tell you all the protocols you have access to in this connection.
```rust
let wl = Connection::connect_to_env()?;

let mut eq = wl.new_event_queue();
wl.display().get_registry(&eq.handle(), ());

eq.blocking_dispatch(&mut state)?;
```
As you can see from this code snippet, we connect via environment variables, create an event queue for our get_registry message and then block until all dispatches have finished executed.

Now we need to bind the protocols we need into the registry, which is done in the Dispatch that is triggered for every single protocl during `blocking_dispatch`.
```rust
impl Dispatch<wl_registry::WlRegistry, ()> for Client {
    fn event(state: &mut Self, registry: &wl_registry::WlRegistry, event: wl_registry::Event, _: &(), _: &Connection, eq_handle: &QueueHandle<Client>) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            if interface == WlOutput::interface().name {
                state.outputs.insert(registry.bind::<WlOutput, _, _>(name, version, eq_handle, ()), OutputInfo::default());
            } else if interface == ZwlrScreencopyManagerV1::interface().name {
                state.wlr_screencopy_manager = Some(registry.bind::<ZwlrScreencopyManagerV1, _, _>(name, version, eq_handle, ()));
            } else if interface == ZwpLinuxDmabufV1::interface().name {
                state.wp_linux_dmabuf = Some(registry.bind::<ZwpLinuxDmabufV1, _, _>(name, version, eq_handle, ()));
            }
        }
    }
}
```
I'll get back to what each protocol does in a bit, but as you can see we're simply binding the protocols to the registry here and adding WlOutput objects to our state (these are the monitors attached to your system).

The protocol we will be using for taking a screenshot is called ZwlrScreencopyManagerV1. Ignore the `Z` prefix and `V1` suffix, those are mandatory for unstable protocols. The `wlr` prefix indicates this is a **wlr**oots protocol, so really the protocol is called "screencopy manager".

The first step is fairly simple, you request to capture a WlOutput optionally specifying a rectangle to capture and wait for the server to respond with a bunch of dispatches...
```rust
impl Dispatch<ZwlrScreencopyFrameV1, ()> for CaptureSession {
    fn event(session: &mut Self, proxy: &ZwlrScreencopyFrameV1, event: <ZwlrScreencopyFrameV1 as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        // ensure screencopy_frame is set
        if session.screencopy_frame.is_none() {
            session.screencopy_frame = Some(proxy.clone());
        }

        // handle events
        match event {
            zwlr_screencopy_frame_v1::Event::LinuxDmabuf { format, width, height } => {
                session.requested_dmabuf_params = Some((format, width, height));
            },
            zwlr_screencopy_frame_v1::Event::Failed { } => {
                session.fail = true;
            },
            _ => {}
        }
    }
}
```
Hold on! "Screencopy Frame", wasn't it "Screencopy Manager" before? Yes, it was, but calling the capture method actually creates a new object. Interestingly we don't have to explicitly bind this one into the registry, I assume it does this in the backend, but I'm not sure.

What's `LinuxDmabuf`? Wait-

Linux dmabuf stands for linux direct memory access buffer. In laymans terms, its a buffer that isn't allocated on the CPU, but the GPU. This is great, because the protocol doesn't support scaling down the framebuffer, and copying the entire frame to CPU is very wasteful. Now, we can leave the buffer on the GPU and scale it down manually before copying it to the gpu.

The `LinuxDmabuf` event doesn't create a DMABUF however, it just tells us the parameters of the buffer that *we* have to create.. amazing, time to figure out how to do that.
Thankfully, it's not that hard. One level above Wayland and X11 and even the NVIDIA driver, is libdrm. libdrm, "Direct Rendering Manager library", is a library that provides a way to interact with the GPU in a way that is not vendor specific, on a very basic level. It is *not* documented at all, but it's not too difficult to figure out how it works. Besides, we don't actually care about DRM, but GBM "Generic Buffer Management", which is a super, super, super tiny drm extension that allows us to create buffers on the GPU. It's relatively simple to create a GBM buffer, however, in order to copy the frame we can't just pass the DMABUF handle.. instead, we need to first create a WlBuffer using the other protocol we bound earlier, ZwpLinuxDmabufV1.
```rust
let gbm = gbm::Device::new(File::open("/dev/dri/renderD128")?)?;

let bo = self.gbm.create_buffer_object::<()>(width, height, gbm::Format::try_from(fourcc)?, BufferObjectFlags::RENDERING)?;

let linux_buffer_params = dmabuf_mgmt.create_params(&eq.handle(), ());
unsafe {
    let modifiers: u64 = bo.modifier()?.into();
    linux_buffer_params.add(
        BorrowedFd::borrow_raw(bo.fd_for_plane(0)?.as_raw_fd()),
        0,
        bo.offset(0)?,
        bo.stride_for_plane(0)?,
        (modifiers >> 32) as u32, modifiers as u32
    );
}

linux_buffer_params.create(width as i32, height as i32, fourcc, Flags::empty());
eq.blocking_dispatch(session)?; // see method below

session.linux_buffer_params = Some(linux_buffer_params);
session.buffer_object = Some(bo);
let buffer = match &session.buffer {
    Some(buffer) => buffer,
    None => return Err("unexpected error: no buffer created".into())
};

if session.fail {
    return Err("failed to create buffer".into());
}

buffer

// method below
```rust
#[allow(unused_variables, non_snake_case)]
impl Dispatch<ZwpLinuxBufferParamsV1, ()> for CaptureSession {
    event_created_child!(CaptureSession, ZwpLinuxBufferParamsV1, [
        EVT_CREATE_BAR => (WlBuffer, ()),
    ]);

    fn event(state: &mut Self, _: &ZwpLinuxBufferParamsV1, event: <ZwpLinuxBufferParamsV1 as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let zwp_linux_buffer_params_v1::Event::Created { buffer } = event {
            state.buffer = Some(buffer);
        }
    }
}
```
This code looks incredibly complex, but it really isn't. I'm not gonna go explain every last bit of it, but basically we're creating a LinuxBufferParams object, adding our DMABUF to it and asking the compositor to create a WlBuffer based on that.

Once the buffer is obtained, we can simply call copy and get the buffer data into the DMABUF. Then we can store the DMABUF and use it for future captures. Hyprland has a memory leak somewhere here anyways, if you keep recreating DMABUFs you will eventually run out of memory and crash the compositor. I'm not sure where the leak is, but I'm sure it's not my fault.

### Rendering the LED strip
..did you just say rendering? Yes, I did :3
We gotta find a way to scale down the buffer on the gpu and then copy it to the GPU. I'm sure I can use some very simple DRM function for this, but where's the fun in that. Let's use OpenGL instead! I'm sure that's a great idea! (spoiler: it's not)

First, let's create an OpenGL context... uhh
*600 lines of boilerplate later*

Alright, first things first, how do we get the DMABUF into an OpenGL texture.. they're both on the GPU, so I'm sure that's easy.
It actually is! There's an EGL extension to import DMABUFs into textures, so let's use that. First, we have to manually get the address of the function and then cast it to a function pointer. This is completely safe, but since we are using memory addresses, we have to use rust's unsafe keyword everywhere...
```rust
///
/// Extensions to the egl module
///
/// #[allow(non_camel_case_types)]
#[allow(non_snake_case)]
mod egl2 {
    use egls::{egl, EGLContext, EGLDisplay, EGLImageKHR, EGLenum};

    pub const LINUX_DMA_BUF_EXT: EGLenum = 0x3270;
    pub const LINUX_DRM_FOURCC_EXT: EGLenum = 0x3271;
    pub const DMA_BUF_PLANE0_FD_EXT: EGLenum = 0x3272;
    pub const DMA_BUF_PLANE0_OFFSET_EXT: EGLenum = 0x3273;
    pub const DMA_BUF_PLANE0_PITCH_EXT: EGLenum = 0x3274;
    pub const DMA_BUF_PLANE0_MODIFIER_LO_EXT: EGLenum = 0x3443;
    pub const DMA_BUF_PLANE0_MODIFIER_HI_EXT: EGLenum = 0x3444;

    type PFNEGLCREATEIMAGEKHRPROC = extern "C" fn(dpy: EGLDisplay, ctx: EGLContext, target: EGLenum, buffer: *const std::ffi::c_void, attrib_list: *const i32) -> EGLImageKHR;
    static mut CREATE_IMAGE_KHR: Option<PFNEGLCREATEIMAGEKHRPROC> = None;
    pub unsafe fn CreateImageKHR(dpy: EGLDisplay, ctx: EGLContext, target: EGLenum, buffer: *const std::ffi::c_void, attrib_list: *const i32) -> EGLImageKHR {
        if !CREATE_IMAGE_KHR.is_some() {
            let proc = std::ffi::CString::new("eglCreateImageKHR").unwrap();
            let addr = egl::GetProcAddress(proc.as_ptr());
            CREATE_IMAGE_KHR = Some(std::mem::transmute(addr));
        }

        CREATE_IMAGE_KHR.unwrap()(dpy, ctx, target, buffer, attrib_list)
    }

    type PFNEGLDESTROYIMAGEKHRPROC = extern "C" fn(dpy: EGLDisplay, image: EGLImageKHR) -> egl::EGLBoolean;
    static mut DESTROY_IMAGE_KHR: Option<PFNEGLDESTROYIMAGEKHRPROC> = None;
    pub unsafe fn DestroyImageKHR(dpy: EGLDisplay, image: EGLImageKHR) -> egl::EGLBoolean {
        if !DESTROY_IMAGE_KHR.is_some() {
            let proc = std::ffi::CString::new("eglDestroyImageKHR").unwrap();
            let addr = egl::GetProcAddress(proc.as_ptr());
            DESTROY_IMAGE_KHR = Some(std::mem::transmute(addr));
        }

        DESTROY_IMAGE_KHR.unwrap()(dpy, image)
    }
}

///
/// Extensions to the gl module
///
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
mod gl2 {
    use std::{ffi::{c_void, CString}, mem};
    use egls::{egl, EGLImageKHR};
    use gl::types::GLenum;

    type PFNGL_EGLIMAGETARGETTEXTURE2DOESPROC = extern "C" fn(target: GLenum, image: EGLImageKHR);
    static mut EGLIMAGE_TARGET_TEXTURE2_DOES: Option<PFNGL_EGLIMAGETARGETTEXTURE2DOESPROC> = None;
    pub unsafe fn EGLImageTargetTexture2DOES(target: GLenum, image: *const c_void) {
        if !EGLIMAGE_TARGET_TEXTURE2_DOES.is_some() {
            let proc = CString::new("glEGLImageTargetTexture2DOES").unwrap();
            let addr = egl::GetProcAddress(proc.as_ptr());
            EGLIMAGE_TARGET_TEXTURE2_DOES = Some(mem::transmute(addr));
        }

        EGLIMAGE_TARGET_TEXTURE2_DOES.unwrap()(target, image as EGLImageKHR);
    }
}
```
Once again, looks more complex than it actually is. All we are really doing is calling `eglGetProcAddress` with a the method name as string parameter and storing the function pointer in a static variable. Then we can call the function pointer as if it was a normal function.

Then we can simply create an EGLImage from the DMABUF and bind it to a texture.
```rust
// create egl image from dmabuf
let image = unsafe { egl2::CreateImageKHR(
    dpy,
    egl::NO_CONTEXT,
    egl2::LINUX_DMA_BUF_EXT,
    ptr::null(),
    [
        egl::WIDTH as i32, width as i32,
        egl::HEIGHT as i32, height as i32,
        egl2::LINUX_DRM_FOURCC_EXT as i32, format as i32,
        egl2::DMA_BUF_PLANE0_FD_EXT as i32, dmabuf.as_raw_fd() as i32,
        egl2::DMA_BUF_PLANE0_OFFSET_EXT as i32, offset as i32,
        egl2::DMA_BUF_PLANE0_PITCH_EXT as i32, stride as i32,
        egl2::DMA_BUF_PLANE0_MODIFIER_LO_EXT as i32, modifiers as i32,
        egl2::DMA_BUF_PLANE0_MODIFIER_HI_EXT as i32, (modifiers >> 32) as i32,
        egl::IMAGE_PRESERVED_KHR as i32, 1,
        egl::NONE as i32
    ].as_ptr()
)};
if image == egl::NO_IMAGE_KHR {
    return Err("failed to create image from dmabuf".into());
}

// create texture from egl image
let texture = unsafe { Texture::create_bound_texture(gl::TEXTURE_2D) };
unsafe {
    gl2::EGLImageTargetTexture2DOES(gl::TEXTURE_2D, image);
    gl::BindTexture(gl::TEXTURE_2D, 0);
};

Ok(Texture {
    id: texture,
    image: Some(image)
})
```
This code is.. very simple. We create an EGLImage from the DMABUF and then bind it to a texture. The texture is then returned to the caller.

Now that we have the texture, we just need an offscreen framebuffer and an opengl context, both of which are easily created with a few lines of code that I will spare you from. Then we can simply render the texture to the framebuffer and read the framebuffer into an integer array. As long has the same resolution as our led strip (1x144) we can simply copy the framebuffer to the CPU, which is super fast and write it straight to the Arduino.

But wait.. wouldn't it be cool if we could add.. special effects? Like.. a gradient? Or.. a rainbow? Or.. a.. uh.. I don't know,
```glsl
#version 330 core

out vec4 FragColor;

in vec2 TexCoord;

uniform sampler2D texture1;

void main() {
	FragColor = texture(texture1, TexCoord);
}
```
This.. is a shader. It's.. very simple. It just takes a texture and renders it to the screen. We don't really need a shader for just scaling down textures, but if we want to add effects, we can do that in the shader. For example, we could add a gradient effect by adding a gradient to the shader.. or.. uh.. a rainbow effect by.. adding a rainbow to the shader.. or.. uh.. I don't know, I'm not a graphics programmer.

So now we have a fully functioning rendering engine for... 144 pixels behind a monitor. I'm sure this was worth the effort.

### Writing to the Arduino
Almost done, just create a serial port, write the color to it and.. done!
I'm just kidding of course. I will overcomplicate this!

See, right now I have a single arduino with two led strips. But what if I want to add more? I could add the same setup to my left and right monitor and have a backlight for them as well... But then, what if I want to be super immersed in a game and combine all three strips into one? Hmmm... let's write virtual strips!

First things first, connecting to a device
```rust
///
/// Create a new serial device
///
/// # Arguments
///
/// * `port` - Path to the serial port
/// * `baud_rate` - Baud rate
/// * `lengths` - Amount of leds per strip connected to this device
///
pub fn new(port: &PathBuf, baud_rate: u32, lengths: Vec<u16>) -> Result<Self, Box<dyn Error>> {
    Ok(Self {
        port: SerialPort::open(port, baud_rate)?,
        buffer: vec![0; lengths.iter().sum::<u16>() as usize * 3],
        lengths,
    })
}

///
/// Get a mutable reference to a subarray of the buffer
///
/// # Arguments
///
/// * `strip` - Strip index
/// * `offset` - Offset in the strip
/// * `length - Length of the subarray
///
pub(super) fn get_mut(&mut self, strip: u8, offset: u16, length: u16) -> &mut [u8] {
    let start = self.lengths.iter().take(strip as usize).sum::<u16>() as usize * 3 + offset as usize * 3;
    &mut self.buffer[start..start + length as usize * 3]
}

///
/// Write the data to the serial port
///
pub(super) fn write(&self) -> Result<(), Box<dyn Error>> {
    self.port.write_all(&self.buffer)?;
    self.port.flush()?;
    Ok(())
}
```
This part is super simple. Since there's no separation between the two buffers in my arduino code, I just have to specify the length of each strip and allocate a large enough buffer to hold all the data. If I want to write to a specific strip, I can simply call `get_mut` with the strip index, offset and length to get the section of the buffer I want to write to. Then I can simply call `write` to write the buffer to the serial port.

Now let's add virtual strips!
```rust
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
}
```
The code so far should be relatively primitive again, we have a struct called Strip with a length we can specify in the constructor. Then we map each part of this virtual strip to physical strips using the Mapping struct. We can then get a mutable reference to the buffer to set the colors of the virtual strip.

Now we need to write the data to the physical strips.
```rust
let mut offset = 0;
for mapping in &self.mappings {
    let device = device_map.get_mut(&mapping.device_id).ok_or("device not found")?;
    let buffer_slice = device.get_mut(mapping.strip_id, mapping.offset, mapping.length);
    buffer_slice.copy_from_slice(&self.buffer[offset..offset + mapping.length as usize * 3]);
    offset += mapping.length as usize * 3;
}
Ok(())
```
This code gets the mutable buffer from the device for the small section specified in the mapping and copies the data from the virtual strip to the physical strip. This is done for every mapping in the virtual strip.

Now, technically, I can split any physical strip into any number of virtual strips, or combine any number of physical strips into a single virtual strip. This is.. very overkill, but it's also very cool. I'm sure I will never use this feature, but it's cool!

## What's next?
We still don't have a way to setup all of the strips without writing rust code and recompiling the program. Next on the list is a configuration file. Shaders are already loaded from the disk, so that's already done.

I also have to fix a small bug... well not really a bug. Currently I personally make two captures of the screen, top and bottom 30 times a second. That totals to 60 times a second, which will make my GPU render frames even though nothing changed. The screencopy protocol does allow me to wait for an update before rerendering, but then I'll have a problem with the Arduino timeout. I'll find a way around this, but for now I'm just gonna leave it as is.

# Conclusion
You might think that this is a lot of work for a simple backlight, and you'd be right. But it's also a lot of fun! I learned a lot about how the GPU works, how to interact with the GPU, how to write shaders, how to write wayland clients, how to write serial communication code and how to write a lot of boilerplate code. I'm sure I'll never use this knowledge again, but it was a fun project and I'm happy with the result. I hope you enjoyed reading this as much as I enjoyed writing it!

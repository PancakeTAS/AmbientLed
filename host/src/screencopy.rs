use std::{collections::HashMap, fs::File, os::fd::{AsRawFd, BorrowedFd}};

use gbm::BufferObjectFlags;
use wayland_client::{backend::ObjectId, event_created_child, protocol::{wl_buffer::WlBuffer, wl_output::{self, WlOutput}, wl_registry::{self}}, Connection, Dispatch, Proxy, QueueHandle};
use wayland_protocols::wp::linux_dmabuf::zv1::client::{zwp_linux_buffer_params_v1::{self, Flags, ZwpLinuxBufferParamsV1}, zwp_linux_dmabuf_v1::ZwpLinuxDmabufV1};
use wayland_protocols_wlr::screencopy::v1::client::{zwlr_screencopy_frame_v1::{self, ZwlrScreencopyFrameV1}, zwlr_screencopy_manager_v1::ZwlrScreencopyManagerV1};

///
/// # OutputInfo
///
/// The OutputInfo struct holds information about an output. This is used to store the output information
/// when the registry is populated. Not all fields are guaranteed to be set.
///
pub struct OutputInfo {
    pub name: Option<String>,
    pub description: Option<String>, // likely unset
    pub mode: Option<(i32, i32, i32)>, // width, height and refresh in mHz
}

impl OutputInfo {
    fn default() -> Self {
        OutputInfo { name: None, description: None, mode: None }
    }
}

///
/// # Client
///
/// The client struct is the main entry point for the screencopy module. It holds the wayland connection and the gbm device.
/// It also holds the outputs and the required protocols.
///
pub struct Client {
    wl: Connection,
    gbm: gbm::Device<File>,

    // wayland objects
    pub outputs: HashMap<WlOutput, OutputInfo>,

    // wayland protocols
    wlr_screencopy_manager: Option<ZwlrScreencopyManagerV1>,
    wp_linux_dmabuf: Option<ZwpLinuxDmabufV1>
}

impl Client {

    ///
    /// Create a new Client
    ///
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let wl = Connection::connect_to_env()?;
        let gbm = gbm::Device::new(File::open("/dev/dri/renderD128")?)?;

        let mut eq = wl.new_event_queue();
        wl.display().get_registry(&eq.handle(), ());

        let mut state = Client {
            wl, gbm,
            outputs: HashMap::new(),
            wlr_screencopy_manager: None, wp_linux_dmabuf: None
        };

        eq.blocking_dispatch(&mut state)?;
        eq.blocking_dispatch(&mut state)?; // fetch outputs after populating registry

        // ensure required globals are present
        if state.wlr_screencopy_manager.is_none() {
            return Err("screencopy manager not found".into());
        }

        if state.wp_linux_dmabuf.is_none() {
            return Err("linux dmabuf not found".into());
        }

        Ok(state)
    }

    ///
    /// Get the display id
    ///
    pub fn get_display_id(&self) -> ObjectId {
        self.wl.display().id()
    }

}

impl Drop for Client {
    fn drop(&mut self) {
        if let Some(wlr_screencopy_manager) = self.wlr_screencopy_manager.as_mut() {
            wlr_screencopy_manager.destroy();
        }
        if let Some(wp_linux_dmabuf) = self.wp_linux_dmabuf.as_mut() {
            wp_linux_dmabuf.destroy();
        }
    }
}

///
/// WlRegistry dispatch
///
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

///
/// WlOutput dispatch
///
impl Dispatch<wl_output::WlOutput, ()> for Client {
    fn event(state: &mut Self, proxy: &wl_output::WlOutput, event: <wl_output::WlOutput as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let Some(info) = state.outputs.get_mut(proxy) {
            match event {
                wl_output::Event::Name { name } => info.name = Some(name),
                wl_output::Event::Description { description } => info.description = Some(description),
                wl_output::Event::Mode { width, height, refresh, .. } => info.mode = Some((width, height, refresh)),
                _ => { }
            }
        }
    }
}

///
/// # Screencopy implementation
///
/// The following code implements the screencopy protocol for capturing the screen. Due to using many dispatches,
/// the code is spread out across many impl blocks and relies on blocking_dispatch to wait for the dispatch execution to finish,
/// before continuing.
///

///
/// # CaptureSession
///
/// The CaptureSession struct holds the state of a capture session. It holds the requested dmabuf parameters, the screencopy frame,
/// the linux buffer params and the buffer. It also holds a fail flag to indicate if any of the dispatches failed.
///
#[derive(Debug)]
pub struct CaptureSession {
    requested_dmabuf_params: Option<(u32, u32, u32)>, // fourcc, width, height
    screencopy_frame: Option<ZwlrScreencopyFrameV1>,
    linux_buffer_params: Option<ZwpLinuxBufferParamsV1>,
    buffer_object: Option<gbm::BufferObject<()>>,
    buffer: Option<WlBuffer>,

    fail: bool, // if any of the dispatches failed
    output: WlOutput,
    x: i32, y: i32, width: i32, height: i32
}

impl CaptureSession {
    pub fn new(output: WlOutput, x: i32, y: i32, width: i32, height: i32) -> Self {
        CaptureSession {
            requested_dmabuf_params: None, screencopy_frame: None, linux_buffer_params: None, buffer_object: None, buffer: None,
            fail: false, output,
            x, y, width, height
        }
    }

    pub fn get_dmabuf(&self) -> Result<&gbm::BufferObject<()>, &'static str> {
        self.buffer_object.as_ref().ok_or("no buffer object found")
    }
}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        if let Some(ref screencopy_frame) = self.screencopy_frame {
            screencopy_frame.destroy();
        }
        if let Some(ref linux_buffer_params) = self.linux_buffer_params {
            linux_buffer_params.destroy();
        }
        if let Some(ref buffer) = self.buffer {
            buffer.destroy();
        }
    }
}

impl Client {

    ///
    /// Capture the output of a session.
    ///
    /// You may reuse the same session for multiple captures to skip the dmabuf creation.
    ///
    pub fn capture(&mut self, session: &mut CaptureSession) -> Result<(), Box<dyn std::error::Error>> {
        if session.fail {
            return Err("session is marked as failed".into());
        }

        let mut eq = self.wl.new_event_queue::<CaptureSession>();
        let output = &session.output;
        let skip_dmabuf =
            if session.requested_dmabuf_params.is_none()
                || session.screencopy_frame.is_none()
                || session.linux_buffer_params.is_none()
                || session.buffer_object.is_none()
                || session.buffer.is_none() {
                false
            } else {
                session.screencopy_frame.as_mut().unwrap().destroy();
                session.screencopy_frame = None;
                true
            };

        // get the required protocols
        let screencopy_mgmt = self.wlr_screencopy_manager.as_ref().ok_or("no screencopy manager found")?;
        let dmabuf_mgmt: &ZwpLinuxDmabufV1 = self.wp_linux_dmabuf.as_ref().ok_or("no linux dmabuf found")?;

        // request output capture
        screencopy_mgmt.capture_output_region::<(), _>(0, output, session.x, session.y, session.width, session.height, &eq.handle(), ());
        eq.blocking_dispatch(session)?; // this will wait for the dispatches to finish

        let screencopy_frame = match &session.screencopy_frame {
            Some(frame) => frame.clone(),
            None => return Err("no screencopy frame found".into())
        };

        let (fourcc, width, height) = match session.requested_dmabuf_params {
            Some(params) => params,
            None => return Err("dmabuf capture not supported".into())
        };

        if session.fail {
            return Err("failed to capture output".into());
        }

        // create buffer
        let buffer =
            if !skip_dmabuf {
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

                // request wl_buffer
                linux_buffer_params.create(width as i32, height as i32, fourcc, Flags::empty());
                eq.blocking_dispatch(session)?;

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
            } else {
                &session.buffer.as_mut().unwrap()
            };

        // copy the buffer
        screencopy_frame.copy(buffer);
        eq.blocking_dispatch(session)?; // wait for the copy to finish

        if session.fail {
            return Err("copy failed".into());
        }

        Ok(())
    }

}

///
/// ZwlrScreencopyFrameV1 dispatch
///
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

///
/// ZwpLinuxBufferParamsV1 dispatch
///
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

///
/// # Unused dispatches
///
/// The following dispatches are not used and just take up space in the file.
/// I have to figure out how to not require them in
///

/// WlBuffer dispatch
/// (will not trigger due to clientside buffer)
impl Dispatch<WlBuffer, ()> for CaptureSession {
    fn event(_: &mut Self, _: &WlBuffer, _: <WlBuffer as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
    }
}

/// ZwpLinuxDmabufV1 dispatch
/// (all events are deprecated)
impl Dispatch<ZwpLinuxDmabufV1, ()> for Client {
    fn event(_: &mut Self, _: &ZwpLinuxDmabufV1, _: <ZwpLinuxDmabufV1 as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
    }
}

/// ZwlrScreencopyManagerV1 dispatch
/// (there are no events)
impl Dispatch<ZwlrScreencopyManagerV1, ()> for Client {
    fn event(_: &mut Self, _: &ZwlrScreencopyManagerV1, _: <ZwlrScreencopyManagerV1 as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
    }
}

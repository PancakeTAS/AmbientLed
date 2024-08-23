use std::{collections::HashMap, fs::File, os::fd::{AsRawFd, BorrowedFd}};

use anyhow::{anyhow, Context};
use gbm::{BufferObject, BufferObjectFlags, Device};
use log::{debug, trace};
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
    fn default() -> Self { Self { name: None, description: None, mode: None } }
}

///
/// # Screencopy
///
/// The screencopy struct is the main entry point for the screencopy module. It holds the wayland connection and the gbm device.
/// It also holds the outputs and the required protocols.
///
pub struct Screencopy {
    wl: Connection,
    gbm: Device<File>,
    sessions: HashMap<u64, CaptureSession>,

    // wayland objects
    pub outputs: HashMap<WlOutput, OutputInfo>,

    // wayland protocols
    wlr_screencopy_manager: Option<ZwlrScreencopyManagerV1>,
    wp_linux_dmabuf: Option<ZwpLinuxDmabufV1>
}

impl Screencopy {

    ///
    /// Create a new Screencopy
    ///
    /// This will create a gbm device as well as a wayland connection and populate the wayland registry and outputs.
    ///
    /// # Arguments
    ///
    /// * `gbm_device` - The path to the gbm device
    ///
    /// # Errors
    ///
    /// This function will return an error if the drm device cannot be opened, the gbm device cannot be created,
    /// the wayland connection cannot be established, the required protocols are not present or if the registry roundtrip fails.
    ///
    pub fn new(gbm_device: String) -> Result<Self, anyhow::Error> {
        // create the gbm device
        let drm_device = File::open(&gbm_device).context("failed to open drm device")?;
        let gbm = Device::new(drm_device).context("failed to create gbm device")?;
        debug!("created gbm device: {:?}", gbm_device);

        // create the wayland connection
        let wl = Connection::connect_to_env().context("failed to connect to wayland server")?;
        debug!("connected to wayland server: {:?}", std::env::var("WAYLAND_DISPLAY").unwrap_or_else(|_| "wayland-0".to_string()));

        let mut eq = wl.new_event_queue();
        wl.display().get_registry(&eq.handle(), ());

        let mut state = Screencopy {
            wl, gbm, sessions: HashMap::new(),
            outputs: HashMap::new(),
            wlr_screencopy_manager: None, wp_linux_dmabuf: None
        };

        eq.blocking_dispatch(&mut state).context("failed to complete registry roundtrip")?;
        eq.blocking_dispatch(&mut state).context("failed to complete output infos roundtrip")?; // fetch outputs after populating registry
        debug!("populated wayland registry and discovered {} outputs", state.outputs.len());

        // ensure required globals are present
        if state.wlr_screencopy_manager.is_none() {
            return Err(anyhow!("no ZwlrScreencopyManagerV1 protocol"));
        }

        if state.wp_linux_dmabuf.is_none() {
            return Err(anyhow!("no ZwpLinuxDmabufV1 protocol"));
        }

        Ok(state)
    }

    ///
    /// Set the capture session
    ///
    /// # Arguments
    ///
    /// - `id` - The id of the session
    /// - `session` - The session to set
    ///
    /// # Errors
    ///
    /// This function will return an error if the initial capture fails.
    ///
    pub fn set_capture_session(&mut self, id: u64, session: CaptureSession) -> Result<&gbm::BufferObject<()>, anyhow::Error> {
        self.sessions.insert(id, session);
        self.capture(id).context("initial capture failed")?;
        Ok(self.sessions.get(&id).unwrap().buffer_object.as_ref().unwrap())
    }

    ///
    /// Get the display id
    ///
    pub fn get_display_id(&self) -> ObjectId {
        self.wl.display().id()
    }

}

impl Drop for Screencopy {
    fn drop(&mut self) {
        debug!("closing Screencopy, this will disconnect from the wayland server and destroy the gbm device");
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
impl Dispatch<wl_registry::WlRegistry, ()> for Screencopy {
    fn event(state: &mut Self, registry: &wl_registry::WlRegistry, event: wl_registry::Event, _: &(), _: &Connection, eq_handle: &QueueHandle<Screencopy>) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            if interface == WlOutput::interface().name {
                debug!("found output global");
                state.outputs.insert(registry.bind::<WlOutput, _, _>(name, version, eq_handle, ()), OutputInfo::default());
            } else if interface == ZwlrScreencopyManagerV1::interface().name {
                debug!("found screencopy manager global");
                state.wlr_screencopy_manager = Some(registry.bind::<ZwlrScreencopyManagerV1, _, _>(name, version, eq_handle, ()));
            } else if interface == ZwpLinuxDmabufV1::interface().name {
                debug!("found linux dmabuf global");
                state.wp_linux_dmabuf = Some(registry.bind::<ZwpLinuxDmabufV1, _, _>(name, version, eq_handle, ()));
            }

            trace!("new global: name={} interface={} version={}", name, interface, version);
        }
    }
}

///
/// WlOutput dispatch
///
impl Dispatch<wl_output::WlOutput, ()> for Screencopy {
    fn event(state: &mut Self, proxy: &wl_output::WlOutput, event: <wl_output::WlOutput as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let Some(info) = state.outputs.get_mut(proxy) {
            match event {
                wl_output::Event::Name { name } => info.name = Some(name),
                wl_output::Event::Description { description } => info.description = Some(description),
                wl_output::Event::Mode { width, height, refresh, .. } => info.mode = Some((width, height, refresh)),
                wl_output::Event::Done { .. } => {
                    trace!("updated output: name={:?} description={:?} mode={:?}", info.name, info.description, info.mode);
                 },
                _ => {}
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
pub struct CaptureSession {
    requested_dmabuf_params: Option<(u32, u32, u32)>, // fourcc, width, height
    screencopy_frame: Option<ZwlrScreencopyFrameV1>,
    linux_buffer_params: Option<ZwpLinuxBufferParamsV1>,
    buffer_object: Option<BufferObject<()>>,
    buffer: Option<WlBuffer>,

    fail: bool, // if any of the dispatches failed
    output: WlOutput,
    x: i32, y: i32, width: i32, height: i32
}

impl CaptureSession {

    ///
    /// Create a new capture session
    ///
    /// Please note that the capture coordinates use scaled coordinates, not pixels. The resulting buffer might differ in size.
    ///
    /// # Arguments
    ///
    /// * `output` - The output to capture
    /// * `x` - The x position to capture
    /// * `y` - The y position to capture
    /// * `width` - The width to capture
    /// * `height` - The height to capture
    ///
    pub fn new(output: WlOutput, x: i32, y: i32, width: i32, height: i32) -> Self {
        CaptureSession {
            requested_dmabuf_params: None, screencopy_frame: None, linux_buffer_params: None, buffer_object: None, buffer: None,
            fail: false, output,
            x, y, width, height
        }
    }

}

impl Drop for CaptureSession {
    fn drop(&mut self) {
        debug!("dropping capture session, this will destroy the dmabuf buffer and the screencopy frame");
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

impl Screencopy {

    ///
    /// Capture the output of a session.
    ///
    /// # Arguments
    ///
    /// * `session` - The session to capture
    ///
    /// # Errors
    ///
    /// This function will return an error if the session is not found, if the session is marked as failed or if the dmabuf capture is not supported.
    /// It will also fail if any of the dispatches fail.
    ///
    pub fn capture(&mut self, session: u64) -> Result<(), anyhow::Error> {
        let session = self.sessions.get_mut(&session).context("session not found")?;
        if session.fail {
            return Err(anyhow!("session is marked as failed"));
        }

        let mut eq = self.wl.new_event_queue::<CaptureSession>();
        let output = &session.output;
        let output_id = output.id().protocol_id();
        let skip_dmabuf = session.buffer.is_some();

        // reset the session if we are skipping dmabuf
        if skip_dmabuf {
            trace!("skipping dmabuf creation for capture");
            session.screencopy_frame.as_mut().unwrap().destroy();
            session.screencopy_frame = None;
        }

        // get the required protocols
        let screencopy_mgmt = self.wlr_screencopy_manager.as_ref().unwrap();
        let dmabuf_mgmt = self.wp_linux_dmabuf.as_ref().unwrap();

        // request output capture
        session.fail = true;
        screencopy_mgmt.capture_output_region::<(), _>(0, output, session.x, session.y, session.width, session.height, &eq.handle(), ());
        eq.blocking_dispatch(session).context("create output capture roundtrip failed")?; // this will wait for the dispatches to finish

        if session.fail {
            return Err(anyhow!("failed to create output capture"));
        }

        let screencopy_frame = session.screencopy_frame.as_ref().unwrap().clone();
        let (fourcc, width, height) = session.requested_dmabuf_params.context("dmabuf capture not supported")?;
        trace!("created output capture with id {} for region {}x{}+{}+{} on {:?}", screencopy_frame.id().protocol_id(), session.width, session.height, session.x, session.y, output_id);

        // create buffer
        let buffer =
            if !skip_dmabuf {
                let bo = self.gbm.create_buffer_object::<()>(width, height, gbm::Format::try_from(fourcc).unwrap(), BufferObjectFlags::RENDERING)
                    .context("failed to create buffer object")?;
                debug!("allocated dmabuf with format {} and size {}x{}", fourcc, width, height);

                let linux_buffer_params = dmabuf_mgmt.create_params(&eq.handle(), ());
                unsafe {
                    let modifiers: u64 = bo.modifier().unwrap().into();
                    linux_buffer_params.add(
                        BorrowedFd::borrow_raw(bo.fd_for_plane(0).unwrap().as_raw_fd()),
                        0,
                        bo.offset(0).unwrap(),
                        bo.stride_for_plane(0).unwrap(),
                        (modifiers >> 32) as u32, modifiers as u32
                    );
                }

                // request wl_buffer
                session.fail = false;
                linux_buffer_params.create(width as i32, height as i32, fourcc, Flags::empty());
                eq.blocking_dispatch(session).context("create wl buffer roundtrip failed")?;

                if session.fail {
                    return Err(anyhow!("failed to create wl buffer"));
                }

                debug!("created wl buffer {}", session.buffer.as_ref().unwrap().id().protocol_id());
                session.linux_buffer_params = Some(linux_buffer_params);
                session.buffer_object = Some(bo);
                session.buffer.as_mut().unwrap()
            } else {
                session.buffer.as_mut().unwrap()
            };

        // copy the buffer
        session.fail = true;
        screencopy_frame.copy(buffer);
        eq.blocking_dispatch(session).context("copy frame roundtrip failed")?; // this will wait for the dispatches to finish

        if session.fail {
            return Err(anyhow!("copy failed"));
        }

        trace!("frame copied to buffer {}", session.buffer.as_ref().unwrap().id().protocol_id());
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
                session.fail = false;
                session.requested_dmabuf_params = Some((format, width, height));
            },
            zwlr_screencopy_frame_v1::Event::Failed => {
                session.fail = true;
            },
            zwlr_screencopy_frame_v1::Event::Ready { .. } => {
                session.fail = false;
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

    fn event(session: &mut Self, _: &ZwpLinuxBufferParamsV1, event: <ZwpLinuxBufferParamsV1 as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let zwp_linux_buffer_params_v1::Event::Created { buffer } = event {
            session.fail = false;
            session.buffer = Some(buffer);
        } else if let zwp_linux_buffer_params_v1::Event::Failed = event {
            session.fail = true;
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
impl Dispatch<ZwpLinuxDmabufV1, ()> for Screencopy {
    fn event(_: &mut Self, _: &ZwpLinuxDmabufV1, _: <ZwpLinuxDmabufV1 as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
    }
}

/// ZwlrScreencopyManagerV1 dispatch
/// (there are no events)
impl Dispatch<ZwlrScreencopyManagerV1, ()> for Screencopy {
    fn event(_: &mut Self, _: &ZwlrScreencopyManagerV1, _: <ZwlrScreencopyManagerV1 as Proxy>::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
    }
}

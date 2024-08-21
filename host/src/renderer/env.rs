use egls::{egl, so::SharedObject, EGLConfig, EGLNativeDisplayType, EGLSurface, EGLint, Environment};
use log::debug;
use wayland_client::backend::ObjectId;

///
/// Helper function to create an environment with egl and gl functions loaded
///
/// # Arguments
///
/// * `native_display` - The native display object id
///
/// # Errors
///
/// This function will return an error if any of the egl functions fail or the shared object fails to load
///
pub unsafe fn create_environment(native_display: ObjectId) -> Result<(Environment, SharedObject), &'static str> {
    // load egl and gl functions
    let libegl = SharedObject::load("/usr/lib/libEGL.so");
    egl::load_with(|s| libegl.get_proc_address(s));
    let libgl = SharedObject::load("/usr/lib/libGL.so");
    gl::load_with(|s| libgl.get_proc_address(s));
    debug!("dynamic egl/gl libraries loaded");

    // get egl display
    let display = egl::GetDisplay(native_display.as_ptr() as EGLNativeDisplayType);
    if display == egl::NO_DISPLAY {
        return Err("failed to get egl display");
    }
    debug!("got egl display: {:?}", display);

    // initialize egl
    let mut major = 0;
    let mut minor = 0;
    egl::Initialize(display, &mut major as *mut EGLint, &mut minor as *mut EGLint);
    if egl::GetError() != egl::SUCCESS as i32 {
        return Err("failed to initialize egl");
    }
    debug!("initialized egl v{}.{}", major, minor);

    // choose egl config
    let mut config = 0 as EGLConfig;
    let mut num_configs = 0;
    let attrib_list = [
        egl::RED_SIZE, 8,
        egl::GREEN_SIZE, 8,
        egl::BLUE_SIZE, 8,
        egl::NONE
    ];
    egl::ChooseConfig(display,attrib_list.as_ptr() as *const EGLint, &mut config as *mut EGLConfig, 1, &mut num_configs);
    if egl::GetError() != egl::SUCCESS as i32 {
        return Err("failed to choose egl config");
    }
    debug!("chose egl config: {:?}", config);

    // create egl context
    let context = egl::CreateContext(display, config, egl::NO_CONTEXT, std::ptr::null());
    if context == egl::NO_CONTEXT {
        return Err("failed to create egl context");
    }
    debug!("created egl context: {:?}", context);

    // make egl context current
    egl::MakeCurrent(display, 0 as EGLSurface, 0 as EGLSurface, context);
    if egl::GetError() != egl::SUCCESS as i32 {
        return Err("failed to make egl context current");
    }
    debug!("made egl context current");

    // bind egl api
    egl::BindAPI(egl::OPENGL_API);
    if egl::GetError() != egl::SUCCESS as i32 {
        return Err("failed to bind egl api");
    }
    debug!("initialized opengl api");

    Ok((Environment::new(libegl, display, context, 0 as EGLSurface, native_display.as_ptr() as EGLNativeDisplayType), libgl))
}
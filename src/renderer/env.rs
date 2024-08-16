use std::error::Error;

use egls::{egl, so::SharedObject, EGLConfig, EGLNativeDisplayType, EGLSurface, EGLint, Environment};
use wayland_client::backend::ObjectId;

///
/// Helper function to create an environment with egl and gl functions loaded
///
/// # Arguments
///
/// * `native_display` - The native display object id
///
pub unsafe fn create_environment(native_display: ObjectId) -> Result<(Environment, SharedObject), Box<dyn Error>> {
    // load egl and gl functions
    let libegl = SharedObject::load("/usr/lib/libEGL.so");
    egl::load_with(|s| libegl.get_proc_address(s));
    let libgl = SharedObject::load("/usr/lib/libGL.so");
    gl::load_with(|s| libgl.get_proc_address(s));

    // get egl display
    let display = egl::GetDisplay(native_display.as_ptr() as EGLNativeDisplayType);
    if display == egl::NO_DISPLAY {
        return Err("failed to get egl display".into());
    }

    // initialize egl
    egl::Initialize(display, std::ptr::null_mut(), std::ptr::null_mut());
    if egl::GetError() != egl::SUCCESS as i32 {
        return Err("failed to initialize egl".into());
    }

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
        return Err("failed to choose egl config".into());
    }

    // create egl context
    let context = egl::CreateContext(display, config, egl::NO_CONTEXT, std::ptr::null());
    if context == egl::NO_CONTEXT {
        return Err("failed to create egl context".into());
    }

    // make egl context current
    egl::MakeCurrent(display, 0 as EGLSurface, 0 as EGLSurface, context);
    if egl::GetError() != egl::SUCCESS as i32 {
        return Err("failed to make egl context current".into());
    }

    // bind egl api
    egl::BindAPI(egl::OPENGL_API);
    if egl::GetError() != egl::SUCCESS as i32 {
        return Err("failed to bind egl api".into());
    }

    Ok((Environment::new(libegl, display, context, 0 as EGLSurface, native_display.as_ptr() as EGLNativeDisplayType), libgl))
}
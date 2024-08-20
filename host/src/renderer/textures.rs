use std::{os::fd::{AsRawFd, OwnedFd}, ptr};

use egls::{egl, EGLDisplay, EGLImageKHR};
use gl::types::{GLenum, GLint, GLuint};

///
/// OpenGL Texture
///
pub struct Texture {
    pub id: GLuint,
    pub image: Option<EGLImageKHR> // optional backing egl image
}

impl Texture {

    ///
    /// Create a new Texture from a dmabuf
    ///
    /// # Arguments
    ///
    /// * `dpy` - EGL Display
    /// * `dmabuf` - File descriptor of the dmabuf
    /// * `width` - Width of the texture
    /// * `height` - Height of the texture
    /// * `format` - Format of the dmabuf
    /// * `offset` - Offset of the dmabuf
    /// * `stride` - Stride of the dmabuf
    /// * `modifiers` - Modifiers of the dmabuf
    ///
    pub fn new_from_dmabuf(dpy: EGLDisplay, dmabuf: OwnedFd, width: u32, height: u32, format: u32, offset: u32, stride: u32, modifiers: u64) -> Result<Self, Box<dyn std::error::Error>> {
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
    }

    pub fn new(width: GLuint, height: GLuint) -> Self {
        let texture = unsafe { Texture::create_bound_texture(gl::TEXTURE_2D) };

        unsafe {
            gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA as GLint, width as i32, height as i32, 0, gl::RGBA, gl::UNSIGNED_BYTE, std::ptr::null());
            gl::BindTexture(gl::TEXTURE_2D, 0);
        }

        Texture {
            id: texture,
            image: None
        }
    }

    unsafe fn create_bound_texture(texture_type: GLenum) -> GLuint {
        // generate texture
        let mut texture = 0;
        gl::GenTextures(1, &mut texture);
        gl::BindTexture(texture_type, texture);

        // set texture parameters
        gl::TexParameteri(texture_type, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
        gl::TexParameteri(texture_type, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
        gl::TexParameteri(texture_type, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as GLint);
        gl::TexParameteri(texture_type, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as GLint);

        texture
    }

    ///
    /// Bind the texture
    ///
    pub fn bind(&self) {
        unsafe { gl::BindTexture(gl::TEXTURE_2D, self.id); }
    }

    ///
    /// Unbind the texture
    ///
    pub fn unbind(&self) {
        unsafe { gl::BindTexture(gl::TEXTURE_2D, 0); }
    }

}

impl Drop for Texture {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.id);
            if let Some(image) = self.image {
                egl2::DestroyImageKHR(egl::GetDisplay(egl::DEFAULT_DISPLAY), image);
            }
        }
    }
}

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

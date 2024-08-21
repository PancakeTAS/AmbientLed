use gl::types::GLuint;
use log::trace;

use super::textures::Texture;

///
/// OpenGL Framebuffer
///
pub struct Framebuffer {
    pub id: GLuint,
    pub color: Texture, // color texture
    pub width: GLuint,
    pub height: GLuint
}

impl Framebuffer {

    ///
    /// Create a new Framebuffer
    ///
    /// # Arguments
    ///
    /// * `width` - Width of the framebuffer
    /// * `height` - Height of the framebuffer
    ///
    pub fn new(width: GLuint, height: GLuint) -> Framebuffer {
        // create framebuffer
        let framebuffer = unsafe { Framebuffer::create_bound_framebuffer() };
        let color = Texture::new(width, height);
        trace!("created framebuffer: framebuffer={}, color={}", framebuffer, color.id);

        // attach color texture
        color.bind();
        unsafe { gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, color.id, 0) };
        color.unbind();

        // unbind framebuffer
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, 0);
        }

        Framebuffer {
            id: framebuffer,
            color,
            width,
            height
        }
    }

    unsafe fn create_bound_framebuffer() -> GLuint {
        let mut framebuffer = 0;
        gl::GenFramebuffers(1, &mut framebuffer);
        gl::BindFramebuffer(gl::FRAMEBUFFER, framebuffer);
        framebuffer
    }

    ///
    /// Bind the framebuffer
    ///
    pub fn bind(&self) {
        trace!("bound framebuffer: {}", self.id);
        unsafe {
            gl::BindFramebuffer(gl::FRAMEBUFFER, self.id);
            gl::Viewport(0, 0, self.width as i32, self.height as i32);
        }
    }

    ///
    /// Unbind the framebuffer (unused but kept for reference)
    ///
    pub fn unbind(&self) {
        trace!("unbound framebuffer: {}", self.id);
        unsafe { gl::BindFramebuffer(gl::FRAMEBUFFER, 0); }
    }

}

impl Drop for Framebuffer {
    fn drop(&mut self) {
        trace!("dropping framebuffer: id={}", self.id);
        unsafe {
            gl::DeleteFramebuffers(1, &self.id);
            gl::DeleteTextures(1, &self.color.id);
        }
    }
}

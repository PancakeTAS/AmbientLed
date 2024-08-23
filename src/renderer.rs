use std::{collections::HashMap, path::PathBuf};

use anyhow::{anyhow, Context};
use egls::{so::SharedObject, Environment};
use framebuffer::Framebuffer;
use log::{debug, trace};
use shaders::Shader;
use textures::Texture;
use vertices::VertexArrayObject;
use wayland_client::backend::ObjectId;

mod env;
mod framebuffer;
mod shaders;
mod textures;
mod vertices;

///
/// EGL-based led render pipeline
///
pub struct RenderPipeline {
    env: Environment,
    _libgl: SharedObject,

    textures: HashMap<u64, Texture>, // screen textures
    shader_program: HashMap<u64, (Shader, Framebuffer)>, // active shader program

    vertex_array: Option<VertexArrayObject>, // vertex array object
}

impl RenderPipeline {

    ///
    /// Create a new render pipeline
    ///
    /// # Arguments
    ///
    /// * `display` - The wayland display object
    ///
    /// # Errors
    ///
    /// This function will fail if the egl environment or required opengl objects cannot be created
    ///
    pub fn new(display: ObjectId) -> Result<Self, anyhow::Error> {
        // create environment
        let (env, _libgl) = unsafe { env::create_environment(display).map_err(|e| anyhow!(e))? };
        debug!("created egl & gl environment");

        // create vao
        let vertex_array = VertexArrayObject::new(
            &[
                /* positions  |  tex coords */
                1.0, 1.0, 0.0,    1.0, 1.0,
                1.0, -1.0, 0.0,   1.0, 0.0,
                -1.0, -1.0, 0.0,  0.0, 0.0,
                -1.0, 1.0, 0.0,   0.0, 1.0
            ],
            &[
                0, 1, 3,
                1, 2, 3
            ]
        ).map_err(|e| anyhow!(e))?;
        vertex_array.bind();
        debug!("created vertex array object");

        // setup opengl
        unsafe {
            gl::Enable(gl::TEXTURE_2D);
            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
        }

        Ok(Self {
            env,
            _libgl,
            textures: HashMap::new(),
            shader_program: HashMap::new(),
            vertex_array: Some(vertex_array)
        })
    }

    ///
    /// Update a specific screencopy texture with a buffer object
    ///
    /// # Arguments
    ///
    /// * `tid` - The texture id
    /// * `bo` - The buffer object
    ///
    /// # Errors
    ///
    /// This function will return an error if the texture cannot be created from the buffer object
    ///
    pub fn set_texture(&mut self, tid: u64, bo: &gbm::BufferObject<()>) -> Result<(), anyhow::Error> {
        let texture = Texture::new_from_dmabuf(
            self.env.get_display(),
            bo.fd_for_plane(0).unwrap(),
            bo.width().unwrap(),
            bo.height().unwrap(),
            bo.format().unwrap() as u32,
            bo.offset(0).unwrap(),
            bo.stride_for_plane(0).unwrap(),
            bo.modifier().unwrap().into()
        ).map_err(|e| anyhow!(e))?;
        debug!("created new texture from dmabuf: tid={}, bo={:?}", tid, bo);

        self.textures.insert(tid, texture);
        Ok(())
    }

    ///
    /// Update the shader program
    ///
    /// # Arguments
    ///
    /// * `sid` - The shader id
    /// * `tids` - The texture ids
    /// * `vert` - The vertex shader path
    /// * `frag` - The fragment shader path
    ///
    /// # Errors
    ///
    /// This function will return an error if the shader program cannot be created
    ///
    pub fn set_shader(&mut self, sid: u64, tids: &[u64], width: u32, height: u32, vert: &PathBuf, frag: &PathBuf) -> Result<(), anyhow::Error> {
        let shader_program = Shader::new(vert, frag, tids).context("failed to create shader program")?;
        let framebuffer = Framebuffer::new(width, height);
        debug!("created new shader program: sid={}, framebuffer={}", sid, framebuffer.id);

        self.shader_program.insert(sid, (shader_program, framebuffer));
        Ok(())
    }

    ///
    /// Render the pipeline, ensure the shader program has all the textures it needs
    ///
    /// # Arguments
    ///
    /// * `sid` - The shader id
    /// * `pixels` - The pixel buffer
    ///
    pub fn render(&self, sid: u64, pixels: &mut [u8]) {
        let (shader, framebuffer) = self.shader_program.get(&sid).unwrap();
        let textures = shader.tids.iter().map(|tid| self.textures.get(tid).unwrap()).collect::<Vec<&Texture>>();

        framebuffer.bind();

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        shader.bind(&textures);

        unsafe {
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, std::ptr::null());
            gl::Flush();
        }
        trace!("render pipeline rendered: sid={}, framebuffer={}", sid, framebuffer.id);

        shader.unbind(&textures);

        unsafe { gl::ReadPixels(0, 0, framebuffer.width as i32, framebuffer.height as i32, gl::RGB, gl::UNSIGNED_BYTE, pixels.as_mut_ptr() as *mut std::ffi::c_void); }
        trace!("read pixels from framebuffer: sid={}, framebuffer={}", sid, framebuffer.id);

        framebuffer.unbind();
    }

}

impl Drop for RenderPipeline {
    fn drop(&mut self) {
        debug!("dropping render pipeline, this will destroy all gl objects as well as the egl environment");
        self.vertex_array.as_ref().unwrap().unbind();
        self.textures.clear();
        self.shader_program.clear();
        self.vertex_array = None;
    }
}
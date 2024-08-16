use std::{collections::HashMap, error::Error, path::PathBuf};

use egls::{so::SharedObject, Environment};
use framebuffer::Framebuffer;
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
    shader_program: HashMap<u64, Shader>, // active shader program

    vertex_array: VertexArrayObject, // vertex array object
    framebuffer: Framebuffer // framebuffer
}

impl RenderPipeline {

    ///
    /// Create a new render pipeline
    ///
    /// # Arguments
    ///
    /// * `display` - The wayland display object
    /// * `width` - The width of the framebuffer
    /// * `height` - The height of the framebuffer
    ///
    pub fn new(display: ObjectId, width: u32, height: u32) -> Result<Self, Box<dyn Error>> {
        // create environment
        let (env, _libgl) = unsafe { env::create_environment(display)? };

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
        )?;
        vertex_array.bind();

        // create framebuffer
        let framebuffer = Framebuffer::new(width, height);
        framebuffer.bind();

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
            vertex_array,
            framebuffer
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
    pub fn set_texture(&mut self, tid: u64, bo: &gbm::BufferObject<()>) -> Result<(), Box<dyn Error>> {
        let texture = Texture::new_from_dmabuf(
            self.env.get_display(),
            bo.fd_for_plane(0)?,
            bo.width()?,
            bo.height()?,
            bo.format()? as u32,
            bo.offset(0)?,
            bo.stride_for_plane(0)?,
            bo.modifier()?.into()
        )?;

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
    pub fn set_shader(&mut self, sid: u64, tids: &[u64], vert: &PathBuf, frag: &PathBuf) -> Result<(), Box<dyn Error>> {
        let shader_program = Shader::new(vert, frag, tids)?;

        self.shader_program.insert(sid, shader_program);
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
    pub fn render(&self, sid: u64, pixels: &mut Vec<u8>) {
        let shader = self.shader_program.get(&sid).unwrap();
        let textures = shader.tids.iter().map(|tid| self.textures.get(tid).unwrap()).collect::<Vec<&Texture>>();

        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        shader.bind(&textures);

        unsafe {
            gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, std::ptr::null());
            gl::Flush();
        }

        shader.unbind(&textures);

        unsafe { gl::ReadPixels(0, 0, self.framebuffer.width as i32, self.framebuffer.height as i32, gl::RGB, gl::UNSIGNED_BYTE, pixels.as_mut_ptr() as *mut std::ffi::c_void); }
    }

}

impl Drop for RenderPipeline {
    fn drop(&mut self) {
        unsafe { // why not, lol
            gl::Disable(gl::TEXTURE_2D);

            self.vertex_array.unbind();
            self.framebuffer.unbind();
        }
    }
}
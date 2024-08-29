use std::{ffi::CString, fs, path::PathBuf, ptr};

use anyhow::{anyhow, Context};
use gl::types::{GLchar, GLenum, GLuint};
use log::{error, trace};

use super::textures::Texture;

///
/// OpenGL Shader Program
///
pub struct Shader {
    pub id: GLuint,
    pub tids: Vec<u64>,
    start_time: std::time::Instant,
    time_uniform: i32,
}

impl Shader {

    ///
    /// Create a new Shader Program
    ///
    /// # Arguments
    ///
    /// * `vertex_shader` - Path to the vertex shader
    /// * `fragment_shader` - Path to the fragment shader
    /// * `tids` - Texture IDs (do not have to exist yet)
    ///
    /// # Errors
    ///
    /// This function will return an error if the shaders cannot be read or fail to compile/link
    ///
    pub fn new(vertex_shader: &PathBuf, fragment_shader: &PathBuf, tids: &[u64]) -> Result<Self, anyhow::Error> {
        // read shader source
        let vertex_shader_source = fs::read_to_string(vertex_shader).context("failed to read vertex shader")?;
        trace!("read vertex shader: {:?}", vertex_shader);
        let fragment_shader_source = fs::read_to_string(fragment_shader).context("failed to read fragment shader")?;
        trace!("read fragment shader: {:?}", fragment_shader);

        // compile shaders
        let vertex_shader = unsafe { Shader::compile_shader(&vertex_shader_source, gl::VERTEX_SHADER).map_err(|e| anyhow!(e))? };
        trace!("compiled vertex shader: id={}", vertex_shader);
        let fragment_shader = unsafe { Shader::compile_shader(&fragment_shader_source, gl::FRAGMENT_SHADER).map_err(|e| anyhow!(e))? };
        trace!("compiled fragment shader: id={}", fragment_shader);

        // create shader program
        let id = unsafe { Shader::create_program(vertex_shader, fragment_shader).map_err(|e| anyhow!(e))? };
        trace!("created shader program: id={}", id);

        // delete shaders
        unsafe {
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
        }

        // set uniform locations
        let time_uniform = unsafe {
            gl::UseProgram(id);
            for i in 0..tids.len() {
                let c_texture_i = CString::new(format!("texture{}", i)).unwrap();
                gl::Uniform1i(gl::GetUniformLocation(id, c_texture_i.as_ptr()), i as i32);
            }

            let c_time = CString::new("time").unwrap();
            let time_uniform = gl::GetUniformLocation(id, c_time.as_ptr());

            gl::UseProgram(0);

            time_uniform
        };

        let start_time = std::time::Instant::now();
        Ok(Self { id, tids: tids.to_vec(), start_time, time_uniform })
    }

    unsafe fn compile_shader(source: &str, shader_type: GLenum) -> Result<GLuint, &'static str> {
        // create shader object
        let shader = gl::CreateShader(shader_type);
        if shader == 0 {
            return Err("failed to create shader object");
        }

        // compile shader
        let c_source = CString::new(source).map_err(|_| "shader source contained null bytes")?;
        gl::ShaderSource(shader, 1, &c_source.as_ptr(), ptr::null());
        gl::CompileShader(shader);

        // check for compilation errors
        let mut success = 0;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success);
        if success == gl::FALSE.into() {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buffer = vec![0; len as usize];
            gl::GetShaderInfoLog(shader, len, ptr::null_mut(), buffer.as_mut_ptr() as *mut GLchar);
            error!("shader compile failed: {}", String::from_utf8(buffer).unwrap());
            return Err("failed to compile shader");
        }

        Ok(shader)
    }

    unsafe fn create_program(vertex_shader: GLuint, fragment_shader: GLuint) -> Result<GLuint, &'static str> {
        // create shader program
        let id = gl::CreateProgram();
        if id == 0 {
            return Err("failed to create shader program");
        }

        // attach shaders
        gl::AttachShader(id, vertex_shader);
        gl::AttachShader(id, fragment_shader);

        // link shader program
        gl::LinkProgram(id);

        // check for linking errors
        let mut success = 0;
        gl::GetProgramiv(id, gl::LINK_STATUS, &mut success);
        if success == gl::FALSE.into() {
            let mut len = 0;
            gl::GetProgramiv(id, gl::INFO_LOG_LENGTH, &mut len);
            let mut buffer = vec![0; len as usize];
            gl::GetProgramInfoLog(id, len, ptr::null_mut(), buffer.as_mut_ptr() as *mut GLchar);
            error!("shader program link failed: {}", String::from_utf8(buffer).unwrap());
            return Err("failed to link shader program");
        }

        Ok(id)
    }

    ///
    /// Use the shader program
    ///
    /// # Arguments
    ///
    /// * `textures` - The textures to bind
    ///
    pub fn bind(&self, textures: &Vec<&Texture>) {
        if textures.len() != self.tids.len() {
            return;
        }

        trace!("bound shader program: {}", self.id);
        unsafe {
            gl::UseProgram(self.id);
            for (i, texture) in textures.iter().enumerate() {
                gl::ActiveTexture(gl::TEXTURE0 + i as u32);
                texture.bind();
            }

            let time = self.start_time.elapsed().as_secs_f32();
            gl::Uniform1f(self.time_uniform, time);
        }
    }

    ///
    /// Unuse the shader program
    ///
    /// # Arguments
    ///
    /// * `textures` - The textures to unbind
    ///
    pub fn unbind(&self, textures: &Vec<&Texture>) {
        if textures.len() != self.tids.len() {
            return;
        }

        trace!("unbound shader program: {}", self.id);
        unsafe {
            for (i, texture) in textures.iter().enumerate() {
                gl::ActiveTexture(gl::TEXTURE0 + i as u32);
                texture.unbind();
            }
            gl::UseProgram(0);
        }
    }

}

impl Drop for Shader {
    fn drop(&mut self) {
        trace!("dropping shader program: id={}", self.id);
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

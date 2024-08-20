use std::{error::Error, ffi::CString, fs, path::PathBuf, ptr};

use gl::types::{GLchar, GLenum, GLuint};

use super::textures::Texture;

///
/// OpenGL Shader Program
///
pub struct Shader {
    pub id: GLuint,
    pub tids: Vec<u64>,
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
    pub fn new(vertex_shader: &PathBuf, fragment_shader: &PathBuf, tids: &[u64]) -> Result<Self, Box<dyn Error>> {
        // read shader source
        let vertex_shader_source = fs::read_to_string(vertex_shader)?;
        let fragment_shader_source = fs::read_to_string(fragment_shader)?;

        // compile shaders
        let vertex_shader = unsafe { Shader::compile_shader(&vertex_shader_source, gl::VERTEX_SHADER) }?;
        let fragment_shader = unsafe { Shader::compile_shader(&fragment_shader_source, gl::FRAGMENT_SHADER) }?;

        // create shader program
        let id = unsafe { Shader::create_program(vertex_shader, fragment_shader) }?;

        // delete shaders
        unsafe {
            gl::DeleteShader(vertex_shader);
            gl::DeleteShader(fragment_shader);
        }

        // set uniform locations
        unsafe {
            gl::UseProgram(id);
            for i in 0..tids.len() {
                let c_texture_i = CString::new(format!("texture{}", i))?;
                gl::Uniform1i(gl::GetUniformLocation(id, c_texture_i.as_ptr()), i as i32);
            }
            gl::UseProgram(0);
        }

        Ok(Self { id, tids: tids.to_vec() })
    }

    unsafe fn compile_shader(source: &str, shader_type: GLenum) -> Result<GLuint, Box<dyn Error>> {
        // create shader object
        let shader = gl::CreateShader(shader_type);
        if shader == 0 { return Err("failed to create shader object".into()); }

        // compile shader
        let c_source = CString::new(source)?;
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
            println!("{}", String::from_utf8(buffer.clone())?);
            return Err(String::from_utf8(buffer)?.into());
        }

        Ok(shader)
    }

    unsafe fn create_program(vertex_shader: GLuint, fragment_shader: GLuint) -> Result<GLuint, Box<dyn Error>> {
        // create shader program
        let id = gl::CreateProgram();
        if id == 0 { return Err("failed to create shader program".into()); }

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
            println!("{}", String::from_utf8(buffer.clone())?);
            return Err(String::from_utf8(buffer)?.into());
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
        if textures.len() != self.tids.len() { return; }
        unsafe {
            gl::UseProgram(self.id);
            for (i, texture) in textures.iter().enumerate() {
                gl::ActiveTexture(gl::TEXTURE0 + i as u32);
                texture.bind();
            }
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
        if textures.len() != self.tids.len() { return; }
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
        unsafe { gl::DeleteProgram(self.id); }
    }
}

use super::config;
use gl::types::{GLchar, GLenum, GLint, GLuint};
use nalgebra::{Matrix3, Vector2};
use std::ffi::{CStr, CString};

pub struct Program {
    id: GLuint,
}

impl Program {
    pub fn from_shaders(shaders: &[Shader]) -> Result<Program, String> {
        let program_id = unsafe { gl::CreateProgram() };

        for shader in shaders {
            unsafe {
                gl::AttachShader(program_id, shader.id());
            }
        }

        unsafe {
            gl::LinkProgram(program_id);
        }

        let mut success: GLint = 1;
        unsafe {
            gl::GetProgramiv(program_id, gl::LINK_STATUS, &mut success);
        }

        if success == 0 {
            let mut len: GLint = 0;
            unsafe {
                gl::GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut len);
            }

            let error = create_whitespace_cstring_with_len(len as usize);

            unsafe {
                gl::GetProgramInfoLog(
                    program_id,
                    len,
                    std::ptr::null_mut(),
                    error.as_ptr() as *mut GLchar,
                );
            }

            return Err(error.to_string_lossy().into_owned());
        }

        for shader in shaders {
            unsafe {
                gl::DetachShader(program_id, shader.id());
            }
        }

        Ok(Program { id: program_id })
    }

    pub fn set_used(&self) {
        unsafe {
            gl::UseProgram(self.id);
        }
    }

    pub fn set_uniform_matrix(&self, name: &str, matrix: &nalgebra::Matrix3<f32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().into_raw());
            gl::UniformMatrix3fv(location, 1, gl::FALSE, matrix.as_ptr());
        }
    }

    pub fn set_uniform_vector(&self, name: &str, vector: &nalgebra::Vector2<f32>) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().into_raw());
            gl::Uniform2fv(location, 1, vector.as_ptr());
        }
    }

    pub fn set_uniform_texture(&self, name: &str, texture: i32) {
        unsafe {
            let location = gl::GetUniformLocation(self.id, CString::new(name).unwrap().into_raw());
            gl::Uniform1i(location, texture);
        }
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}

pub struct Shader {
    id: GLuint,
}

impl Shader {
    pub fn from_source(source: &CStr, kind: GLenum) -> Result<Shader, String> {
        let id = shader_from_source(source, kind)?;
        Ok(Shader { id })
    }

    pub fn from_vert_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::VERTEX_SHADER)
    }

    pub fn from_frag_source(source: &CStr) -> Result<Shader, String> {
        Shader::from_source(source, gl::FRAGMENT_SHADER)
    }

    pub fn id(&self) -> GLuint {
        self.id
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteShader(self.id);
        }
    }
}

pub fn shader_from_source(source: &CStr, kind: GLenum) -> Result<GLuint, String> {
    let id = unsafe { gl::CreateShader(kind) };
    unsafe {
        gl::ShaderSource(id, 1, &source.as_ptr(), std::ptr::null());
        gl::CompileShader(id);
    }

    let mut success: GLint = 1;
    unsafe {
        gl::GetShaderiv(id, gl::COMPILE_STATUS, &mut success);
    }

    if success == 0 {
        let mut len: GLint = 0;
        unsafe {
            gl::GetShaderiv(id, gl::INFO_LOG_LENGTH, &mut len);
        }

        let error = create_whitespace_cstring_with_len(len as usize);

        unsafe {
            gl::GetShaderInfoLog(id, len, std::ptr::null_mut(), error.as_ptr() as *mut GLchar);
        }

        return Err(error.to_string_lossy().into_owned());
    }

    Ok(id)
}

fn create_whitespace_cstring_with_len(len: usize) -> CString {
    // allocate buffer of correct size
    let mut buffer: Vec<u8> = Vec::with_capacity(len + 1);
    // fill it with len spaces
    buffer.extend([b' '].iter().cycle().take(len));
    // convert buffer to CString
    unsafe { CString::from_vec_unchecked(buffer) }
}

#[derive(Copy, Clone, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl From<&config::Color> for Color {
    fn from(color: &config::Color) -> Self {
        Color {
            r: color.rgb[0] as f32 / 255.0,
            g: color.rgb[1] as f32 / 255.0,
            b: color.rgb[2] as f32 / 255.0,
            a: color.opacity.unwrap_or(1.0),
        }
    }
}

pub fn initialize_shaders() -> Program {
    let vert_shader =
        Shader::from_vert_source(&CString::new(include_str!("shaders/basic.vert")).unwrap())
            .unwrap();

    let frag_shader =
        Shader::from_frag_source(&CString::new(include_str!("shaders/basic.frag")).unwrap())
            .unwrap();

    Program::from_shaders(&[vert_shader, frag_shader]).unwrap()
}

pub fn initialize_text_shaders() -> Program {
    let vert_shader =
        Shader::from_vert_source(&CString::new(include_str!("shaders/text.vert")).unwrap())
            .unwrap();

    let frag_shader =
        Shader::from_frag_source(&CString::new(include_str!("shaders/text.frag")).unwrap())
            .unwrap();

    Program::from_shaders(&[vert_shader, frag_shader]).unwrap()
}

pub fn initialize_window(
    width: u32,
    height: u32,
    name: &str,
    background_color: Color,
) -> (
    glfw::Window,
    glfw::Glfw,
    std::sync::mpsc::Receiver<(f64, glfw::WindowEvent)>,
) {
    let glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    let (mut window, events) = glfw
        .create_window(width, height, name, glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    use glfw::Context;
    window.make_current();
    window.set_key_polling(true);

    gl_loader::init_gl();
    gl::load_with(|symbol| gl_loader::get_proc_address(symbol) as *const _);

    unsafe {
        gl::Enable(gl::BLEND);
        gl::BlendFunci(0, gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
        gl::PixelStorei(gl::UNPACK_ALIGNMENT, 1);

        gl::ClearColor(
            background_color.r,
            background_color.g,
            background_color.b,
            background_color.a,
        );
    }

    (window, glfw, events)
}

pub fn transformation_matrix(translation: [f32; 2], scale: [f32; 2]) -> Matrix3<f32> {
    Matrix3::new_nonuniform_scaling(&Vector2::new(scale[0], scale[1]))
        * (Matrix3::new_translation(&Vector2::new(translation[0], translation[1])))
}

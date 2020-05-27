use std::ptr;
use std::str;
use std::ffi::{ CString, c_void };
use std::mem;
use gl::types::*;

use crate::window::Window;

const VERTEX_SHADER : &str = r#"#version 330 core
layout (location = 0) in vec3 aPos;
layout (location = 1) in vec2 aTexCoord;

out vec2 TexCoord;

void main()
{
    gl_Position = vec4(aPos, 1.0);
    TexCoord = vec2(aTexCoord.x, 1.0 - aTexCoord.y);
}"#;
const FRAGMENT_SHADER : &str = r#"#version 330 core
out vec4 FragColor;

in vec2 TexCoord;

uniform sampler2D texture1;

void main()
{
    FragColor = texture(texture1, TexCoord);
}"#;

pub struct GlRenderer {
    program : GlProgram,
    vao : GLuint,
    texture : GLuint,
    window : Window,
}

impl GlRenderer {
    pub fn new(window : Window) -> GlRenderer {
        gl::load_with(|symbol| window.windowed_context.get_proc_address(symbol) as *const _);

        let (gl_program, _vbo, vao, _ebo, texture) = unsafe {
            let gl_program = create_gl_program().unwrap_or_else(| err | {
                eprintln!("Error while creating gl Program: {}", err);
                std::process::exit(1);
            });

            // set up vertex data (and buffer(s)) and configure vertex attributes
            // ------------------------------------------------------------------
            // HINT: type annotation is crucial since default for float literals is f64
            let vertices: [GLfloat; 32] = [
                // positions       // colors        // texture coords
                1.0,  1.0, 0.0,   1.0, 0.0, 0.0,   1.0, 1.0, // top right
                1.0, -1.0, 0.0,   0.0, 1.0, 0.0,   1.0, 0.0, // bottom right
                -1.0, -1.0, 0.0,   0.0, 0.0, 1.0,   0.0, 0.0, // bottom left
                -1.0,  1.0, 0.0,   1.0, 1.0, 0.0,   0.0, 1.0  // top left
            ];
            let indices = [
                0, 1, 3,  // first Triangle
                1, 2, 3   // second Triangle
            ];
            let (mut vbo, mut vao, mut ebo) = (0, 0, 0);
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);
            gl::GenBuffers(1, &mut ebo);

            gl::BindVertexArray(vao);

            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (vertices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                &vertices[0] as *const GLfloat as *const c_void,
                gl::STATIC_DRAW);

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(gl::ELEMENT_ARRAY_BUFFER,
                (indices.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
                &indices[0] as *const i32 as *const c_void,
                gl::STATIC_DRAW);

            let stride = 8 * mem::size_of::<GLfloat>() as GLsizei;
            // position attribute
            gl::VertexAttribPointer(0, 3, gl::FLOAT, gl::FALSE, stride, ptr::null());
            gl::EnableVertexAttribArray(0);
            // texture coord attribute
            gl::VertexAttribPointer(1, 2, gl::FLOAT, gl::FALSE, stride, (6 * mem::size_of::<GLfloat>()) as *const c_void);
            gl::EnableVertexAttribArray(1);

            let mut texture = 0;
            gl::GenTextures(1, &mut texture);
            gl::BindTexture(gl::TEXTURE_2D, texture);
            // set the texture wrapping parameters
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32);

            // set texture filtering parameters
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
            gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);
            // gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);
            // gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
            clear_gl_color();

            (gl_program, vbo, vao, ebo, texture)
        };
        window.refresh();
        GlRenderer {
            program: gl_program,
            vao,
            texture,
            window
        }
    }

//     /// Change the dimensions of the window
//     pub fn update_window_size(&self, width : u32, height : u32) {
//         self.window.update_window_size(width, height);
//     }

    pub unsafe fn redraw(&self) {
        clear_gl_color();
        gl::BindTexture(gl::TEXTURE_2D, self.texture);
        self.program.use_program();
        gl::BindVertexArray(self.vao);
        gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, ptr::null());
        self.window.refresh();
    }

    pub unsafe fn resize(&self, width : u32, height : u32) {
        let (initial_width, initial_height) = (
            self.window.base_width as f64,
            self.window.base_height as f64);
        let initial_ratio = initial_width / initial_height;
        let new_ratio = width as f64 / height as f64;
        if new_ratio == initial_ratio {
            gl::Viewport(0, 0, width as i32, height as i32);
        } else if new_ratio > initial_ratio {
            // bigger width
            let new_height = height as f64;
            let new_width = initial_ratio * new_height;
            let width_offset = (width as f64 - new_width) / 2.0;
            gl::Viewport(width_offset as i32, 0, new_width as i32, new_height as i32);
        } else {
            // bigger height
            let new_width = width as f64;
            let new_height = new_width / initial_ratio;
            let height_offset = (height as f64 - new_height) / 2.0;
            gl::Viewport(0, height_offset as i32, new_width as i32, new_height as i32);
        }
        self.redraw();
    }

    pub unsafe fn draw(&self, data : &[u32]) {
        clear_gl_color();
        // let window_size = self.window.get_inner_size();
        gl::TexImage2D(gl::TEXTURE_2D,
            0,
            gl::RGBA as i32,
            self.window.base_width as i32,
            self.window.base_height as i32,
            // window_size.width as i32,
            // window_size.height as i32,
            0,
            gl::RGBA,
            gl::UNSIGNED_BYTE,
            &data[0] as *const u32 as *const c_void);

        gl::BindTexture(gl::TEXTURE_2D, self.texture);

        // render container
        self.program.use_program();
        gl::BindVertexArray(self.vao);
        gl::DrawElements(gl::TRIANGLES, 6, gl::UNSIGNED_INT, ptr::null());
        self.window.refresh();
    }
}

pub unsafe fn clear_gl_color() {
    gl::ClearColor(0., 0., 0., 1.);
    check_gl_error("ClearColor");

    gl::Clear(gl::COLOR_BUFFER_BIT);
    check_gl_error("Clear");
}

fn check_gl_error(source: &str) {
    let err = unsafe { gl::GetError() };
    if err != gl::NO_ERROR {
        eprintln!("GL error [{}]: {:?}", source, err);
    }
}

pub fn create_gl_program() -> Result<GlProgram, String> {
    let vertex_shader = CString::new(VERTEX_SHADER.as_bytes()).unwrap();
    let fragment_shader = CString::new(FRAGMENT_SHADER.as_bytes()).unwrap();

    let mut shaders : Vec<GlShader> = Vec::with_capacity(2);
    shaders.push(GlShader::from_vert_source(&vertex_shader)?);
    shaders.push(GlShader::from_frag_source(&fragment_shader)?);
    let gl_program = GlProgram::from_shaders(&shaders)?;
    Ok(gl_program)
}

// pub fn load_gl_symbols(ctxt : &glutin::WindowedContext<glutin::PossiblyCurrent>) {
//     gl::load_with(|symbol| ctxt.get_proc_address(symbol) as *const _);
// }

fn create_placeholder_cstring(len: usize) -> CString {
    let mut buffer: Vec<u8> = vec![0; len + 1];
    buffer.extend([b' '].iter().cycle().take(len));
    unsafe { CString::from_vec_unchecked(buffer) }
}

pub struct GlProgram {
    pub id: gl::types::GLuint,
}

impl GlProgram {
    pub fn from_shaders(shaders: &[GlShader]) -> Result<GlProgram, String> {
        let program_id = unsafe { gl::CreateProgram() };

        for shader in shaders {
            unsafe { gl::AttachShader(program_id, shader.id); }
        }

        unsafe { gl::LinkProgram(program_id); }

        let mut success: gl::types::GLint = 1;
        unsafe { gl::GetProgramiv(program_id, gl::LINK_STATUS, &mut success); }

        if success != gl::TRUE as GLint {
            let mut len: gl::types::GLint = 0;
            unsafe { gl::GetProgramiv(program_id, gl::INFO_LOG_LENGTH, &mut len); }
            let error = create_placeholder_cstring(len as usize);

            unsafe {
                gl::GetProgramInfoLog(
                    program_id,
                    len,
                    std::ptr::null_mut(),
                    error.as_ptr() as *mut gl::types::GLchar);
            }

            return Err(error.to_string_lossy().into_owned());
        }

        for shader in shaders {
            unsafe { gl::DetachShader(program_id, shader.id); }
        }

        Ok(GlProgram { id: program_id })
    }

    pub unsafe fn use_program(&self) {
        gl::UseProgram(self.id);
    }
}

impl Drop for GlProgram {
    fn drop(&mut self) {
        unsafe { gl::DeleteProgram(self.id); }
    }
}

pub struct GlShader {
    pub id: gl::types::GLuint,
}

impl GlShader {
    pub fn from_source(source: &CString, shader_type: gl::types::GLenum) -> Result<GlShader, String> {
        let shader = unsafe { gl::CreateShader(shader_type) };
        unsafe {
            gl::ShaderSource(shader, 1, &source.as_ptr(), ptr::null());
            gl::CompileShader(shader);
        };

        let mut success : gl::types::GLint = 0;
        unsafe { gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut success); }

        if success == 0 {
            let mut len: gl::types::GLint = 0;
            unsafe { gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len); }
            let error = create_placeholder_cstring(len as usize);
            unsafe {
                gl::GetShaderInfoLog(shader,
                    len,
                    std::ptr::null_mut(),
                    error.as_ptr() as *mut gl::types::GLchar);
            }

            return Err(error.to_string_lossy().into_owned());
        }

        Ok(GlShader { id: shader })
    }

    pub fn from_vert_source(source: &CString) -> Result<GlShader, String> {
        GlShader::from_source(source, gl::VERTEX_SHADER)
    }

    pub fn from_frag_source(source: &CString) -> Result<GlShader, String> {
        GlShader::from_source(source, gl::FRAGMENT_SHADER)
    }
}

impl Drop for GlShader {
    fn drop(&mut self) {
        // DeleteShader actually only flag for deletion if the shader is in use
        // by a program.
        // We have thus no risk deleting it as soon as it goes out of scope.
        unsafe { gl::DeleteShader(self.id); }
    }
}


use super::renderer;
use gl::types::{GLint, GLsizeiptr, GLuint, GLvoid};
use rusttype::{point, Font, Scale};

#[derive(Copy, Clone, Debug)]
struct Vertex {
    x: f32,
    y: f32,
    tx: f32,
    ty: f32,
    color: renderer::Color,
}
pub struct Text {
    vertices: Vec<Vertex>,
    vao: GLuint,
    vbo: GLuint,
    texture: GLuint,
}

pub enum Orientation {
    Horizontal,
    Vertical,
}

impl Drop for Text {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.vbo);
            gl::DeleteVertexArrays(1, &mut self.vao);
            gl::DeleteTextures(1, &mut self.texture);
        }
    }
}

fn generate_buffers(vertices: &Vec<Vertex>) -> (GLuint, GLuint, GLuint) {
    let mut vbo: GLuint = 0;
    let mut texture: GLuint = 0;
    unsafe {
        gl::GenBuffers(1, &mut vbo);
        gl::GenTextures(1, &mut texture);
    }

    unsafe {
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<f32>() * 8) as GLsizeiptr,
            vertices.as_ptr() as *const GLvoid,
            gl::STATIC_DRAW,
        );
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    }

    let mut vao: GLuint = 0;
    unsafe {
        gl::GenVertexArrays(1, &mut vao);
    }

    unsafe {
        gl::BindVertexArray(vao);
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

        gl::EnableVertexAttribArray(0);
        gl::VertexAttribPointer(
            0,
            2,
            gl::FLOAT,
            gl::FALSE,
            (8 * std::mem::size_of::<f32>()) as GLint,
            std::ptr::null(),
        );

        gl::EnableVertexAttribArray(1);
        gl::VertexAttribPointer(
            1,
            2,
            gl::FLOAT,
            gl::FALSE,
            (8 * std::mem::size_of::<f32>()) as gl::types::GLint,
            (2 * std::mem::size_of::<f32>()) as *const core::ffi::c_void,
        );

        gl::EnableVertexAttribArray(2);
        gl::VertexAttribPointer(
            2,
            4,
            gl::FLOAT,
            gl::FALSE,
            (8 * std::mem::size_of::<f32>()) as gl::types::GLint,
            (4 * std::mem::size_of::<f32>()) as *const core::ffi::c_void,
        );

        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);
    }

    (vao, vbo, texture)
}

pub fn generate_text(
    cx: f32,
    cy: f32,
    text: &str,
    scale: f32,
    font: &Font,
    width: u32,
    height: u32,
    color: renderer::Color,
    orientation: Orientation,
) -> Text {
    let scale = Scale::uniform(scale);
    let v_metrics = font.v_metrics(scale);

    let padding = 20usize;

    let glyphs: Vec<_> = font
        .layout(
            text,
            scale,
            point(padding as f32, padding as f32 + v_metrics.ascent),
        )
        .collect();

    let glyphs_height = (v_metrics.ascent - v_metrics.descent).ceil() as u32;
    let glyphs_width = {
        let min_x = glyphs
            .first()
            .map(|g| g.pixel_bounding_box().unwrap().min.x)
            .unwrap();
        let max_x = glyphs
            .last()
            .map(|g| g.pixel_bounding_box().unwrap().max.x)
            .unwrap();
        (max_x - min_x) as u32
    };

    let mut texture_data = nalgebra::DMatrix::<u8>::zeros(
        glyphs_width as usize + 2 * padding,
        glyphs_height as usize + 2 * padding,
    );

    for glyph in glyphs {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, v| {
                texture_data[(
                    (x + bounding_box.min.x as u32) as usize,
                    if let Orientation::Vertical = orientation {
                        (y + bounding_box.min.y as u32) as usize
                    } else {
                        (glyphs_height + 2 * padding as u32 - (y + bounding_box.min.y as u32))
                            as usize
                    },
                )] = (v * 255.0) as u8;
            });
        }
    }

    let (rect_width, rect_height) = match orientation {
        Orientation::Horizontal => (
            (glyphs_width as usize + 2 * padding) as f32 / width as f32,
            (glyphs_height as usize + 2 * padding) as f32 / height as f32,
        ),
        Orientation::Vertical => {
            texture_data = texture_data.transpose();
            (
                (glyphs_height as usize + 2 * padding) as f32 / width as f32,
                (glyphs_width as usize + 2 * padding) as f32 / height as f32,
            )
        }
    };

    let bl = Vertex {
        x: cx - rect_width / 2.0,
        y: cy - rect_height / 2.0,
        tx: 0.0,
        ty: 0.0,
        color,
    };

    let tl = Vertex {
        x: cx - rect_width / 2.0,
        y: cy + rect_height / 2.0,
        tx: 0.0,
        ty: 1.0,
        color,
    };

    let tr = Vertex {
        x: cx + rect_width / 2.0,
        y: cy + rect_height / 2.0,
        tx: 1.0,
        ty: 1.0,
        color,
    };

    let br = Vertex {
        x: cx + rect_width / 2.0,
        y: cy - rect_height / 2.0,
        tx: 1.0,
        ty: 0.0,
        color,
    };

    let vertices = vec![bl, tl, tr, tr, br, bl];

    let (vao, vbo, texture) = generate_buffers(&vertices);

    let (glyphs_width, glyphs_height) = match orientation {
        Orientation::Horizontal => (glyphs_width, glyphs_height),
        Orientation::Vertical => (glyphs_height, glyphs_width),
    };

    unsafe {
        gl::BindTexture(gl::TEXTURE_2D, texture);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::LINEAR as i32);

        gl::TexImage2D(
            gl::TEXTURE_2D,
            0,
            gl::RED as i32,
            (glyphs_width as usize + 2 * padding) as i32,
            (glyphs_height as usize + 2 * padding) as i32,
            0,
            gl::RED,
            gl::UNSIGNED_BYTE,
            texture_data.data.as_vec().as_ptr() as *const GLvoid,
        );

        gl::BindTexture(gl::TEXTURE_2D, 0);
    }

    Text {
        vertices,
        vao,
        vbo,
        texture,
    }
}

pub fn draw_text(text: &Text, width: u32, height: u32) {
    unsafe {
        gl::Viewport(0, 0, width as i32, height as i32);

        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture(gl::TEXTURE_2D, text.texture);
        gl::BindVertexArray(text.vao);
        gl::LineWidth(1.0);

        gl::DrawArrays(gl::TRIANGLES, 0, text.vertices.len() as i32);

        gl::BindTexture(gl::TEXTURE_2D, 0);
    }
}

use super::renderer;
use gl::types::{GLint, GLsizeiptr, GLuint, GLvoid};
use ordered_float::OrderedFloat;
use std::collections::BTreeMap;

#[derive(Clone)]
struct Vertex {
    x: f32,
    y: f32,
    color: renderer::Color,
}
pub struct Mesh {
    vertices: Vec<Vertex>,
    vao: GLuint,
    vbo: GLuint,
}

impl Drop for Mesh {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &mut self.vbo);
            gl::DeleteVertexArrays(1, &mut self.vao);
        }
    }
}

fn generate_buffers(vertices: &Vec<Vertex>) -> (GLuint, GLuint) {
    let mut vbo: GLuint = 0;
    unsafe {
        gl::GenBuffers(1, &mut vbo);
    }

    unsafe {
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<f32>() * 6) as GLsizeiptr,
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
            (6 * std::mem::size_of::<f32>()) as GLint,
            std::ptr::null(),
        );

        gl::EnableVertexAttribArray(1);
        gl::VertexAttribPointer(
            1,
            4,
            gl::FLOAT,
            gl::FALSE,
            (6 * std::mem::size_of::<f32>()) as gl::types::GLint,
            (2 * std::mem::size_of::<f32>()) as *const core::ffi::c_void,
        );

        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        gl::BindVertexArray(0);
    }

    (vao, vbo)
}

pub fn generate_polynomial_graph(
    coefficients: &Vec<f32>,
    range: &std::ops::Range<OrderedFloat<f32>>,
    color: renderer::Color,
) -> Mesh {
    let mut vertices = vec![];

    for x in -1000..1000 {
        let x = (((x as f32 - -1000.0) * (range.end - range.start).into_inner())
            / (1000.0 - -1000.0))
            + range.start.into_inner();

        let mut y = 0.0;

        for (i, coefficient) in coefficients.iter().enumerate() {
            y += x.powf(i as f32) * coefficient;
        }

        vertices.push(Vertex { x, y, color });
    }

    let (vao, vbo) = generate_buffers(&vertices);
    Mesh { vertices, vao, vbo }
}

pub fn generate_grid(time_divisions: u32, data_divisions: u32, color: renderer::Color) -> Mesh {
    let mut vertices = vec![];

    for i in 0..=time_divisions {
        let offset = if i == 0 { 0.0001 } else { 0.0 };

        vertices.push(Vertex {
            x: (2.0f32 / time_divisions as f32 * i as f32 - 1.0) + offset,
            y: -1.0,
            color,
        });
        vertices.push(Vertex {
            x: (2.0f32 / time_divisions as f32 * i as f32 - 1.0) + offset,
            y: 1.0,
            color,
        });
    }

    for i in 0..=data_divisions {
        let offset = if i == data_divisions {
            0.0001 as f32
        } else {
            0.0
        };

        vertices.push(Vertex {
            x: -1.0,
            y: (2.0f32 / data_divisions as f32 * i as f32 - 1.0) - offset,
            color,
        });
        vertices.push(Vertex {
            x: 1.0,
            y: (2.0f32 / data_divisions as f32 * i as f32 - 1.0) - offset,
            color,
        });
    }

    let (vao, vbo) = generate_buffers(&vertices);

    Mesh { vertices, vao, vbo }
}

pub fn get_dimensions(
    width: u32,
    height: u32,
    padding: u32,
    time_divisions: u32,
    data_divisions: u32,
) -> (i32, i32, i32, i32) {
    // Coefficients for making grid square.
    let mut scaling_coefficient_x = 1.0;
    let mut scaling_coefficient_y = 1.0;

    if width as f32 / time_divisions as f32 > height as f32 / data_divisions as f32 {
        scaling_coefficient_x =
            width as f32 / time_divisions as f32 * data_divisions as f32 / height as f32;
    } else {
        scaling_coefficient_y =
            time_divisions as f32 / width as f32 * height as f32 / data_divisions as f32;
    }

    let new_width = (width - 2 * padding) as f32;
    let new_scaled_width = (width - 2 * padding) as f32 / scaling_coefficient_x;

    let new_height = (height - 2 * padding) as f32;
    let new_scaled_height = (height - 2 * padding) as f32 / scaling_coefficient_y;

    (
        padding as i32 + ((new_width - new_scaled_width) / 2.0) as i32,
        padding as i32 + ((new_height - new_scaled_height) / 2.0) as i32,
        new_scaled_width as i32,
        new_scaled_height as i32,
    )
}

pub fn generate_graphs(
    data: &BTreeMap<OrderedFloat<f32>, Vec<f32>>,
    range: &std::ops::Range<OrderedFloat<f32>>,
    channels: usize,
    colors: &Vec<renderer::Color>,
    subrange: &Option<std::ops::Range<OrderedFloat<f32>>>,
    focused_channel: &Option<usize>,
) -> Vec<Mesh> {
    let mut vertices: Vec<Vec<Vertex>> = vec![vec![]; channels];

    for (time, data) in data.range(range.to_owned()) {
        for (i, y) in data.iter().enumerate() {
            let mut focused = true;

            if let (Some(subrange), Some(j)) = (&subrange, focused_channel) {
                focused = subrange.contains(time) && i == *j;
            } else if let Some(subrange) = &subrange {
                focused = subrange.contains(time);
            } else if let Some(j) = focused_channel {
                focused = i == *j;
            }

            vertices[i].push(Vertex {
                x: time.into_inner(),
                y: *y,
                color: match focused {
                    false => renderer::Color {
                        r: colors[i].r,
                        g: colors[i].g,
                        b: colors[i].b,
                        a: colors[i].a * 0.2,
                    },
                    true => colors[i],
                },
            });
        }
    }

    let mut result = vec![];
    for i in 0..channels {
        let (vao, vbo) = generate_buffers(&vertices[i]);
        result.push(Mesh {
            vertices: vertices[i].clone(),
            vao,
            vbo,
        })
    }

    result
}

pub fn draw_grid(
    grid: &Mesh,
    width: u32,
    height: u32,
    padding: u32,
    time_divisions: u32,
    data_divisions: u32,
) {
    let (x, y, width, height) =
        get_dimensions(width, height, padding, time_divisions, data_divisions);

    unsafe {
        gl::Viewport(x, y, width, height);
        gl::BindVertexArray(grid.vao);
        gl::LineWidth(1.0);
        gl::DrawArrays(gl::LINES, 0, grid.vertices.len() as i32);
    }
}

pub fn draw_graph(graph: &Mesh) {
    unsafe {
        gl::BindVertexArray(graph.vao);
        gl::LineWidth(3.0);
        gl::DrawArrays(gl::LINE_STRIP, 0, graph.vertices.len() as i32);
    }
}

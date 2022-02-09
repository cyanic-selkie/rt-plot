use crate::renderer;
use std::vec;

#[derive(Copy, Clone)]
pub struct Vertex {
    x: f32,
    y: f32,
    color: renderer::Color,
}

pub struct Rectangle {
    pub vertices: vec::Vec<Vertex>,
    vao: gl::types::GLuint,
    vbo: gl::types::GLuint,
}

pub struct Grid {
    pub vertices: vec::Vec<Vertex>,
    vao: gl::types::GLuint,
    vbo: gl::types::GLuint,

    milliseconds_per_division: u32,
    x_divisions: u32,

    units_per_division: u32,
    y_divisions: u32,
}

pub struct Graph {
    pub vertices: vec::Vec<Vertex>,
    vao: gl::types::GLuint,
    vbo: gl::types::GLuint,
}

#[derive(Clone)]
pub struct DataPoint {
    pub y: Vec<f64>,
    pub x: u64,
}

fn transform_range(value: f32, from_min: f32, from_max: f32, to_min: f32, to_max: f32) -> f32 {
    (value - from_min) * (to_max - to_min) / (from_max - from_min) + to_min as f32
}

fn generate_buffers(vertices: &vec::Vec<Vertex>) -> (gl::types::GLuint, gl::types::GLuint) {
    let mut vbo: gl::types::GLuint = 0;
    unsafe {
        gl::GenBuffers(1, &mut vbo);
    }

    unsafe {
        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferData(
            gl::ARRAY_BUFFER,
            (vertices.len() * std::mem::size_of::<f32>() * 6) as gl::types::GLsizeiptr,
            vertices.as_ptr() as *const gl::types::GLvoid,
            gl::STATIC_DRAW,
        );
        gl::BindBuffer(gl::ARRAY_BUFFER, 0);
    }

    let mut vao: gl::types::GLuint = 0;
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
            (6 * std::mem::size_of::<f32>()) as gl::types::GLint,
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

pub fn normalize_datapoint(
    data_point: &DataPoint,
    grid: &Grid,
    delta: u64,
) -> (f32, vec::Vec<f32>) {
    (
        transform_range(
            data_point.x as f32 - delta as f32,
            -(grid.x_divisions as f32 * grid.milliseconds_per_division as f32),
            0.0,
            -1.0,
            1.0,
        ),
        data_point
            .y
            .iter()
            .map(|y| {
                transform_range(
                    y.clone() as f32,
                    (-(grid.y_divisions as f32) / 2.0 * grid.units_per_division as f32) as f32,
                    ((grid.y_divisions as f32) / 2.0 * grid.units_per_division as f32) as f32,
                    -1.0,
                    1.0,
                )
            })
            .collect(),
    )
}

pub fn generate_polynomial_graph(coefficients: &vec::Vec<f32>, color: renderer::Color) -> Graph {
    let mut vertices = vec![];

    for x in -1000..1000 {
        let x = x as f32 * 0.001;

        let mut y = 0.0;

        for (i, coefficient) in coefficients.iter().enumerate() {
            y += x.powf(i as f32) * coefficient;
        }

        vertices.push(Vertex { x, y, color });
    }

    let (vao, vbo) = generate_buffers(&vertices);
    Graph { vertices, vao, vbo }
}

pub fn generate_graphs(
    data: &vec::Vec<DataPoint>,
    grid: &Grid,
    delta: u64,
    colors: &vec::Vec<renderer::Color>,
) -> vec::Vec<Graph> {
    let channels = data[0].y.len();
    let mut vertices: vec::Vec<vec::Vec<Vertex>> = vec![vec![]; channels];

    for data_point in data.iter().rev() {
        let (x, y) = normalize_datapoint(&data_point, &grid, delta);

        if x < -1.0 {
            break;
        }

        for (i, y) in y.iter().enumerate() {
            vertices[i].push(Vertex {
                x,
                y: y.clone(),
                color: colors[i],
            });
        }
    }

    let mut result = vec![];
    for i in 0..channels {
        let (vao, vbo) = generate_buffers(&vertices[i]);
        result.push(Graph {
            vertices: vertices[i].clone(),
            vao,
            vbo,
        })
    }

    result
}

pub fn generate_grid(
    x_divisions: u32,
    y_divisions: u32,
    width: u32,
    height: u32,
    milliseconds_per_division: u32,
    units_per_division: u32,
    color: renderer::Color,
) -> Grid {
    let mut vertices = vec![];

    for i in 0..=x_divisions {
        let offset = if i == 0 { 1.0 / width as f32 } else { 0.0 };

        vertices.push(Vertex {
            x: (2.0f32 / x_divisions as f32 * i as f32 - 1.0) + offset,
            y: -1.0,
            color,
        });
        vertices.push(Vertex {
            x: (2.0f32 / x_divisions as f32 * i as f32 - 1.0) + offset,
            y: 1.0,
            color,
        });
    }

    for i in 0..=y_divisions {
        let offset = if i == y_divisions {
            1.0 / height as f32
        } else {
            0.0
        };

        vertices.push(Vertex {
            x: -1.0,
            y: (2.0f32 / y_divisions as f32 * i as f32 - 1.0) - offset,
            color,
        });
        vertices.push(Vertex {
            x: 1.0,
            y: (2.0f32 / y_divisions as f32 * i as f32 - 1.0) - offset,
            color,
        });
    }

    let (vao, vbo) = generate_buffers(&vertices);

    Grid {
        vertices,
        vao,
        vbo,
        milliseconds_per_division,
        x_divisions,
        units_per_division,
        y_divisions,
    }
}

pub fn draw_grid(grid: &Grid) {
    unsafe {
        gl::BindVertexArray(grid.vao);
        gl::LineWidth(1.0);
        gl::DrawArrays(gl::LINES, 0, grid.vertices.len() as i32);
    }
}

pub fn draw_graph(graph: &Graph, thickness: f32) {
    unsafe {
        gl::BindVertexArray(graph.vao);
        gl::LineWidth(thickness);
        gl::DrawArrays(gl::LINE_STRIP, 0, graph.vertices.len() as i32);
    }
}

pub fn draw_rectangle(rectangle: &Rectangle) {
    unsafe {
        gl::BindVertexArray(rectangle.vao);
        gl::LineWidth(1.0);
        gl::DrawArrays(gl::QUADS, 0, rectangle.vertices.len() as i32);
    }
}

pub fn generate_bound_rectangles(bounds: &(f32, f32), alpha: f32) -> (Rectangle, Rectangle) {
    let left_vertices = vec![
        Vertex {
            x: -1.0,
            y: 1.0,
            color: renderer::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
        },
        Vertex {
            x: bounds.0,
            y: 1.0,
            color: renderer::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
        },
        Vertex {
            x: bounds.0,
            y: -1.0,
            color: renderer::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
        },
        Vertex {
            x: -1.0,
            y: -1.0,
            color: renderer::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
        },
    ];

    let right_vertices = vec![
        Vertex {
            x: bounds.1,
            y: 1.0,
            color: renderer::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
        },
        Vertex {
            x: 1.0,
            y: 1.0,
            color: renderer::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
        },
        Vertex {
            x: 1.0,
            y: -1.0,
            color: renderer::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
        },
        Vertex {
            x: bounds.1,
            y: -1.0,
            color: renderer::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
        },
    ];

    let (left_vao, left_vbo) = generate_buffers(&left_vertices);
    let (right_vao, right_vbo) = generate_buffers(&right_vertices);

    (
        Rectangle {
            vertices: left_vertices,
            vao: left_vao,
            vbo: left_vbo,
        },
        Rectangle {
            vertices: right_vertices,
            vao: right_vao,
            vbo: right_vbo,
        },
    )
}

pub fn generate_approximation_window(bounds: &(f32, f32), alpha: f32) -> Rectangle {
    let vertices = vec![
        Vertex {
            x: bounds.0,
            y: 1.0,
            color: renderer::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
        },
        Vertex {
            x: bounds.0,
            y: -1.0,
            color: renderer::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
        },
        Vertex {
            x: bounds.1,
            y: -1.0,
            color: renderer::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
        },
        Vertex {
            x: bounds.1,
            y: 1.0,
            color: renderer::Color {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: alpha,
            },
        },
    ];

    let (vao, vbo) = generate_buffers(&vertices);

    Rectangle { vertices, vao, vbo }
}

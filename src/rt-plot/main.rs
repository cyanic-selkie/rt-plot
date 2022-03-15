mod approximation;
mod config;
mod plot;
mod renderer;
mod text;

use config::{ColorScheme, DataConfig};
use glfw::{Action, Context, Key};
use nalgebra::{Matrix3, Vector2};
use ordered_float::OrderedFloat;
use rusttype::Font;
use std::cmp;
use std::collections::BTreeMap;
use std::io;
use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;

fn current_time() -> u64 {
    // in seconds
    let time = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap();
    // in miliseconds
    time.as_secs() * 1000 + time.subsec_nanos() as u64 / 1_000_000
}

fn read_data(
    data: Arc<Mutex<BTreeMap<OrderedFloat<f32>, Vec<f32>>>>,
    data_config: Arc<DataConfig>,
    stop_signal: Arc<AtomicBool>,
) {
    let stdin = io::stdin();

    for line in stdin.lock().lines() {
        if stop_signal.load(Ordering::SeqCst) {
            break;
        }

        let parts: Vec<u64> = line
            .unwrap()
            .trim_end()
            .split(' ')
            .map(|x| x.parse::<u64>().unwrap())
            .collect();

        assert_eq!(
            data_config.y.len(),
            parts.len() - 1,
            "Data configuration specifies {} data inputs, but got {}.",
            data_config.y.len(),
            parts.len() - 1
        );

        let time = parts[0] as f32;

        // Transform time to grid units.
        let time = time
            / data_config.grid.time.seconds_per_division
            / data_config.grid.time.raw_per_second;

        // Transform data to grid units.
        let y: Vec<f32> = parts[1..]
            .iter()
            .enumerate()
            .map(|(i, &y)| {
                (y as f32 - data_config.y[i].raw_offset) / data_config.y[i].raw_per_division
            })
            .collect();

        data.lock().unwrap().insert(OrderedFloat(time), y);
    }
}

fn main() {
    // Load settings and configuration files.
    let settings = config::parse_cli_options();

    let data_config: Arc<DataConfig> = Arc::new(config::read_data_config(&settings.data_config));
    let color_scheme: ColorScheme = config::read_color_scheme(&settings.color_scheme);

    // Create a thread-safe btree of data points that will be used to read and write incoming data.
    let data = Arc::new(Mutex::new(BTreeMap::new()));

    // Create a child thread that will read data from the data source.
    // data_write is a reference to the data that will be used by the input thread for writing the incoming data to
    let stop_signal = Arc::new(AtomicBool::new(false));
    let input_thread = thread::spawn({
        let data = data.clone();
        let data_config = data_config.clone();
        let stop_signal = stop_signal.clone();
        move || read_data(data, data_config, stop_signal)
    });

    // Graphics.
    let background_color = renderer::Color::from(&color_scheme.background);
    let grid_color = renderer::Color::from(&color_scheme.grid);
    let channel_colors: Vec<renderer::Color> = color_scheme
        .channel
        .iter()
        .map(|channel_color| renderer::Color::from(channel_color))
        .collect();

    // Initialize the window.
    let (mut window, mut glfw, events) =
        renderer::initialize_window(settings.width, settings.height, "rt-plot", background_color);

    // Load the font for rendering text.
    let font = include_bytes!("fonts/SourceSansPro-ExtraLight.ttf");
    let font = Font::try_from_bytes(font as &[u8]).unwrap();

    // Initalize the shaders.
    let shader_program = renderer::initialize_shaders();
    let text_shader_program = renderer::initialize_text_shaders();

    // Transformation matrix for transforming from grid coordinates to OpenGL coordinates.
    let coordinate_transform = renderer::transformation_matrix(
        [
            data_config.grid.time.divisions as f32 / 2.0,
            data_config.grid.data.zero_shift,
        ],
        [
            2.0 / data_config.grid.time.divisions as f32,
            2.0 / data_config.grid.data.divisions as f32,
        ],
    );

    let identity: Matrix3<f32> = Matrix3::identity();
    let zero_vector: Vector2<f32> = Vector2::zeros();

    let grid = plot::generate_grid(
        data_config.grid.time.divisions,
        data_config.grid.data.divisions,
        grid_color,
    );

    let (_, _, grid_width, grid_height) = plot::get_dimensions(
        settings.width,
        settings.height,
        settings.padding,
        data_config.grid.time.divisions,
        data_config.grid.data.divisions,
    );

    let grid_label = text::generate_text(
        0.0,
        1.0 - (settings.height - grid_height as u32) as f32 / settings.height as f32 / 2.0,
        &data_config.grid.label,
        settings.padding as f32 / 1.5,
        &font,
        settings.width,
        settings.height,
        renderer::Color::from(&color_scheme.labels),
        text::Orientation::Horizontal,
    );

    let grid_data_label = text::generate_text(
        -1.0 + (settings.width - grid_width as u32) as f32 / settings.width as f32 / 2.0,
        0.0,
        &data_config.grid.data.label,
        settings.padding as f32 / 1.5,
        &font,
        settings.width,
        settings.height,
        renderer::Color::from(&color_scheme.labels),
        text::Orientation::Vertical,
    );

    let grid_time_label = text::generate_text(
        0.0,
        -1.0 + (settings.height - grid_height as u32) as f32 / settings.height as f32 / 2.0,
        &data_config.grid.time.label,
        settings.padding as f32 / 1.5,
        &font,
        settings.width,
        settings.height,
        renderer::Color::from(&color_scheme.labels),
        text::Orientation::Horizontal,
    );

    let zero_label = text::generate_text(
        1.0 - (settings.width - grid_width as u32) as f32 / settings.width as f32 / 2.0,
        (data_config.grid.data.zero_shift / data_config.grid.data.divisions as f32
            * grid_height as f32)
            / settings.height as f32
            * 2.0,
        "0",
        settings.padding as f32 / 1.5,
        &font,
        settings.width,
        settings.height,
        renderer::Color::from(&color_scheme.labels),
        text::Orientation::Horizontal,
    );

    // Main loop.
    let mut time_started = None;
    let mut frozen_translation: Option<f64> = None;
    let mut focused_channel = None;
    let mut approximation_type = None;
    let mut approximation_range = None;
    let mut approximation_label = None;
    while !window.should_close() {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);
        }

        shader_program.set_used();

        // Make sure to remove all the vertex transformations set in the shader before drawing the grid.
        shader_program.set_uniform_matrix("coordinate_transform", &identity);
        shader_program.set_uniform_vector("translation", &zero_vector);

        plot::draw_grid(
            &grid,
            settings.width,
            settings.height,
            settings.padding,
            data_config.grid.time.divisions,
            data_config.grid.data.divisions,
        );

        // Lock the mutex so that we can safely access the data.
        let data = data.lock().unwrap();

        // If we have no points, there's nothing to draw.
        if data.len() == 0 {
            continue;
        }

        // Here is where we start the clock to ensure proper translation when drawing the graphs.
        if let None = time_started {
            let (min, _) = data.iter().next().unwrap();

            // Align the time so that the right side of the plot is the beginning. Use the minimum
            // value from the data that we are drawing to determine the shift.
            time_started = Some(
                current_time() as f64 / data_config.grid.time.seconds_per_division as f64 / 1000f64
                    - min.into_inner() as f64,
            );
        }

        // How much time had passed determines the translation when drawing the graphs.
        let time_passed = match frozen_translation {
            Some(time_passed) => time_passed,
            None => {
                current_time() as f64 / data_config.grid.time.seconds_per_division as f64 / 1000f64
                    - time_started.unwrap()
            }
        };

        // For optimizations purposes, make sure to draw only what is actually visible.
        // The code below specifies the range of points to draw based on how much time had passed.
        let range = cmp::max(
            OrderedFloat(time_passed as f32 - data_config.grid.time.divisions as f32),
            OrderedFloat(0.0),
        )..OrderedFloat(time_passed as f32);

        let graphs = plot::generate_graphs(
            &data,
            &range,
            data_config.y.len(),
            &channel_colors,
            &approximation_range,
            &focused_channel,
        );

        // Set the coordinate transformation matrix and the time translation vector for use in the
        // shader. This means that the GPU will handle all the transformations and therefore will be more
        // performant since it will be done in parallel.
        let polynomial_graph = if let (
            Some(approximation_type),
            Some(approximation_range),
            Some(i),
            true,
        ) = (
            &approximation_type,
            &approximation_range,
            &focused_channel,
            data.len() >= 3,
        ) {
            let (coefficients, errors) =
                approximation::fit(&data, &approximation_range, approximation_type, *i);

            let (transformed_coefficients, transformed_errors) =
                approximation::transform_coefficients(&coefficients, &errors, approximation_type);

            let measurement_strings: Vec<String> = transformed_coefficients
                .iter()
                .zip(&transformed_errors)
                .map(|(&coefficient, &error)| {
                    approximation::measurement_to_string(coefficient, error)
                })
                .collect();

            let approximation_label_string = match approximation_type {
                approximation::Type::Constant => {
                    format!("y = {}", measurement_strings[0])
                }
                approximation::Type::Linear => {
                    format!(
                        "k = {}   t₀ = {}",
                        measurement_strings[1], measurement_strings[0]
                    )
                }
                approximation::Type::Quadratic => {
                    format!(
                        "a = {}   t₀ = {}   y₀ = {}",
                        measurement_strings[2], measurement_strings[1], measurement_strings[0]
                    )
                }
            };

            approximation_label = Some(text::generate_text(
                0.0,
                1.0 - (settings.height - grid_height as u32) as f32 / settings.height as f32 / 2.0,
                &approximation_label_string,
                settings.padding as f32 / 1.5,
                &font,
                settings.width,
                settings.height,
                renderer::Color::from(&color_scheme.labels),
                text::Orientation::Horizontal,
            ));

            let graph = plot::generate_polynomial_graph(
                &coefficients,
                &range,
                renderer::Color::from(&color_scheme.fit),
            );

            Some(graph)
        } else {
            None
        };

        // Free the mutex as we no longer need the data after generating the graphs.
        drop(data);

        shader_program.set_uniform_matrix("coordinate_transform", &coordinate_transform);
        shader_program.set_uniform_vector("translation", &Vector2::new(-time_passed as f32, 0.0));
        graphs.iter().for_each(|graph| plot::draw_graph(graph));

        if let Some(graph) = polynomial_graph {
            plot::draw_graph(&graph);
        }

        text_shader_program.set_used();
        text_shader_program.set_uniform_texture("textTexture", gl::TEXTURE0 as i32);

        match &approximation_label {
            Some(label) => {
                text::draw_text(&label, settings.width, settings.height);
            }
            None => {
                text::draw_text(&grid_label, settings.width, settings.height);
            }
        };

        text::draw_text(&grid_time_label, settings.width, settings.height);
        text::draw_text(&grid_data_label, settings.width, settings.height);
        text::draw_text(&zero_label, settings.width, settings.height);

        // Display the image the GPU drew.
        window.swap_buffers();

        // This section is just for handling keyboard input.
        glfw.poll_events();

        // For approximation window selection.
        let resolution = 0.01;
        let step_multiplier = 4.0;

        for (_, event) in glfw::flush_messages(&events) {
            match event {
                // Stop the program.
                glfw::WindowEvent::Key(Key::Q, _, Action::Press, _) => {
                    window.set_should_close(true);
                    stop_signal.store(true, Ordering::SeqCst);
                }
                // Freeze the graph.
                glfw::WindowEvent::Key(Key::Space, _, Action::Press, _) => {
                    match frozen_translation {
                        Some(_) => {
                            frozen_translation = None;
                            approximation_type = None;
                            approximation_range = None;
                            approximation_label = None;
                        }
                        None => {
                            frozen_translation = Some(time_passed);
                            approximation_range = Some(range.clone());
                        }
                    };
                }
                // Cycle through approximations modes. Off, constant, linear, quadratic.
                glfw::WindowEvent::Key(Key::M, _, Action::Press, _) => {
                    if let Some(_) = frozen_translation {
                        match approximation_type {
                            None => {
                                approximation_type = Some(approximation::Type::Constant);
                            }
                            Some(approximation::Type::Constant) => {
                                approximation_type = Some(approximation::Type::Linear)
                            }
                            Some(approximation::Type::Linear) => {
                                approximation_type = Some(approximation::Type::Quadratic)
                            }
                            Some(approximation::Type::Quadratic) => {
                                approximation_type = None;
                                approximation_label = None;
                            }
                        }
                    }
                }
                // Keys for adjusting the approximation window bounds.
                // H and L move the window left and right.
                glfw::WindowEvent::Key(Key::H, _, mode, _) => {
                    let step = match mode {
                        Action::Press => resolution,
                        Action::Repeat => resolution * step_multiplier,
                        Action::Release => 0.0,
                    };

                    if let Some(approximation_range) = &mut approximation_range {
                        if approximation_range.start - step > range.start {
                            approximation_range.start -= step;
                            approximation_range.end -= step;
                        }
                    }
                }
                glfw::WindowEvent::Key(Key::L, _, mode, _) => {
                    let step = match mode {
                        Action::Press => resolution,
                        Action::Repeat => resolution * step_multiplier,
                        Action::Release => 0.0,
                    };

                    if let Some(approximation_range) = &mut approximation_range {
                        if approximation_range.end + step < range.end {
                            approximation_range.start += step;
                            approximation_range.end += step;
                        }
                    }
                }
                glfw::WindowEvent::Key(Key::J, _, mode, _) => {
                    let step = match mode {
                        Action::Press => resolution,
                        Action::Repeat => resolution * step_multiplier,
                        Action::Release => 0.0,
                    };

                    if let Some(approximation_range) = &mut approximation_range {
                        if (approximation_range.start + step - approximation_range.end - step).abs()
                            > resolution * step_multiplier * 5.0
                        {
                            approximation_range.start += step;
                            approximation_range.end -= step;
                        }
                    }
                }
                glfw::WindowEvent::Key(Key::K, _, mode, _) => {
                    let step = match mode {
                        Action::Press => resolution,
                        Action::Repeat => resolution * step_multiplier,
                        Action::Release => 0.0,
                    };

                    if let Some(approximation_range) = &mut approximation_range {
                        if approximation_range.end + step < range.end
                            && approximation_range.start - step > range.start
                        {
                            approximation_range.start -= step;
                            approximation_range.end += step;
                        }
                    }
                }
                // Use numbers 1..9 to focus channels. 0 to unfocus.
                glfw::WindowEvent::Key(Key::Num0, _, Action::Press, _) => {
                    focused_channel = None;
                }
                glfw::WindowEvent::Key(Key::Num1, _, Action::Press, _) => {
                    if data_config.y.len() >= 1 {
                        focused_channel = Some(0);
                    }
                }
                glfw::WindowEvent::Key(Key::Num2, _, Action::Press, _) => {
                    if data_config.y.len() >= 2 {
                        focused_channel = Some(1);
                    }
                }
                glfw::WindowEvent::Key(Key::Num3, _, Action::Press, _) => {
                    if data_config.y.len() >= 3 {
                        focused_channel = Some(2);
                    }
                }
                glfw::WindowEvent::Key(Key::Num4, _, Action::Press, _) => {
                    if data_config.y.len() >= 4 {
                        focused_channel = Some(3);
                    }
                }
                glfw::WindowEvent::Key(Key::Num5, _, Action::Press, _) => {
                    if data_config.y.len() >= 5 {
                        focused_channel = Some(4);
                    }
                }
                glfw::WindowEvent::Key(Key::Num6, _, Action::Press, _) => {
                    if data_config.y.len() >= 6 {
                        focused_channel = Some(5);
                    }
                }
                glfw::WindowEvent::Key(Key::Num7, _, Action::Press, _) => {
                    if data_config.y.len() >= 7 {
                        focused_channel = Some(6);
                    }
                }
                glfw::WindowEvent::Key(Key::Num8, _, Action::Press, _) => {
                    if data_config.y.len() >= 8 {
                        focused_channel = Some(7);
                    }
                }
                glfw::WindowEvent::Key(Key::Num9, _, Action::Press, _) => {
                    if data_config.y.len() >= 9 {
                        focused_channel = Some(8);
                    }
                }
                _ => {}
            }
        }
    }

    window.close();

    input_thread.join().unwrap();
}

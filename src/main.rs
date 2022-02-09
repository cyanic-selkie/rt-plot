mod config;
mod plot;
mod renderer;

use glfw::{Action, Context, Key};
use rand::Rng;
use std::f64::consts::PI;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time;
use std::vec;

fn current_time() -> u64 {
    // in seconds
    let time = time::SystemTime::now()
        .duration_since(time::UNIX_EPOCH)
        .unwrap();
    // in miliseconds
    time.as_secs() * 1000 + time.subsec_nanos() as u64 / 1_000_000
}

fn read_input(
    _serial_port: String,
    data: Arc<Mutex<vec::Vec<plot::DataPoint>>>,
    shutdown_signal: Arc<Mutex<bool>>,
    time_started: u64,
) {
    let mut rng = rand::thread_rng();

    while !(*shutdown_signal.lock().unwrap()) {
        let _y: f64 = rng.gen_range(0.0f64..1.0f64);

        let time = current_time();

        let mut data = data.lock().unwrap();

        // Fake input for testing.
        // === === ===
        let data_point = plot::DataPoint {
            y: vec![
                ((time - time_started) as f64 * 2.0 * PI / 2000.0).sin() * 10.0,
                ((time - time_started) as f64 * 2.0 * PI / 10000.0).cosh() * 0.5,
                ((time - time_started) as f64 * 2.0 * PI / 10000.0).tan() * 0.5,
                //((time - time_started) as f64 * 2.0 * PI / 3000.0).cos() * 3.0,
                //((time - time_started) as f64 * 2.0 * PI / 10000.0).tanh() * 5.0,
                // TODO add a check somewhere to make sure the number of channels is correct
            ],
            x: time - time_started,
        };
        // === === ===

        if data.len() == 0 || data_point.x > data.last().unwrap().x {
            data.push(data_point.clone());

            drop(data);

            let mut out = format!("{}\t", data_point.x);
            data_point
                .y
                .iter()
                .for_each(|channel| out.push_str(&format!("{}\t", channel)));

            println!("{}", &out[..out.len() - 1]);
        } else {
            drop(data);
        }
    }
}

enum DisplayState {
    Frozen,
    Running,
    TransitionToFrozen,
}

enum ApproximationMode {
    Constant,
    Linear,
    Quadratic,
    Off,
}

pub fn polyfit<T: nalgebra::RealField>(
    x_values: &[T],
    y_values: &[T],
    polynomial_degree: usize,
) -> Result<Vec<T>, &'static str> {
    let number_of_columns = polynomial_degree + 1;
    let number_of_rows = x_values.len();
    let mut a = nalgebra::DMatrix::zeros(number_of_rows, number_of_columns);

    for (row, &x) in x_values.iter().enumerate() {
        a[(row, 0)] = T::one();

        for col in 1..number_of_columns {
            a[(row, col)] = x.powf(nalgebra::convert(col as f64));
        }
    }

    let b = nalgebra::DVector::from_row_slice(y_values);

    let decomp = nalgebra::SVD::new(a, true, true);

    match decomp.solve(&b, nalgebra::convert(1e-18f64)) {
        Ok(mat) => Ok(mat.data.into()),
        Err(error) => Err(error),
    }
}

fn approximate(
    data: &vec::Vec<plot::DataPoint>,
    approximation_bounds: (f32, f32),
    approximation_mode: &ApproximationMode,
    grid: &plot::Grid,
    delta: u64,
    focused_channel: i8,
) -> Result<vec::Vec<f32>, &'static str> {
    let mut a = vec![];
    let mut b = vec![];

    let mut data_points: usize = 0;

    for data_point in data.iter().rev() {
        let (x, y) = plot::normalize_datapoint(&data_point, &grid, delta);

        if x < approximation_bounds.0 {
            break;
        } else if x > approximation_bounds.1 {
            continue;
        }

        let y = y[focused_channel as usize];
        a.push(x);
        b.push(y);

        data_points += 1;
    }

    if data_points < 3 {
        return Err("Approximation needs at least 3 data points.");
    }

    polyfit(
        &a,
        &b,
        match approximation_mode {
            ApproximationMode::Linear => 1,
            ApproximationMode::Quadratic => 2,
            _ => 0,
        },
    )
}

fn main() {
    // Get options from the command line interface.
    let options = config::parse_cli();

    // Read data config and color scheme from files.
    let data_config = config::read_data_config(&options.data_config);
    let color_scheme = config::read_color_scheme(&options.color_scheme);

    assert!(
        color_scheme.channels.len() >= data_config.channels,
        "There are more channels in the data config than there are in the color scheme."
    );

    // Create a thread-safe vector of data points that will be used to read and write incoming data.
    // data reference is used for writing in the child thread
    // data_read reference is used for reading in the main thread
    let data = Arc::new(Mutex::new(Vec::new()));
    let data_read = data.clone();

    // Signal to be used when the thread reading from serial port should stop.
    // shutdown_signal reference is used for setting the signal in the main thread
    // shutdown_signal_read reference is used for listening for the signal in the child thread
    let shutdown_signal = Arc::new(Mutex::new(false));
    let shutdown_signal_read = shutdown_signal.clone();

    // Start the clock. This marks 0 on the data points.
    let time_started = current_time();

    // Create a child thread that will read data from the serial port.
    let serial_port = options.serial_port;
    let input_thread =
        thread::spawn(move || read_input(serial_port, data, shutdown_signal_read, time_started));

    // Graphics.
    let width: u32 = options.width;
    let height: u32 = options.height;

    let background_color = renderer::Color::from(&color_scheme.background);
    let grid_color = renderer::Color::from(&color_scheme.grid);

    let channel_colors: vec::Vec<renderer::Color> = color_scheme
        .channels
        .iter()
        .map(|channel_color| renderer::Color::from(channel_color))
        .collect();

    // Initialize the window.
    let (mut window, mut glfw, events) =
        renderer::initialize_window(width, height, "rt-plot", background_color);
    let shader_program = renderer::initialize_shaders();
    shader_program.set_used();

    // Generate the grid.
    let grid = plot::generate_grid(
        data_config.x.divisions,
        data_config.y.divisions,
        width,
        height,
        data_config.x.milliseconds_per_division,
        data_config.y.units_per_division,
        grid_color,
    );

    // Application state variables.
    let mut display_state = DisplayState::Running;

    let mut focused_channel: i8 = -1;

    let mut frozen_data = vec![];

    let mut approximation_mode = ApproximationMode::Off;
    let mut approximation_bounds = (-0.5, 0.5);

    let mut delta = 0;
    // Main loop
    while !window.should_close() {
        unsafe {
            gl::Clear(gl::COLOR_BUFFER_BIT);
        }

        let data = data_read.lock().unwrap();

        // Generate graphs from data depending on whether it's frozen or not.
        let graphs;
        match display_state {
            DisplayState::TransitionToFrozen => {
                frozen_data = data.clone();
                graphs = plot::generate_graphs(&frozen_data, &grid, delta, &channel_colors);
                display_state = DisplayState::Frozen;
            }
            DisplayState::Running => {
                delta = current_time() - time_started;
                graphs = plot::generate_graphs(&data, &grid, delta, &channel_colors);
            }
            DisplayState::Frozen => {
                graphs = plot::generate_graphs(&frozen_data, &grid, delta, &channel_colors);
            }
        }

        // Generate approximation polynomial graph if necessary.
        let mut approximation_graph = None;
        match approximation_mode {
            ApproximationMode::Off => drop(data),
            _ => {
                if focused_channel > -1 {
                    let coefficients = approximate(
                        if let DisplayState::Running = display_state {
                            &data
                        } else {
                            &frozen_data
                        },
                        approximation_bounds,
                        &approximation_mode,
                        &grid,
                        delta,
                        focused_channel,
                    );

                    drop(data);

                    match coefficients {
                        Ok(coefficients) => {
                            eprintln!("{:?}", coefficients);
                            approximation_graph = Some(plot::generate_polynomial_graph(
                                &coefficients,
                                renderer::Color::from(&color_scheme.approximation),
                            ));
                        }
                        Err(_err) => {}
                    };
                }

                // Draw rectangles left and right creating a focus window for approximation bounds.
                let (left, right) = plot::generate_bound_rectangles(
                    &approximation_bounds,
                    color_scheme.approximation_opacity,
                );
                plot::draw_rectangle(&left);
                plot::draw_rectangle(&right);
            }
        }

        // Draw the graphs on the screen. Make the focused graph thicker.
        plot::draw_grid(&grid);
        graphs.iter().enumerate().for_each(|(i, graph)| {
            plot::draw_graph(&graph, if i as i8 == focused_channel { 3.0 } else { 1.0 })
        });
        if let Some(graph) = approximation_graph {
            plot::draw_graph(&graph, 3.0);
        }

        window.swap_buffers();

        // Input handling
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            match event {
                // Stop the program.
                glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                    window.set_should_close(true);
                }
                // Freeze the display.
                glfw::WindowEvent::Key(Key::Space, _, Action::Press, _) => match display_state {
                    DisplayState::Frozen => display_state = DisplayState::Running,
                    DisplayState::TransitionToFrozen => display_state = DisplayState::Running,
                    DisplayState::Running => {
                        frozen_data.clear();
                        display_state = DisplayState::TransitionToFrozen;
                    }
                },
                // Cycle through approximations states. Off, constant, linear, quadratic.
                glfw::WindowEvent::Key(Key::M, _, Action::Press, _) => match approximation_mode {
                    ApproximationMode::Off => approximation_mode = ApproximationMode::Constant,
                    ApproximationMode::Constant => approximation_mode = ApproximationMode::Linear,
                    ApproximationMode::Linear => approximation_mode = ApproximationMode::Quadratic,
                    ApproximationMode::Quadratic => approximation_mode = ApproximationMode::Off,
                },
                // Keys for adjusting the approximation window bounds.
                // H and L move the window left and right.
                // J and K make the window smaller and larger.
                glfw::WindowEvent::Key(Key::H, _, Action::Repeat, _)
                | glfw::WindowEvent::Key(Key::H, _, Action::Press, _) => {
                    if approximation_bounds.0 - 0.01 > -1.0 {
                        approximation_bounds.0 -= 0.01;
                        approximation_bounds.1 -= 0.01;
                    }
                }
                glfw::WindowEvent::Key(Key::L, _, Action::Repeat, _)
                | glfw::WindowEvent::Key(Key::L, _, Action::Press, _) => {
                    if approximation_bounds.1 + 0.01 < 1.0 {
                        approximation_bounds.0 += 0.01;
                        approximation_bounds.1 += 0.01;
                    }
                }
                glfw::WindowEvent::Key(Key::J, _, Action::Repeat, _)
                | glfw::WindowEvent::Key(Key::J, _, Action::Press, _) => {
                    if (approximation_bounds.0 + 0.01 - approximation_bounds.1 - 0.01).abs() > 0.05
                    {
                        approximation_bounds.0 += 0.01;
                        approximation_bounds.1 -= 0.01;
                    }
                }
                glfw::WindowEvent::Key(Key::K, _, Action::Repeat, _)
                | glfw::WindowEvent::Key(Key::K, _, Action::Press, _) => {
                    if approximation_bounds.1 + 0.01 < 1.0 && approximation_bounds.0 - 0.01 > -1.0 {
                        approximation_bounds.0 -= 0.01;
                        approximation_bounds.1 += 0.01;
                    }
                }
                // Use numbers 1..9 to focus channels. 0 to unfocus.
                glfw::WindowEvent::Key(Key::Num0, _, Action::Press, _) => {
                    focused_channel = -1;
                }
                glfw::WindowEvent::Key(Key::Num1, _, Action::Press, _) => {
                    focused_channel = 0;
                }
                glfw::WindowEvent::Key(Key::Num2, _, Action::Press, _) => {
                    focused_channel = 1;
                }
                glfw::WindowEvent::Key(Key::Num3, _, Action::Press, _) => {
                    focused_channel = 2;
                }
                glfw::WindowEvent::Key(Key::Num4, _, Action::Press, _) => {
                    focused_channel = 3;
                }
                glfw::WindowEvent::Key(Key::Num5, _, Action::Press, _) => {
                    focused_channel = 4;
                }
                glfw::WindowEvent::Key(Key::Num6, _, Action::Press, _) => {
                    focused_channel = 5;
                }
                glfw::WindowEvent::Key(Key::Num7, _, Action::Press, _) => {
                    focused_channel = 6;
                }
                glfw::WindowEvent::Key(Key::Num8, _, Action::Press, _) => {
                    focused_channel = 7;
                }
                glfw::WindowEvent::Key(Key::Num9, _, Action::Press, _) => {
                    focused_channel = 8;
                }
                _ => {}
            }
        }
    }

    // Signal the child thread shutdown and wait for it to finish.
    *shutdown_signal.lock().unwrap() = true;
    input_thread.join().unwrap();
}

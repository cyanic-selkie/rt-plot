use clap::{App, Arg};
use serde_derive::Deserialize;
use std::fs;
use std::vec;

#[derive(Deserialize)]
pub struct ColorScheme {
    pub background: [u8; 3],
    pub grid: [u8; 3],
    pub approximation: [u8; 3],
    pub approximation_opacity: f32,
    pub channels: vec::Vec<[u8; 3]>,
}

#[derive(Deserialize)]
pub struct X {
    pub divisions: u32,
    pub milliseconds_per_division: u32,
}

#[derive(Deserialize)]
pub struct Y {
    pub divisions: u32,
    pub units_per_division: u32,
}

#[derive(Deserialize)]
pub struct DataConfig {
    pub channels: usize,
    pub x: X,
    pub y: Y,
}

pub struct Options {
    pub data_config: String,
    pub color_scheme: String,
    pub serial_port: String,
    pub width: u32,
    pub height: u32,
}

pub fn parse_cli() -> Options {
    let matches = App::new("rt-plot")
        .version("1.0")
        .about("Receives data from a serial port and plots it in real-time using GPU acceleration.")
        .arg(
            Arg::with_name("data-config")
                .short("d")
                .long("data-config")
                .value_name("FILE")
                .help("Sets the data config file.")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("color-scheme")
                .short("s")
                .long("color-scheme")
                .value_name("FILE")
                .help("Sets the color scheme file.")
                .required(true)
                .takes_value(true)
                .default_value("resources/default/color_scheme.toml"),
        )
        .arg(
            Arg::with_name("serial-port")
                .short("p")
                .long("serial-port")
                .value_name("SERIAL-PORT")
                .help("Sets the serial port to read data from.")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("width")
                .short("w")
                .long("width")
                .value_name("WIDTH")
                .help("Sets the width of the screen.")
                .required(false)
                .takes_value(true)
                .default_value("1280"),
        )
        .arg(
            Arg::with_name("height")
                .short("h")
                .long("height")
                .value_name("HEIGHT")
                .help("Sets the height of the screen.")
                .required(false)
                .takes_value(true)
                .default_value("720"),
        )
        .get_matches();

    Options {
        data_config: String::from(matches.value_of("data-config").unwrap()),
        color_scheme: String::from(matches.value_of("color-scheme").unwrap()),
        serial_port: String::from(matches.value_of("serial-port").unwrap()),
        width: matches.value_of("width").unwrap().parse::<u32>().unwrap(),
        height: matches.value_of("height").unwrap().parse::<u32>().unwrap(),
    }
}

pub fn read_data_config(data_config_filename: &str) -> DataConfig {
    let data_config = fs::read_to_string(data_config_filename).unwrap();
    toml::from_str(&data_config).unwrap()
}

pub fn read_color_scheme(data_config_filename: &str) -> ColorScheme {
    let color_scheme = fs::read_to_string(data_config_filename).unwrap();
    toml::from_str(&color_scheme).unwrap()
}

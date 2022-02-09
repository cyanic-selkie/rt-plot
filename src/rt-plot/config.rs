use clap::{App, Arg};
use serde_derive::Deserialize;
use std::fs;

#[derive(Deserialize, Debug)]
pub struct Color {
    pub rgb: [u8; 3],
    pub opacity: Option<f32>,
}

#[derive(Deserialize, Debug)]
pub struct ColorScheme {
    pub background: Color,
    pub labels: Color,
    pub grid: Color,
    pub fit: Color,
    pub channel: Vec<Color>,
}

#[derive(Deserialize, Debug)]
pub struct Time {
    pub divisions: u32,
    pub seconds_per_division: f32,
    pub raw_per_second: f32,
    pub label: String,
}

#[derive(Deserialize, Debug)]
pub struct Data {
    pub divisions: u32,
    pub zero_shift: f32,

    pub label: String,
}

#[derive(Deserialize, Debug)]
pub struct Grid {
    pub label: String,
    pub time: Time,
    pub data: Data,
}

#[derive(Deserialize, Debug)]
pub struct Y {
    pub raw_offset: f32,
    pub raw_per_division: f32,
}

#[derive(Deserialize, Debug)]
pub struct DataConfig {
    pub grid: Grid,
    pub y: Vec<Y>,
}

pub struct Settings {
    pub data_config: String,
    pub color_scheme: String,
    pub width: u32,
    pub height: u32,
    pub padding: u32,
}

pub fn parse_cli_options() -> Settings {
    let matches = App::new("rt-plot")
        .version("0.1.0")
        .about("Receives data from stdin and plots it in real-time using GPU acceleration.")
        .arg(
            Arg::with_name("data-config")
                .long("data-config")
                .value_name("FILE")
                .help("Sets the data config file.")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("color-scheme")
                .long("color-scheme")
                .value_name("FILE")
                .help("Sets the color scheme file.")
                .required(true)
                .takes_value(true)
                .default_value("resources/default/color_scheme.toml"),
        )
        .arg(
            Arg::with_name("width")
                .long("width")
                .value_name("WIDTH")
                .help("Sets the width of the screen.")
                .required(false)
                .takes_value(true)
                .default_value("1280"),
        )
        .arg(
            Arg::with_name("height")
                .long("height")
                .value_name("HEIGHT")
                .help("Sets the height of the screen.")
                .required(false)
                .takes_value(true)
                .default_value("720"),
        )
        .arg(
            Arg::with_name("padding")
                .long("padding")
                .value_name("PADDING")
                .help("Sets the padding around the plot.")
                .required(false)
                .takes_value(true)
                .default_value("100"),
        )
        .get_matches();

    Settings {
        data_config: String::from(matches.value_of("data-config").unwrap()),
        color_scheme: String::from(matches.value_of("color-scheme").unwrap()),
        width: matches.value_of("width").unwrap().parse::<u32>().unwrap(),
        height: matches.value_of("height").unwrap().parse::<u32>().unwrap(),
        padding: matches.value_of("padding").unwrap().parse::<u32>().unwrap(),
    }
}

pub fn read_data_config(data_config_filename: &str) -> DataConfig {
    let data_config = fs::read_to_string(data_config_filename).unwrap();
    toml::from_str(&data_config).unwrap()
}

pub fn read_color_scheme(color_scheme_filename: &str) -> ColorScheme {
    let color_scheme = fs::read_to_string(color_scheme_filename).unwrap();
    toml::from_str(&color_scheme).unwrap()
}

use clap::{App, Arg, SubCommand};
use serialport;
use std::io::{BufRead, BufReader};
use std::time::Duration;

fn print_available_ports() {
    let ports = serialport::available_ports().expect("No ports found!");
    for port in ports {
        println!("{}", port.port_name);
    }
}

fn read_from_serial(serial_port: &str, baud_rate: u32) -> Result<(), Box<dyn std::error::Error>> {
    let serial_port = serialport::new(serial_port, baud_rate)
        .timeout(Duration::from_millis(10000))
        .open()?;

    let mut reader = BufReader::new(serial_port);

    let mut i = 0;
    loop {
        let mut input_string = String::new();
        match reader.read_line(&mut input_string) {
            Ok(_) => {}
            Err(_) => continue,
        };

        // Ignore the first 10 entries for the data to stabilize.
        if i < 10 {
            i += 1;
            continue;
        }

        let parts = input_string
            .trim_end()
            .trim_end_matches(",")
            .split(',')
            .map(|x| x.parse::<u64>())
            .collect::<Result<Vec<u64>, core::num::ParseIntError>>()?;

        for part in parts {
            print!("{} ", part);
        }

        println!();
    }
}

fn main() {
    let matches = App::new("rt-relay")
        .version("0.1.0")
        .about("Receives data from a serial port or a UDP connection and outputs it to stdout.")
        .subcommand(SubCommand::with_name("list-serial").about("Show all available serial ports."))
        .subcommand(
            SubCommand::with_name("read")
                .about("Read data from a serial port.")
                .arg(
                    Arg::with_name("serial-port")
                        .long("serial-port")
                        .value_name("SERIAL-PORT")
                        .help("Sets the serial port to read data from.")
                        .takes_value(true)
                        .required(true),
                )
                .arg(
                    Arg::with_name("baud-rate")
                        .long("baud-rate")
                        .value_name("BAUD-RATE")
                        .help("Sets the baud rate of the serial port used to read data from.")
                        .takes_value(true)
                        .required(true),
                ),
        )
        .get_matches();

    if let Some(ref _matches) = matches.subcommand_matches("list-serial") {
        // Handles printing the ports
        print_available_ports()
    } else if let Some(ref matches) = matches.subcommand_matches("read") {
        let baud_rate = matches
            .value_of("baud-rate")
            .unwrap()
            .parse::<u32>()
            .unwrap();
        let serial_port = matches.value_of("serial-port").unwrap();
        if let Err(e) = read_from_serial(serial_port, baud_rate) {
            if let Some(e) = e.downcast_ref::<serialport::Error>() {
                eprintln!("Error connecting to the serial port: {}", e);
            } else if let Some(e) = e.downcast_ref::<core::num::ParseIntError>() {
                eprintln!("Error parsing data from the serial port: {}", e);
            }
        }
    }
}

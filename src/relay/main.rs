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

fn read_from_serial(serial_port: &str) -> Result<(), Box<dyn std::error::Error>> {
    let serial_port = serialport::new(serial_port, 115_200)
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
    let matches = App::new("relay")
        .version("0.1.0")
        .about("Receives data from a serial port or a UDP connection and outputs it to stdout.")
        .subcommand(SubCommand::with_name("list-serial").about("Show all available serial ports."))
        .subcommand(
            SubCommand::with_name("read")
                .about("Read data from a serial port or a UDP connection.")
                .arg(
                    Arg::with_name("serial-port")
                        .long("serial-port")
                        .value_name("SERIAL-PORT")
                        .help("Sets the serial port to read data from.")
                        .conflicts_with_all(&["ip", "udp_port"])
                        .required_unless_all(&["ip", "udp_port"])
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("ip")
                        .long("ip")
                        .value_name("IP")
                        .help("Sets the IP to read data from. Can only be used in tandem with udp-port.")
                        .conflicts_with("serial-port")
                        .required_unless("serial-port")
                        .takes_value(true),
                )
                .arg(
                    Arg::with_name("udp-port")
                        .long("udp-port")
                        .value_name("UDP-PORT")
                        .help("Sets the UDP port to read data from. Can only be used in tandem with ip.")
                        .conflicts_with("serial-port")
                        .required_unless("serial-port")
                        .takes_value(true),
                ),
        )
        .get_matches();

    if let Some(ref _matches) = matches.subcommand_matches("list-serial") {
        // Handles printing the ports
        print_available_ports()
    } else if let Some(ref matches) = matches.subcommand_matches("read") {
        // Handles reading the data from the serial port
        if let Some(serial_port) = matches.value_of("serial-port") {
            if let Err(e) = read_from_serial(serial_port) {
                if let Some(e) = e.downcast_ref::<serialport::Error>() {
                    eprintln!("Error connecting to the serial port: {}", e);
                } else if let Some(e) = e.downcast_ref::<core::num::ParseIntError>() {
                    eprintln!("Error parsing data from the serial port: {}", e);
                }
            }
        // Handles reading the data from the UDP connection
        } else {
            let _ip = matches.value_of("ip");
            let _udp_port = matches.value_of("udp-port");
        }
    }
}

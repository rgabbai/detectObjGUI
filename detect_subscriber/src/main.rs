use std::os::unix::net::{UnixListener, UnixStream};
use std::io::prelude::*;
use std::thread;
use serde::{Serialize, Deserialize};
use serde_json;
use std::time::{Duration, Instant};
use rand::Rng; // Import the Rng trait

const SOCKET_PATH: &str = "/tmp/robot/detect-socket";

#[derive(Serialize, Deserialize, Debug)]
struct BoxCor(f32, f32, f32, f32);

#[derive(Serialize, Deserialize, Debug)]
struct DetObj {
    box_location: BoxCor,
    otype: String,
}


fn handle_client(mut stream: UnixStream) {
    println!("> Client request detected: ");

    // Loop for sending data periodically
    loop {
        // Randomly generate the first value of BoxCor
        let mut rng = rand::thread_rng();
        let random_x: f32 = rng.gen_range(100.0..=700.0);
        let random_y: f32 = rng.gen_range(100.0..=500.0);

        // Create a vector of DetObj
        let detected_objects = vec![
            DetObj {
                box_location: BoxCor(470.0, 220.0, 50.0, 180.0),
                otype: "person1".to_string(),
            },
            DetObj {
                box_location: BoxCor(random_x, random_y, 80.0, 100.0),
                otype: "person2".to_string(),
            },
            // ... Add more objects as needed
        ];

        // Serialize the data
        match serde_json::to_string(&detected_objects) {
            Ok(serialized) => {
                // Attempt to write serialized data to the stream
                if let Err(e) = stream.write_all(serialized.as_bytes()) {
                    eprintln!("Failed to write to stream: {}", e);
                    break; // Exit the loop if writing to the stream fails
                }
                println!("> Sent client detected data: {:?}", serialized);
            },
            Err(e) => {
                eprintln!("Failed to serialize data: {}", e);
                break; // Exit the loop if serialization fails
            }
        }

        // Sleep for one second before sending the next update
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn main() -> std::io::Result<()> {

    println!("************ Detect subscriber node: ********** ");

    std::fs::remove_file(SOCKET_PATH).ok(); // Remove the socket file if it exists
    let listener = UnixListener::bind(SOCKET_PATH)?;

    // accept connections and process them serially
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                /* Spawn a new thread for each connection */
                thread::spawn(|| handle_client(stream));
            }
            Err(err) => {
                eprintln!("Error: {}", err);
                break;
            }
        }
    }
    Ok(())
}

use std::os::unix::net::{UnixListener, UnixStream};
use std::io::prelude::*;
use std::thread;
use serde::{Serialize, Deserialize};
use serde_json;

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

   // Creating a vector of DetObj
   let detected_objects = vec![
    DetObj {
        box_location: BoxCor(470.0, 220.0, 50.0, 180.0),
        otype: "person1".to_string(),
    },
    DetObj {
        box_location: BoxCor(300.0, 260.0, 80.0, 100.0),
        otype: "person2".to_string(),
    },
    // Add more objects as needed
];

    let serialized = serde_json::to_string(&detected_objects).unwrap();
    stream.write_all(serialized.as_bytes()).unwrap();
    println!("> Send client detected data: {:?}",serialized);

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

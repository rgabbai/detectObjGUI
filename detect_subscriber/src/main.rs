use std::os::unix::net::{UnixListener, UnixStream};
use std::io::prelude::*;
use std::thread;
use serde::{Serialize, Deserialize};
use serde_json;
use std::time::{Duration, Instant};
use rand::Rng; // Import the Rng trait

// ROS Client Lib 
use std::sync::Arc;

use rclrust::{qos::QoSProfile, rclrust_info};
use rclrust_msg::std_msgs::msg::String as String_;

use std::sync::Mutex; // to pass shared data between threads


//Const sections
const SOCKET_PATH: &str = "/tmp/robot/detect-socket";
const TOPIC_NAME: &str = "detect";


#[derive(Serialize, Deserialize, Clone, Debug)]
struct BoxCor(f32, f32, f32, f32);

#[derive(Serialize, Deserialize, Clone, Debug)]
struct DetObj {
    box_location: BoxCor,
    otype: String,
    prob: f32,
}


fn handle_client(mut stream: UnixStream, shared_detected_objects: Arc<Mutex<Vec<DetObj>>>) {
    println!("> Client request detected: ");

    // Loop for sending data periodically
    loop {

        let detected_objects = shared_detected_objects.lock().unwrap().clone();

        // Randomly generate the first value of BoxCor
        let mut rng = rand::thread_rng();
        let random_x: f32 = rng.gen_range(100.0..=700.0);
        let random_y: f32 = rng.gen_range(100.0..=500.0);

        // Create a vector of DetObj
        //let detected_objects = vec![
        //    DetObj {
        //        box_location: BoxCor(470.0, 220.0, 50.0, 180.0),
        //        otype: "person1".to_string(),
        //        prob: 0.9,            
        //    },
        //    DetObj {
        //        box_location: BoxCor(random_x, random_y, 80.0, 100.0),
        //        otype: "person2".to_string(),
        //        prob: 0.9,
        //    },
        //    // ... Add more objects as needed
        //];

        

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {

    println!("************ Detect subscriber node: ********** ");

    let shared_detected_objects = Arc::new(Mutex::new(Vec::new()));



    std::fs::remove_file(SOCKET_PATH).ok(); // Remove the socket file if it exists
    let listener = UnixListener::bind(SOCKET_PATH)?;

    let shared_detected_objects_clone_for_unix_listener = shared_detected_objects.clone();

    // Spawn a new thread for the Unix listener
    thread::spawn(move || {
        // accept connections and process them serially
        for stream in listener.incoming() {
            let shared_detected_objects_clone_for_thread = shared_detected_objects_clone_for_unix_listener.clone();
            match stream {
                Ok(stream) => {
                    /* Spawn a new thread for each connection */
                    thread::spawn(move || handle_client(stream,shared_detected_objects_clone_for_thread.clone()));
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                    break;
                }
            }
        }
    });
    // continue with ROS node as part of the main 

    println!("> ROS subscriber node activation ");
    let shared_detected_objects_clone = Arc::clone(&shared_detected_objects);


    let ctx = rclrust::init()?;
    let mut node = ctx.create_node("detect_subscriber")?;
    let logger = node.logger();

    let _subscription = node.create_subscription(
        TOPIC_NAME,
        move |msg: Arc<String_>| {
            //rclrust_info!(logger, "I heard: '{}'", msg.data);

            // Deserialize msg.data into Vec<DetObj>
            match serde_json::from_str::<Vec<DetObj>>(&msg.data) {
                Ok(detected_objects) => {
                //send it to Unix stream 
                    println!(">>> Recived in DetObj format: {:?}",detected_objects);
                    let mut shared_data = shared_detected_objects_clone.lock().unwrap();
                    *shared_data = detected_objects;
                },
                Err(e) => {
                    eprintln!("Failed to deserialize msg.data: {}", e);
                }
        }

        },
        &QoSProfile::default(),
    )?;

    node.wait();
    



    Ok(())
}

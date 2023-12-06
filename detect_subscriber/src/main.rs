use std::os::unix::net::{UnixListener, UnixStream};
use std::io::prelude::*;
use std::thread;
use serde::{Serialize, Deserialize};
use serde_json;
use std::time::{Duration,SystemTime};
//use rand::Rng; // Import the Rng trait

// ROS Client Lib 
use std::sync::Arc;

use rclrust::{qos::QoSProfile, rclrust_info};
use rclrust_msg::std_msgs::msg::String as String_;
use rclrust_msg::sensor_msgs::msg::CompressedImage;
//use chrono::Utc;
use clap::{App, Arg}; // handle arguments 

use std::sync::Mutex; // to pass shared data between threads
use std::path::Path;
use std::fs;

//Const sections
const SOCKET_DIR: &str = "/tmp/robot";
const SOCKET_PATH: &str = "/tmp/robot/detect-socket";
const TOPIC_NAME: &str = "detect";
const IMAGE_TOPIC_NAME: &str = "Compressed_camera_image";


#[derive(Serialize, Deserialize, Clone, Debug)]
struct BoxCor(f32, f32, f32, f32);

#[derive(Serialize, Deserialize, Clone, Debug)]
struct DetObj {
    box_location: BoxCor,
    otype: String,
    prob: f32,
    dist: f64,
}


fn handle_client(mut stream: UnixStream, shared_detected_objects: Arc<Mutex<Vec<DetObj>>>) {
    println!("> Client request detected: ");

    // Loop for sending data periodically
    loop {

        let detected_objects = shared_detected_objects.lock().unwrap().clone();

        // Serialize the data
        match serde_json::to_string(&detected_objects) {
            Ok(serialized) => {
                // Attempt to write serialized data to the stream
                if let Err(e) = stream.write_all(serialized.as_bytes()) {
                    eprintln!("Failed to write to stream: {}", e);
                    break; // Exit the loop if writing to the stream fails
                }
                //println!("> Sent client detected data: {:?}", serialized);
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
    let matches = App::new("ROS Node Tracker")
    .version("1.0")
    .author("Rony Gabbai")
    .about("Detect subscriber node")
    .arg(Arg::new("capture")
         .short('c')
         .long("capture")
         .value_name("CAPTURE")
         .help("save recived images")
         .takes_value(false)
         .required(false))
    .get_matches();

    let capture = matches.is_present("capture");
    if capture {
        println!("Recived images will be saved - remember to clear later ...")
    }

    let shared_detected_objects = Arc::new(Mutex::new(Vec::new()));

    // create unix pipe tmp dir of not exist
    let b: bool = Path::new(SOCKET_DIR).is_dir();
    if !b {
        println!("Creating temp socket dir:{:?}",SOCKET_DIR);
        fs::create_dir(SOCKET_DIR)?;
    }


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
                    //println!(">>> Recived in DetObj format: {:?}",detected_objects);
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

    // Subscription for CompressedImage data
    let _image_subscription = node.create_subscription(
        IMAGE_TOPIC_NAME, // Change to your actual image topic name
        move |msg: Arc<CompressedImage>| {
            //keep image for GUI voew
            let filename = format!("received_image.jpg");
            std::fs::write(&filename, &msg.data).expect("Failed to write image file");
            // keep images for AI training  
            if capture {
                //let filename = format!("received_image_{}.jpg", Utc::now().timestamp_millis());
                //let t = ts!(1335020400);
                let filename = format!("received_image_{:?}.jpg",SystemTime::now());
                std::fs::write(&filename, &msg.data).expect("Failed to write image file");
            }

            //println!("Received and saved an image as {}", filename);
        },
        &QoSProfile::default(),
    )?;


    node.wait();
    



    Ok(())
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::Rect;  // Import Rect from the egui module
use egui::{Color32, vec2, pos2,Stroke};  
//Unix sockets
use std::os::unix::net::UnixStream;
use std::io::{Read, Write};
use std::sync::mpsc; // channel between the threads 
use std::thread;
use std::thread::JoinHandle;
use std::sync::mpsc::Receiver;
use std::sync::{Arc, Mutex};
use serde_json::Error;
use serde::{Serialize, Deserialize};

const SOCKET_PATH: &str = "/tmp/robot/detect-socket";

// Client thread for reciveing detection data from ROS node infra
fn start_unix_socket_client_thread( shared_data: Arc<Mutex<Vec<String>>>) -> (JoinHandle<()>) {
    println!("> Connect to Detect server");
    //let (tx, rx) = mpsc::channel::<String>();
    let (response_tx, response_rx) = mpsc::channel::<String>();

    let client_thread = thread::spawn(move || {
        let mut stream = match UnixStream::connect(SOCKET_PATH) {
            Ok(stream) => stream,
            Err(e) => {
                eprintln!("Cannot connect to server: {}", e);
                return;
            }
        };
        println!("> Connection done!");
        loop {
            // Reading from the socket for server responses
            let mut response = [0; 1024];
            match stream.read(&mut response) {
                Ok(size) => {
                    if size == 0 { 
                        // Server closed connection or sent empty message
                        break;
                    }

                    let response_str = String::from_utf8_lossy(&response[..size]).to_string();
                    println!("> Received from server: {:?}", response_str);

                    let mut shared_data_locked = shared_data.lock().unwrap();
                    shared_data_locked.push(response_str);
                },
                Err(e) => {
                    eprintln!("Error reading from socket: {}", e);
                    break;
                }
            }
        }
    });

    client_thread
}




fn main() -> Result<(), eframe::Error> {



    println!("************ Detect GUI: ********** ");

     // Create a shared buffer for storing received messages
    let shared_data: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));

    // Channel for sending messages to the client thread
    //let (tx, response_rx, client_thread) = start_unix_socket_client_thread(shared_data.clone());
    let client_thread = start_unix_socket_client_thread(shared_data.clone());


    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        resizable: false,
        ..Default::default()
    };
    eframe::run_native(
        "Object detection View GUI",
        options,
        Box::new(move |cc| { // 'move' to transfer ownership
            egui_extras::install_image_loaders(&cc.egui_ctx);

            Box::new(MyApp {
                name: "Robot 1".to_owned(),
                item: 42,
                //response_rx: response_rx.clone(), // Clone Arc for shared ownership
                shared_data: shared_data.clone(), // Add this line

            })
        }),
    )
}

struct MyApp {
    name: String,
    item: u32,
    //response_rx: Arc<Mutex<Receiver<String>>>, 
    shared_data: Arc<Mutex<Vec<String>>>, // Add this line



}

//impl Default for MyApp {
//    fn default() -> Self {
//        Self {
//            name: "Robot 1".to_owned(),
//            item: 42,
//        }
//    }
//}




impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Image detection Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("From: ");
                ui.text_edit_singleline(&mut self.name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut self.item, 0..=120).text("item"));
            if ui.button("Click next").clicked() {
                self.item += 1;
            }
            ui.label(format!("From '{}', Object {}", self.name, self.item));

            ui.image(egui::include_image!(
                "/home/rgabbai/projects/Rust/cam_det_pub_node/image_family1.jpg"
            ));

            // Draw a square box on top of the image based on detected objects 
            //let dboxes: Vec<DetObj> = get_detected_objs(self.response_rx.clone());
            let mut prev_dboxes: Vec<DetObj> = Vec::new();
            prev_dboxes.push(DetObj {
                box_location: BoxCor(470.0, 220.0, 50.0, 180.0),
                otype: "person1".to_string(),
            });
            let dboxes: Vec<DetObj> = get_detected_objs(self.shared_data.clone());
            if dboxes.is_empty() {
                // Do something if dboxes is empty
                println!("No objects detected.");
            } else {
                // Do something if dboxes is not empty
                println!("Detected objects: {:?}", dboxes);
            }
            // Iterate over all detected objects
            //for dbox in &dboxes {    
            for dbox in dboxes {
                //println!("> Recive dbox: {:?}",dbox);
                let BoxCor(x,y,width,height) = dbox.box_location;
                let box_type = &dbox.otype;
                let box_rect = Rect::from_min_size(pos2(x, y), vec2(width, height)); // Example coordinates and size
                println!("> box_type: {:?} box_rect {:?}",box_type,box_rect);
                //ui.rect(box_rect, 5.0, Color32::RED); // Adjust color as needed
                let stroke = Stroke::new(2.0, Color32::from_rgb(255, 0, 0));
                //let fill = Color32::from_rgb(0, 255, 0); // Green fill
                //ui.painter().rect(box_rect, 5.0, Color32::from_rgb(255, 0, 0), stroke); // Adjust color as needed
                //ui.painter().rect_stroke(box_rect, 15.0, stroke); // Adjust color as needed
                let label_text = format!("Detected Object Type: {}", box_type);

                // Calculate position for the label on top of the image
                let label_position = pos2(300.0, 500.0); // Adjust the offset as needed

                // Use a horizontal layout to position the label
                    ui.horizontal(|ui| {
                        // Add an empty spacer to push the label to the desired position
                        ui.add_space(label_position.x);
                
                        // Add the label at the specified position
                        ui.label(label_text);
                        ui.painter().rect_stroke(box_rect, 15.0, stroke); // Adjust color as needed
                        println!("> draw ");
                    });
            }
            //let prev_dboxes = dboxes.clone();
               
        });
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct BoxCor(f32,f32,f32,f32);

#[derive(Serialize, Deserialize, Debug)]
struct DetObj {
    box_location: BoxCor,
    otype: String,
}

fn get_detected_objs(shared_data: Arc<Mutex<Vec<String>>>) -> Vec<DetObj> {
    let mut objects = Vec::new();

    {
        let mut data_vec = shared_data.lock().unwrap(); // Acquire lock
        let mut indexes_to_remove = Vec::new();

        for (index, json_str) in data_vec.iter().enumerate() {
            println!("> Received detected data: {:?}", json_str);
            match serde_json::from_str::<Vec<DetObj>>(json_str) {
                Ok(det_objs) => {
                    objects.extend(det_objs);
                    indexes_to_remove.push(index);
                },
                Err(e) => eprintln!("Failed to parse message: {:?}, Error: {:?}", json_str, e),
            }
        }

        // Remove processed data in reverse order to avoid shifting indexes
        //for index in indexes_to_remove.iter().rev() {
        //    data_vec.swap_remove(*index);
        //}
    }

    objects
}



fn parse_to_det_obj_json(data: &str) -> Result<DetObj, Error> {
    serde_json::from_str::<DetObj>(data)
}

#[derive(Debug)]
enum ParseError {
    InvalidFormat,
    // Other error types can be added here
}

fn parse_to_det_obj(data: &str) -> Result<DetObj,ParseError> {
    let parts: Vec<_> = data.split(';').collect();
    if parts.len() == 5 {
        // Assuming these values should be parsed from `parts`
        Ok(DetObj {
            box_location: BoxCor(470.0, 220.0, 50.0, 180.0), // Replace with actual parsed values
            otype: "person1".to_string(), // Replace with actual parsed values
        })
    } else {
        Err(ParseError::InvalidFormat)
    }
}
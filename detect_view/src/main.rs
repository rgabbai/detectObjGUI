#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use eframe::egui;
use egui::Rect;  // Import Rect from the egui module
use egui::{Color32, vec2, pos2,Stroke,TextureOptions};  

use egui::Align as Align;

//Unix sockets
use std::os::unix::net::UnixStream;
use std::io::{Read};
use std::thread;
use std::thread::JoinHandle;
use std::sync::{Arc, Mutex};
use serde::{Serialize, Deserialize};


use std::time::{SystemTime, UNIX_EPOCH};
use eframe::egui::{TextureHandle};
use eframe::epaint::{ColorImage,ImageData};


use std::sync::atomic::{AtomicBool, Ordering};


// Constants
const SOCKET_PATH: &str = "/tmp/robot/detect-socket";
const REC_IMAGE_PATH: &str  = "/home/rgabbai/projects/Rust/obj_det_view_node/received_image.jpg";
//Image offser in GUI
const Y_OFFSET: f32 = 2.0;
const X_OFFSET: f32 = 8.0;
const IMAGE_X_SIZE: f32 = 640.0;
const IMAGE_Y_SIZE: f32 = 360.0;
const X_CENTER: f32 = IMAGE_X_SIZE / 2.0;
const Y_CENTER: f32 = IMAGE_Y_SIZE / 2.0;



// Client thread for reciveing detection data from ROS node infra
fn start_unix_socket_client_thread( shared_data: Arc<Mutex<Vec<String>>>, needs_update: Arc<Mutex<bool>>) -> JoinHandle<()> {
    println!("> Connect to Detect server");

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
                        println!("> server send empty msg");
                        break;
                    }

                    let response_str = String::from_utf8_lossy(&response[..size]).to_string();
                    //println!("> Received from server: {:?}", response_str);

                    let mut shared_data_locked = shared_data.lock().unwrap();
                    shared_data_locked.push(response_str);

                    let mut needs_update = needs_update.lock().unwrap();
                    *needs_update = true;

                    //println!("> set needs_update -Received from server: ");
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

     // Create a shared buffer for storing received detected messages
    let shared_data: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
    let prev_objects: Vec<DetObj> = Vec::new();

    //let needs_update = Arc::new(AtomicBool::new(false));
    let needs_update = Arc::new(Mutex::new(false));


    // Channel for sending messages to the client thread
    let _client_thread = start_unix_socket_client_thread(shared_data.clone(),needs_update.clone());


    env_logger::init(); // Log to stderr (if you run with `RUST_LOG=debug`).
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1000.0, 800.0)),
        resizable: false,
        mouse_passthrough: false,
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
                det_timeout: 20,
                //response_rx: response_rx.clone(), // Clone Arc for shared ownership
                shared_data: shared_data.clone(), // connect client data to GUI
                prev_objects: prev_objects, // state for holding prev valid detected obj
                dynamic_texture: None,
                last_update_time: UNIX_EPOCH,
                needs_update:needs_update.clone(),
            })
        }),
    )
}

struct MyApp {
    name: String,
    item: u32,
    det_timeout: u32,
    shared_data: Arc<Mutex<Vec<String>>>, // Add this line
    prev_objects:  Vec<DetObj>,
    dynamic_texture: Option<TextureHandle>,
    last_update_time: SystemTime,
    needs_update: Arc<Mutex<bool>>,

}




impl MyApp {
    // Load Image from file

    fn load_image_if_updated(&mut self, ctx: &egui::Context) {
        let metadata = std::fs::metadata(REC_IMAGE_PATH).unwrap();
        let modified = metadata.modified().unwrap_or(UNIX_EPOCH);
    
        //println!(">image Meta data{:?} modified: {:?}",metadata,modified);
        if modified > self.last_update_time {
            let image = image::open(REC_IMAGE_PATH).unwrap().to_rgba8();
            //let image = image::imageops::resize(
            //    &image, 
            //    640, 640, 
            //    image::imageops::FilterType::Nearest // You can choose different filters based on your need
            //);

            let size = [image.width() as _, image.height() as _];
    
  
            // Convert the RgbaImage to Vec<Color32>
            let pixels: Vec<Color32> = image
                .pixels()
                .map(|p| Color32::from_rgba_premultiplied(p[0], p[1], p[2], p[3]))
                .collect();
    
            // Create a ColorImage
            let color_image = ColorImage {
                size,
                pixels,
            };
            //println!(">image modified size: {:?}",size );

            // Create ImageData from ColorImage
            let image_data = ImageData::from(color_image);
            
            let texture_options = TextureOptions {
                ..Default::default()  // Use default for other fields
            };
            

            // Load the texture with the ImageData
            let texture = ctx.load_texture(
                "dynamic_image",
                image_data,
                texture_options,
            );
    
            self.dynamic_texture = Some(texture);
            self.last_update_time = modified;
        }
    }

    // Rest of your impl...
}

/*
impl MyApp {
    // Define a method to handle part of the UI
    fn show_image_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(texture) = &self.dynamic_texture {
            // Directly use the TextureHandle
            ui.image(texture);
        } else {
            ui.label("Image not loaded yet.");
        }
    }
}
*/

impl MyApp {
// Place Image and AXIS 
    fn show_image_ui(&mut self, ui: &mut egui::Ui) {
           
        
        if let Some(texture) = &self.dynamic_texture {
            // Draw the image
            ui.image(texture);

            // Draw X and Y axes on the image
            let tex_size = texture.size();
            let axis_color = egui::Color32::from_rgb(255, 255, 255); // White color for axes
            let stroke = egui::Stroke::new(1.0, axis_color);  // 1 pixel line width
   
            //println!("Image size {}, {}",tex_size[0],tex_size[1]); 
            // Center of the image
            let center_x = tex_size[0] as f32 / 2.0 + X_OFFSET;
            let center_y = tex_size[1] as f32 / 2.0 + Y_OFFSET;
   
            // Drawing X axis (horizontal line)
            ui.painter().line_segment(
                [egui::pos2(X_OFFSET, center_y), egui::pos2(tex_size[0] as f32+X_OFFSET, center_y)],
                 stroke,
            );
   
            // Drawing Y axis (vertical line)
            ui.painter().line_segment(
              [egui::pos2(center_x, Y_OFFSET), egui::pos2(center_x, tex_size[1] as f32 + Y_OFFSET)],
              stroke,
            );

        } else {
            ui.label("Image not loaded yet.");
        }
    
    }

    // Rest of your implementation...
}




impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {

        //println!("Update GUI");
        
        // this code was expected to cause update - but it can't cause update is not called unless GUI is tuched.
        // solusion is to activate 
        /*
        let mut update_flag = false;
        {
            let mut needs_update = self.needs_update.lock().unwrap();
            if *needs_update {
                println!(">need_update is true");
                update_flag = true;
                *needs_update = false; // Reset flag
            }
        }
        
        if update_flag {
            println!(">request repaint");
            ctx.request_repaint();
        }
        */
        //TODO - this is costly but didn't find other solution to keep updating 
        ctx.request_repaint();

       // egui::CentralPanel::default().show(ctx, |ui| {
        
            /*
            ui.heading("Image detection Application");
            ui.horizontal(|ui| {
                let name_label = ui.label("From: ");
                ui.text_edit_singleline(&mut self.name)
                    .labelled_by(name_label.id);
            });
            ui.add(egui::Slider::new(&mut self.item, 0..=120).text("item"));
            if ui.button("Click next").clicked() {
                self.item += 1;
                ctx.request_repaint();
            }
            ui.label(format!("From '{}', Object {}", self.name, self.item));
            */
        egui::TopBottomPanel::top("image")
            .show(ctx, |ui| {    
            self.load_image_if_updated(ctx);
            self.show_image_ui(ui);

            // Draw a square box on top of the image based on detected objects 
            //let dboxes: Vec<DetObj> = get_detected_objs(self.response_rx.clone());

            let mut dboxes: Vec<DetObj> = get_detected_objs(self.shared_data.clone());
            if dboxes.is_empty() {
                dboxes = self.prev_objects.clone();
             } else {
                // Do something if dboxes is not empty
                //println!("Detected objects: {:?}", dboxes);
                self.prev_objects = dboxes.clone();
            }
            // Iterate over all detected objects
            //for dbox in &dboxes {    
            for dbox in dboxes {
                //println!("> Recive dbox: {:?}",dbox);
                let BoxCor(x1,y1,x2,y2) = dbox.box_location;
                //println!("> Detect Cor: {x1} {y1} {x2} {y2}");

                let height = y2-y1;  
                let width = x2-x1; 
                //println!("> width:{width} height:{height}");
                let y1 = y1 + Y_OFFSET; // Y offset fix top left corner point
                let x1 = x1 + X_OFFSET; // Y offset fix top left corner point


                let box_type = &dbox.otype;
                let mut est_dist:f64 = 0.0;
                if box_type == "pylon" {
                    est_dist = (&dbox.dist*10.0).round() / 10.0; // round dist 
                } else {est_dist = 0.0;}
                let box_rect = Rect::from_min_size(pos2(x1, y1), vec2(width, height)); // Example coordinates and size
                //println!("> box_type: {:?} box_rect {:?}",box_type,box_rect);
                let stroke = Stroke::new(2.0, Color32::from_rgb(255, 0, 0));
                let label_text = format!("Detected Object Type: {} Probability: {}  Pixel height:{} Est distance [m] {}", 
                    box_type,
                    &dbox.prob,
                    height,
                    est_dist,
                );

                // Calculate position for the label on top of the image
                let label_position = pos2(50.0, 500.0); // Adjust the offset as needed

                // Use a horizontal layout to position the label
                    ui.horizontal(|ui| {
                        // Add an empty spacer to push the label to the desired position
                        ui.add_space(label_position.x);
                
                        // Add the label at the specified position
                        ui.label(label_text);
                        ui.painter().rect_stroke(box_rect, 15.0, stroke); // Adjust color as needed
                        //println!("> draw ");
                    });
            }
            //let prev_dboxes = dboxes.clone();
               
        });
    }
}
#[derive(Serialize, Deserialize, Debug, Clone)]
struct BoxCor(f32,f32,f32,f32);

#[derive(Serialize, Deserialize, Debug, Clone)]
struct DetObj {
    box_location: BoxCor,
    otype: String,
    prob: f32,
    dist: f64,
}

fn get_detected_objs(shared_data: Arc<Mutex<Vec<String>>>) -> Vec<DetObj> {
    let mut objects = Vec::new();

    {
        let mut data_vec = shared_data.lock().unwrap(); // Acquire lock
        let mut indexes_to_remove = Vec::new();

        for (index, json_str) in data_vec.iter().enumerate() {
            //println!("> Received detected data: {:?}", json_str);
            match serde_json::from_str::<Vec<DetObj>>(json_str) {
                Ok(det_objs) => {
                    objects.extend(det_objs);
                    indexes_to_remove.push(index);
                },
                Err(e) => eprintln!("Failed to parse message: {:?}, Error: {:?}", json_str, e),
            }
        }

        // Remove processed data in reverse order to avoid shifting indexes
        //println!("before shared data {:?}",data_vec);
        for index in indexes_to_remove.iter().rev() {
            data_vec.swap_remove(*index);
        }
        //println!("after shared data {:?}",data_vec);

    }
    //println!("Objects: {:?}",objects);
    objects
}




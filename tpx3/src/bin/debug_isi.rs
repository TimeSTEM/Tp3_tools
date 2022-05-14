use timepix3::tdclib::isi_box;
use std::sync::{Arc, Mutex};
use std::{thread, time};

fn main() {
    loop {
        println!("Starting new debug session...");
        //let nvec_list: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let mut handler = isi_box::IsiBoxHandler::new();
        let time = time::Duration::from_millis(100);
        handler.run();
        //isi_box::connect(&nvec_list);
        for _ in 0..25 {
            thread::sleep(time);
            handler.request2();
            //isi_box::request(&nvec_list);
        }
    }
}

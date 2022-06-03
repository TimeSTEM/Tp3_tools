use timepix3::tdclib::{isi_box, isi_box::{CHANNELS, IsiBoxTools, IsiBoxHand}};
use timepix3::isi_box_new;
use std::{thread, time};

fn main() {
        println!("Starting new debug session...");
        let mut handler = isi_box_new!(spim);
        handler.bind_and_connect();
        handler.configure_scan_parameters(32, 32, 8334);
        handler.configure_measurement_type();
        //let time = time::Duration::from_millis(100);
        handler.start_threads();
        for _ in 0..10 {
            handler.get_data();
            thread::sleep(time::Duration::from_millis(1));
        }
}

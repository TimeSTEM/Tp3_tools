use timepix3::tdclib::{isi_box, isi_box::{CHANNELS, IsiBoxTools, IsiBoxHand}};
use timepix3::isi_box_new;
use std::{thread, time};
use std::fs::File;

fn main() {
    let f = File::open("isi_raw105.isi").unwrap();
    isi_box::get_channel_timelist(f);
    /*
    println!("Starting new debug session...");
    let mut handler = isi_box_new!(spec);
    handler.bind_and_connect();
    handler.configure_scan_parameters(32, 32, 8334);
    handler.configure_measurement_type();
    let time = time::Duration::from_millis(100);
    handler.start_threads();
    for _ in 0..10 {
        handler.get_data();
        thread::sleep(time::Duration::from_millis(1));
    }
    */
}

//use timepix3::tdclib::isi_box::{CHANNELS, IsiBoxTools, IsiBoxHand};
//use timepix3::postlib::isi_box;
use timepix3::postlib::coincidence::*;
use timepix3::auxiliar::ConfigAcquisition;
use timepix3::clusterlib::cluster;
use std::env;
//use timepix3::isi_box_new;
//use std::{thread, time};
//use std::fs::File;

fn main() {
    //Only IsiBox
    //let f = File::open("isi_raw239.isi").unwrap();
    //isi_box::get_channel_timelist(f);
    
    let args: Vec<String> = env::args().collect();
    let config_set = ConfigAcquisition::new(&args[0..6], cluster::NoCorrection);
    let mut coinc_data = ElectronData::new(&config_set);
    search_coincidence_isi(&config_set.file(), &args[6], &mut coinc_data).unwrap();
    
    coinc_data.output_spectrum(true);
    coinc_data.output_corr_spectrum(false);
    coinc_data.output_relative_time();
    coinc_data.output_time();
    coinc_data.output_g2_time();
    coinc_data.output_channel();
    coinc_data.output_dispersive();
    coinc_data.output_non_dispersive();
    coinc_data.output_spim_index();
    coinc_data.output_tot();


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

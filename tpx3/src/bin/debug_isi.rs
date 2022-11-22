//use timepix3::tdclib::isi_box::{CHANNELS, IsiBoxTools, IsiBoxHand};
//use timepix3::postlib::isi_box;
use timepix3::postlib::coincidence::*;
use timepix3::auxiliar::ConfigAcquisition;
use timepix3::clusterlib::{cluster, cluster::ClusterCorrection};
use timepix3::cluster_correction;
use std::env;
//use timepix3::isi_box_new;
//use std::{thread, time};
//use std::fs::File;

fn main() {
    let args: Vec<String> = env::args().collect();
    match args[5].parse::<usize>() {
        Ok(0) => {},
        Ok(1) => {},
        Ok(_) => {},
        Err(_) => {},
    }
    //let config_set = ConfigAcquisition::new(&args[0..6], cluster::ClosestToTWithThreshold(50, 30));
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
}

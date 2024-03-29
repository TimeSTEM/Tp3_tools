//use timepix3::tdclib::isi_box::{CHANNELS, IsiBoxTools, IsiBoxHand};
//use timepix3::postlib::isi_box;
//use timepix3::postlib::coincidence::*;
use timepix3::auxiliar::ConfigAcquisition;
use timepix3::clusterlib::cluster;
//use timepix3::cluster_correction;
use std::env;
//use timepix3::isi_box_new;
//use std::{thread, time};
//use std::fs::File;

fn main() {
    let args: Vec<String> = env::args().collect();
    let cluster_correction_type = cluster::grab_cluster_correction(&args[5]);
    let _config_set = ConfigAcquisition::new(&args, cluster_correction_type);
    /*
    //TODO: Must fix this otherwise no data will be output.
    let mut coinc_data = ElectronData::new(config_set);
    search_coincidence_isi(&args[6], &mut coinc_data).unwrap();
    
    coinc_data.output_spectrum();
    coinc_data.output_corr_spectrum();
    coinc_data.output_relative_time();
    coinc_data.output_time();
    coinc_data.output_g2_time();
    coinc_data.output_channel();
    coinc_data.output_dispersive();
    coinc_data.output_non_dispersive();
    coinc_data.output_spim_index();
    coinc_data.output_tot();
    coinc_data.output_cluster_size();
    */
}

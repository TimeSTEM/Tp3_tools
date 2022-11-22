use timepix3::postlib::coincidence::*;
use timepix3::auxiliar::ConfigAcquisition;
use timepix3::clusterlib::cluster;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args: Vec<String> = env::args().collect();
    let cluster_correction_type = cluster::grab_cluster_correction(&args[5]);
    let config_set = ConfigAcquisition::new(&args[0..6], cluster_correction_type);
    let mut coinc_data = ElectronData::new(config_set);
    search_coincidence(&mut coinc_data)?;
    
    //let mut entries = fs::read_dir("DataCoinc")?;
    //while let Some(x) = entries.next() {
    //    let path = x?.path();
    //    let dir = path.to_str().unwrap();
    //    println!("Looping over file {:?}", dir);
    //    search_coincidence(dir, &mut coinc_data)?;
    //}

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

    Ok(())
}



use timepix3::postlib::ntime_resolved::*;
use timepix3::auxiliar::ConfigAcquisition;
use timepix3::clusterlib::cluster;
use std::env;

fn main() -> Result<(), ErrorType> {
    let args: Vec<String> = env::args().collect();
    let cluster_correction_type = cluster::grab_cluster_correction(&args[5]);
    let config_set = ConfigAcquisition::new(&args, cluster_correction_type);
    let mut meas = TimeSpectralSpatial::new(config_set, true)?;
    analyze_data(&mut meas);

    Ok(())
}


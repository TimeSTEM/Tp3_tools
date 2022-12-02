use timepix3::postlib::calibration::*;
use timepix3::auxiliar::ConfigAcquisition;
use timepix3::clusterlib::{cluster, cluster::ClusterCorrection};
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args: Vec<String> = env::args().collect();
    //let config_set = ConfigAcquisition::new(&args, cluster::FixedToTCalibration(70));
    let config_set = ConfigAcquisition::new(&args, cluster::SingleClusterToTCalibration);
    //let config_set = ConfigAcquisition::new(&args, cluster::AverageCorrection);
    calibrate(&config_set.file(), &config_set.correction_type).unwrap();
    Ok(())
}



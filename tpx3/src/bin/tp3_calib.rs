use timepix3::postlib::calibration::*;
use timepix3::auxiliar::ConfigAcquisition;
use timepix3::clusterlib::cluster;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args: Vec<String> = env::args().collect();
    
    let config_set = ConfigAcquisition::new(&args, cluster::FixedToTCalibration(30, 60));
    let config_set = ConfigAcquisition::new(&args, cluster::MuonTrack);
    //let config_set = ConfigAcquisition::new(&args, cluster::SingleClusterToTCalibration);
    
    //let config_set = ConfigAcquisition::new(&args, cluster::NoCorrectionVerbose);
    //let config_set = ConfigAcquisition::new(&args, cluster::AverageCorrection);
    calibrate(&config_set.file(), &config_set.correction_type).unwrap();
    Ok(())
}



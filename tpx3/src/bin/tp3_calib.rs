use timepix3::postlib::Calibration::*;
use timepix3::auxiliar::ConfigAcquisition;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args: Vec<String> = env::args().collect();
    let config_set = ConfigAcquisition::new(&args);
    calibrate(&config_set.file()).unwrap();
    Ok(())
}



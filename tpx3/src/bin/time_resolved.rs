use timepix3::postlib::ntime_resolved::*;
use timepix3::tdclib::TdcType;
use timepix3::auxiliar::ConfigAcquisition;
use std::{env};

fn main() -> Result<(), ErrorType> {
    let args: Vec<String> = env::args().collect();
    let config_set = ConfigAcquisition::new(&args);
   
    let mut meas = TimeSpectralSpatial::new(&config_set)?;
    analyze_data(&config_set.file(), &mut meas);

    Ok(())
}


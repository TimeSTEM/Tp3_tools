use timepix3::postlib::time_resolved::*;
use timepix3::tdclib::TdcType;
use std::fs;

fn main() -> Result<(), ErrorType> {
    let time = 1e9 as usize;
    
    /*
    let mut specs = TimeSet {
        set: 
            vec![Box::new(TimeSpectral::new(time, 0, 1024, String::from("TimeSpectral"))?),
            Box::new(TimeSpectral::new(time, 30, 1024, String::from("TimeSpectral"))?),
            Box::new(TimeSpectral::new(time, 30, 250, String::from("TimeSpectral"))?),
            Box::new(TimeSpectral::new(time, 50, 57, String::from("TimeSpectral"))?),
            Box::new(TimeSpectral::new(time, 98, 110, String::from("TimeSpectral"))?),
            Box::new(TimeSpectral::new(time, 664, 673, String::from("TimeSpectral"))?),
            Box::new(TimeSpectral::new(time, 664, 705, String::from("TimeSpectral"))?),
            //Box::new(TimeSpectral::new(time, 667, 671, String::from("TimeSpectral"))?),
            Box::new(TimeSpectral::new(time, 794, 810, String::from("TimeSpectral"))?),
            Box::new(TimeSpectral::new(time, 794, 801, String::from("TimeSpectral"))?),
            Box::new(TimeSpectral::new(time, 801, 810, String::from("TimeSpectral"))?)],
    };
    */
    
    let spim_size = 67;

    let mut specs = TimeSet {
        set:
            vec![Box::new(TimeSpectralSpatial::new(time, 0, 1024, spim_size, spim_size, None, None, None, TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral"))?),
            Box::new(TimeSpectralSpatial::new(time, 0, 1024, spim_size, spim_size, Some(5), Some(5), Some(10), TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral02"))?),
            Box::new(TimeSpectralSpatial::new(time, 0, 1024, spim_size, spim_size, Some(25), Some(25), Some(10), TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral02"))?),
            Box::new(TimeSpectralSpatial::new(time, 0, 1024, spim_size, spim_size, Some(45), Some(45), Some(10), TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral02"))?),
            Box::new(TimeSpectralSpatial::new(time, 0, 1024, spim_size, spim_size, Some(65), Some(65), Some(10), TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral02"))?)],
    };


    let mut entries = fs::read_dir("Data").expect("Could not read the directory.");
    while let Some(x) = entries.next() {
        let path = x.unwrap().path();
        let dir = path.to_str().unwrap();
        println!("Looping over file {:}.", dir);
        analyze_data(dir, &mut specs);
    }

    for spec in specs.set.iter_mut() {
        spec.display_info()?;
        spec.output()?;
    }

    //println!("Total number of spectra are: {}. Total number of electrons are: {:?}. Electrons / spectra is {}. First electron detected at {:?}.", specs.spectra.len(), specs.total_electrons(), specs.total_electrons() / specs.spectra.len(), specs.initial_time);


    Ok(())
}

use timepix3::postlib::time_resolved::*;
use std::fs;

fn main() -> Result<(), ErrorType> {
    //let mut specs = TimeSpectral::new(1e8 as usize, 31, 1024)?;
    let mut specs = TimeSet {
        set: 
            vec![Box::new(TimeSpectral::new(1e8 as usize, 31, 1024, String::from("TimeSpectral"))?),
            Box::new(TimeSpectral::new(1e7 as usize, 125, 1024, String::from("TimeSpectral"))?),
            Box::new(TimeSpectral::new(1e9 as usize, 12, 102, String::from("TimeSpectral"))?)],
    };

    let mut entries = fs::read_dir("Data").expect("Could not read the directory.");
    while let Some(x) = entries.next() {
        let path = x.unwrap().path();
        let dir = path.to_str().unwrap();
        println!("Looping over file {:}.", dir);
        analyze_data(dir, &mut specs);
    }

    for spec in specs.set.iter_mut() {
        spec.output()?;
    }

    //println!("Total number of spectra are: {}. Total number of electrons are: {:?}. Electrons / spectra is {}. First electron detected at {:?}.", specs.spectra.len(), specs.total_electrons(), specs.total_electrons() / specs.spectra.len(), specs.initial_time);


    Ok(())
}

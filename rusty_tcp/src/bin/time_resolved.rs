use timepix3::postlib::time_resolved::*;
use std::fs;

fn main() -> Result<(), ErrorType> {
    //let mut specs = TimeSpectral::new(1e9 as usize);
    let mut specs = TimePixel::new(1e9 as usize, 600, 900)?;

    let mut entries = fs::read_dir("Data").expect("Could not read the directory.");
    while let Some(x) = entries.next() {
        let path = x.unwrap().path();
        let dir = path.to_str().unwrap();
        println!("Looping over file {:}.", dir);
        analyze_data(dir, &mut specs);
    }

    specs.output_all()?;

    println!("Total number of spectra are: {}. Total number of electrons are: {:?}. Electrons / spectra is {}. First electron detected at {:?}.", specs.spectra.len(), specs.total_electrons(), specs.total_electrons() / specs.spectra.len(), specs.initial_time);


    Ok(())
}

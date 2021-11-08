use timepix3::postlib::coincidence::*;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let args: Vec<String> = env::args().collect();
    let config_set = Config::new(&args);
    let mut coinc_data = ElectronData::new(&config_set);
    search_coincidence(&config_set.file(), &mut coinc_data)?;
    
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
    coinc_data.output_dispersive();
    coinc_data.output_non_dispersive();

    Ok(())
}



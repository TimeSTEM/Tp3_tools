use timepix3::postlib::time_resolved::*;
use std::fs;
use std::time::Instant;

fn main() {
    let mut specs = TimeSpectral::new(1e9 as usize);

    let mut entries = fs::read_dir("Data").expect("Could not read the directory.");
    while let Some(x) = entries.next() {
        let path = x.unwrap().path();
        let dir = path.to_str().unwrap();
        println!("Looping over file {:}.", dir);
        analyze_data("Data/raw000000.tpx3", &mut specs);
    }

    println!("{}", specs.spectra.len());
    println!("{}", specs.counter);


}

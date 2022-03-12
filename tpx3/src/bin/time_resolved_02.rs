use timepix3::postlib::ntime_resolved::*;
use timepix3::tdclib::TdcType;
use std::fs;

fn main() -> Result<(), ErrorType> {
    let time_per_slice = (41e6/1.5625) as usize; //Time, in units of 640 Mhz (1.5625 ns);
    let spim_size = 32; //Size of the spim;
    
    let mut my_vec: Vec<Box<dyn TimeTypes>> = Vec::new();
    my_vec.push(Box::new(TimeSpectralSpatial::new(time_per_slice, spim_size, spim_size, true, TdcType::TdcOneFallingEdge, String::from("test/results"))?));
    let mut specs = TimeSet {
        set: my_vec,
    };

    let mut entries = fs::read_dir("backupPaper").expect("Could not read the directory.");
    while let Some(x) = entries.next() {
        let path = x.unwrap().path();
        let dir = path.to_str().unwrap();
        analyze_data(dir, &mut specs);
    }

    for spec in specs.set.iter_mut() {
        spec.display_info()?;
        spec.output()?;
    }

    Ok(())
}


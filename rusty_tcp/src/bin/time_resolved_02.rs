use timepix3::postlib::ntime_resolved::*;
use timepix3::tdclib::TdcType;
use std::fs;

fn main() -> Result<(), ErrorType> {
    let time = 4e9 as usize;
    
    let spim_size = 32;
    
    let mut my_vec: Vec<Box<dyn TimeTypes>> = Vec::new();
    my_vec.push(Box::new(TimeSpectralSpatial::new(time, spim_size, spim_size, true, TdcType::TdcOneFallingEdge, String::from("testSpim_new_new_new"))?));
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


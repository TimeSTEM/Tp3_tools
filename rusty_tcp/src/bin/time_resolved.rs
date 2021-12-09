use timepix3::postlib::time_resolved::*;
use timepix3::tdclib::TdcType;
use std::fs;

fn main() -> Result<(), ErrorType> {
    let time = 4e11 as usize;
    
    let line_offset = 23;
    let spim_size = 32;
    
    let mut my_vec: Vec<Box<dyn TimeTypes>> = Vec::new();
    my_vec.push(Box::new(TimeSpectralSpatial::new(time, 0, 1024, spim_size, spim_size, line_offset, None, TdcType::TdcOneFallingEdge, String::from("testSpim_new_new"))?));
    let mut specs = TimeSet {
        set: my_vec,
    };

    let mut entries = fs::read_dir("backupPaper").expect("Could not read the directory.");
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

    Ok(())
}


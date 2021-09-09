use timepix3::postlib::time_resolved::*;
use timepix3::tdclib::TdcType;
use std::fs;

fn main() -> Result<(), ErrorType> {
    let time = 1e8 as usize;
    
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
    
    let line_offset = 22;
    let offset = 27;
    let spim_size = 32;
    let first = 54+offset;
    let second = 103+offset;
    let carbon = 670+offset;
    let calcium = 801+offset;
    let cell_size = 8;
    let posx = 25;
    let posy = 25;

    let mut my_vec: Vec<Box<dyn TimeTypes>> = Vec::new();
    
    
    my_vec.push(Box::new(TimeSpectralSpatial::new(1e9 as usize, 0, 1024, spim_size, spim_size, line_offset, Some((spim_size/2, spim_size/2, spim_size)), TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral_position/complete"))?));
    my_vec.push(Box::new(TimeSpectralSpatial::new(1e9 as usize, 0, 1024, spim_size, spim_size, line_offset, None, TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral"))?));
    

    /*
    for x in (0..spim_size+1).step_by(cell_size) {
        my_vec.push(
            Box::new(TimeSpectralSpatial::new(time, first-5, first+5, spim_size, spim_size, Some((x, x, cell_size)), TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral_position/first"))?)
            );
        my_vec.push(
            Box::new(TimeSpectralSpatial::new(time, second-5, second+5, spim_size, spim_size, Some((x, x, cell_size)), TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral_position/second"))?)
            );
        my_vec.push(
            Box::new(TimeSpectralSpatial::new(time, carbon-5, carbon+30, spim_size, spim_size, Some((x, x, cell_size)), TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral_position/carbon"))?)
            );
        my_vec.push(
            Box::new(TimeSpectralSpatial::new(time, calcium-5, calcium+15, spim_size, spim_size, Some((x, x, cell_size)), TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral_position/calcium"))?)
            );
    }
    my_vec.push(
        Box::new(TimeSpectralSpatial::new(time, first-5, first+5, spim_size, spim_size, Some((spim_size/2, spim_size/2, spim_size)), TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral_position/first"))?)
        );
    my_vec.push(
        Box::new(TimeSpectralSpatial::new(time, second-5, second+5, spim_size, spim_size, Some((spim_size/2, spim_size/2, spim_size)), TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral_position/second"))?)
        );
    my_vec.push(
        Box::new(TimeSpectralSpatial::new(time, carbon-5, carbon+30, spim_size, spim_size, Some((spim_size/2, spim_size/2, spim_size)), TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral_position/carbon"))?)
        );
    my_vec.push(
        Box::new(TimeSpectralSpatial::new(time, calcium-5, calcium+15, spim_size, spim_size, Some((spim_size/2, spim_size/2, spim_size)), TdcType::TdcOneFallingEdge, String::from("SpimTimeSpectral_position/calcium"))?)
        );
    */

    

    let mut specs = TimeSet {
        set: my_vec,
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

    Ok(())
}

use timepix3::auxiliar::raw_into_readable;
use std::{fs, env};
use rayon::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let args: Vec<String> = env::args().collect();

    println!("
    ***Instructions***:
    A single argument must be parsed, which is the folder containing your (multiple) .tpx3 data. 
    "
    );
    
    let entries = fs::read_dir(&args[1]).unwrap();
    entries.into_iter().par_bridge().for_each(|x| {
        let path = x.unwrap().path();
        let dir = path.to_str().unwrap();
        let path_length = dir.len();
        if &dir[path_length - 4 ..path_length] == "tpx3" {
            if let Err(error) = raw_into_readable::build_data(dir, 0) {
                println!("***List analysis***: Error during treatment: {:?}.", error);
            }
        }
    });
    Ok(())
}



use timepix3::postlib::coincidence::*;
use timepix3::auxiliar::ConfigAcquisition;
use timepix3::clusterlib::cluster;
use std::{fs, env};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let mut args: Vec<String> = env::args().collect();
    
    let mut entries = fs::read_dir(&args[1]).unwrap();
    while let Some(x) = entries.next() {
        let path = x?.path();
        let dir = path.to_str().unwrap();
        let path_length = dir.len();
        if &dir[path_length - 4 ..path_length] == "tpx3" {
            println!("***Coincidence***: Looping over file {:?}", dir);
            args[1] = dir.to_string();
            let cluster_correction_type = cluster::grab_cluster_correction(&args[5]);
            let config_set = ConfigAcquisition::new(&args, cluster_correction_type);
            let mut coinc_data = ElectronData::new(config_set);
            if let Err(_) = search_coincidence(&mut coinc_data) {
                println!("***Coincidence***: Skipping file {}. Possibly already done it.", args[1]);
            }
        }
    }


    //let cluster_correction_type = cluster::grab_cluster_correction(&args[5]);
    //let config_set = ConfigAcquisition::new(&args, cluster_correction_type);
    //let mut coinc_data = ElectronData::new(config_set);
    //search_coincidence(&mut coinc_data)?;
    

    //coinc_data.output_data();

    Ok(())
}



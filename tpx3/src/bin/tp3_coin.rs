use timepix3::postlib::coincidence::*;
use timepix3::auxiliar::{ConfigAcquisition, Settings};
use timepix3::clusterlib::cluster;
use std::{fs, env};
use rayon::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let args: Vec<String> = env::args().collect();

    println!("
    ***Instructions***:

    Turn the script using the --release tag. A single argument must be parsed, which is the folder containing your .tpx3 data. Data must be accompanied with a json file, with matching names. Parsing other arguments will be ignored🙅‍♂️.
    Example:

    >>  cargo run --bin tp3_coin --release 'C:\\Users\\SpongeBob\\Data'

    Example of the json file and the required fields:

    {{bin: false, bytedepth: 2, cumul: false, mode: 0, xspim_size: 10, yspim_size: 10, xscan_size: 512, yscan_size: 512, pixel_time: 320, time_delay: 0, time_width: 0, spimoverscanx: 1, spimoverscany: 1, save_locally: true, sup0: 0.0155, sup1: 0.0}}

    For this particular script:
        🤜 (mode == 0) => No hyperspectral image;
        🤜 (mode == 1) => Hyperspectral image;
            👀 xscan_size & yscan_size => Hyperspectral image sampling;
        🔧 Cluster correction is deactivated. Please request if you wish to do o;
        🕯️ The other fields of the json are currently not considered, but they give you the parameters you have used during data
        acquisition. sup0 & sup1, for example, are the EELS dispersion and offset, respectively;
        💡 The time delay and time width are defined at compile-time, so you should change them at constlib.rs insted;

    "
    );

    let entries = fs::read_dir(&args[1]).unwrap();
    entries.into_iter().par_bridge().for_each(|x| {
        let path = x.unwrap().path();
        let dir = path.to_str().unwrap();
        let path_length = dir.len();
        if &dir[path_length - 4 ..path_length] == "tpx3" {
            //println!("***Coincidence***: Looping over file {:?}", dir);
            if let Ok(settings) = Settings::get_settings_from_json(&dir[0..path_length - 5]) {
                let config_set = ConfigAcquisition{file: dir.to_owned(), is_spim: settings.mode != 0, xspim: settings.xscan_size, yspim: settings.yscan_size, correction_type: cluster::grab_cluster_correction("0")};
                println!("***Coincidence***: File {} has the following settings from json: {:?}.", dir, settings);
                let mut coinc_data = ElectronData::new(config_set);
                if let Err(_) = search_coincidence(&mut coinc_data) {
                    println!("***Coincidence***: Skipping file {}. Possibly already done it.", dir);
                }
            } else {
                println!("***Coincidence***: Skipping file {}. No JSON file is present.", dir);
            }

            
            /*
            args_copy[1] = dir.to_string();
            */

            //let cluster_correction_type = cluster::grab_cluster_correction(&args[5]);
            //let config_set = ConfigAcquisition::new(&args_copy, cluster_correction_type);
        }
    });


    //let cluster_correction_type = cluster::grab_cluster_correction(&args[5]);
    //let config_set = ConfigAcquisition::new(&args, cluster_correction_type);
    //let mut coinc_data = ElectronData::new(config_set);
    //search_coincidence(&mut coinc_data)?;
    

    //coinc_data.output_data();

    Ok(())
}



use timepix3::postlib::coincidence::*;
use timepix3::clusterlib::cluster;
use timepix3::auxiliar::Settings;
use std::{fs, env};
use rayon::prelude::*;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let args: Vec<String> = env::args().collect();

    println!("
    ***Instructions***:

    At least a single argument must be parsed, which is the folder containing your (multiple) .tpx3 data. Data must be accompanied with a json file, with matching names. If there are more than one .tpx3 file, data treatment is done in parallel.

    Example of the json file and the required fields:

    {{bin: false, bytedepth: 2, cumul: false, mode: 0, xspim_size: 10, yspim_size: 10, xscan_size: 512, yscan_size: 512, pixel_time: 320, time_delay: 0, time_width: 0, spimoverscanx: 1, spimoverscany: 1, save_locally: true, sup0: 0.0155, sup1: 0.0}}


    For this particular script:
        -> (mode != 0) => No hyperspectral image;
        -> (mode == 2) => Hyperspectral image;
            o xscan_size & yscan_size => Hyperspectral image sampling;
        -> Cluster correction can be activated by parsing a second argument, varying from 0 to 5. There are three automatic cluster correction settings:
            o '0' => No correction;
            o '1' => Average;
            o '2' => Maximum ToT;
        -> The other fields of the json are currently not considered, but they give you the parameters you have used during data
        acquisition. sup0 & sup1, for example, are the EELS dispersion and offset, respectively;
        -> The time delay and time width are defined at compile-time, so you should change them at constlib.rs insted;

    "
    );
    
    let cluster_correction = if args.get(2).is_none() {
        "0"
    } else {
        &args[2]
    };

    let entries = fs::read_dir(&args[1]).unwrap();
    entries.into_iter().par_bridge().for_each(|x| {
        let path = x.unwrap().path();
        let dir = path.to_str().unwrap();
        let path_length = dir.len();
        if &dir[path_length - 4 ..path_length] == "tpx3" {
            if let Ok(settings) = Settings::get_settings_from_json(&dir[0..path_length - 5]) {
                println!("***Coincidence***: File {} has the following settings from json: {:?}.", dir, settings);
                let mut electron_data = ElectronData::new(dir.to_owned(), cluster::grab_cluster_correction(cluster_correction), settings);
                if let Err(error) = electron_data.prepare_to_search() {
                    println!("***Coincidence***: Error during prepare: {:?}.", error);
                }
                search_coincidence(&mut electron_data);
            } else {
                println!("***Coincidence***: Skipping file {}. No JSON file is present.", dir);
         }
        }
    });
    Ok(())
}



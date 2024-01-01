use timepix3::postlib::ntime_resolved::*;
use timepix3::auxiliar::{ConfigAcquisition, Settings};
use timepix3::clusterlib::cluster;
use timepix3::errorlib::Tp3ErrorKind;
use std::{fs, env};
use rayon::prelude::*;

fn main() -> Result<(), Tp3ErrorKind> {
    let args: Vec<String> = env::args().collect();
    
    println!("
    ***Instructions***:

    At least a single argument must be parsed, which is the folder containing your (multiple) .tpx3 data. Data must be accompanied with a json file, with matching names. If there are more than one .tpx3 file, data treatment is done in parallel.

    Example of the json file and the required fields:

    {{bin: false, bytedepth: 2, cumul: false, mode: 0, xspim_size: 10, yspim_size: 10, xscan_size: 512, yscan_size: 512, pixel_time: 320, time_delay: 0, time_width: 0, spimoverscanx: 1, spimoverscany: 1, save_locally: true, sup0: 0.0155, sup1: 0.0}}

    For this particular script:
        -> (mode == 2) => Hyperspectral image;
        -> (mode != 2) => 4D image;
            o xscan_size & yscan_size = Spatial sampling;
        -> Cluster correction can be activated by parsing a second argument, varying from 0 to 5. There are three automatic cluster correction settings:
            o '0' => No correction;
            o '1' => Average;
            o '2' => Maximum ToT;
        -> The other fields of the json are currently not considered, but they give you the parameters you have used during data
        acquisition. sup0 & sup1, for example, are the EELS dispersion and offset, respectively;

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
                let config_set = ConfigAcquisition{file: dir.to_owned(), is_spim: settings.mode != 0, xspim: settings.xscan_size, yspim: settings.yscan_size, correction_type: cluster::grab_cluster_correction(cluster_correction)};
                println!("***Time resolved***: File {} has the following settings from json: {:?}.", dir, settings);
                let mut meas = TimeSpectralSpatial::new(config_set, settings).unwrap();
                if let Err(_) = analyze_data(&mut meas) {
                    println!("***Time resolved***: Skipping file {}. Possibly already done it.", dir);
                }
            } else {
                println!("***Time resolved***: Skipping file {}. No JSON file is present.", dir);
            }
        }
    });
    Ok(())
}


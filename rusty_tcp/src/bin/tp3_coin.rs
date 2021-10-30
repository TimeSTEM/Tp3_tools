use timepix3::postlib::coincidence::*;
use std::fs;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {

    let mut coinc_data = ElectronData::new();
 
    let start = Instant::now();

    let mut entries = fs::read_dir("DataCoinc")?;
    while let Some(x) = entries.next() {
        let path = x?.path();
        let dir = path.to_str().unwrap();
        println!("Looping over file {:?}", dir);
        search_coincidence(dir, &mut coinc_data)?;
    }
    println!("Total time elapsed is: {:?}", start.elapsed());


    coinc_data.output_spectrum(true);
    coinc_data.output_corr_spectrum(false);
    coinc_data.output_relative_time();
    coinc_data.output_dispersive();


    /*
    let output_vec: Vec<String> = cele_vec.iter().map(|x| x.to_string()).collect();
    let output_string = output_vec.join(", ");
    fs::write("xyH.txt", output_string)?;
    
    let output_vec: Vec<String> = ele_vec.iter().map(|x| x.to_string()).collect();
    let output_string = output_vec.join(", ");
    fs::write("xHT.txt", output_string)?;
    
    let output_vec: Vec<String> = cluster_list.iter().map(|(_, trel, _, _, _, _)| trel.to_string()).collect();
    let output_string = output_vec.join(", ");
    fs::write("tH.txt", output_string)?;
    
    let output_vec: Vec<String> = cluster_list.iter().map(|&(_, _, _, _, tot, cs)| (tot as usize*cs).to_string()).collect();
    let output_string = output_vec.join(", ");
    fs::write("stot.txt", output_string)?;
    
    let output_vec: Vec<String> = cluster_list.iter().map(|(_, _, _, _, _, cs)| cs.to_string()).collect();
    let output_string = output_vec.join(", ");
    fs::write("cs.txt", output_string)?;
    */

    Ok(())
}



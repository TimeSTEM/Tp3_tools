//use timepix3::auxiliar::{RunningMode, BytesConfig, Settings};
//use timepix3::tdclib::{TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
//use timepix3::{modes, misc};

//use plotters::prelude::*;
use timepix3::packetlib::PacketEELS as Packet;
use timepix3::tdclib::TdcType;
use timepix3::postlib::postproc;
use std::io;
use std::io::prelude::*;
use std::fs;
use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let mut ele_vec:Vec<usize> = vec![0; 1024];
    let mut cele_vec:Vec<usize> = vec![0; 1024*256];
    let mut cluster_list:Vec<(f64, f64, usize, usize, u16, usize)> = Vec::new();
            
    let mut nphotons = 0usize;
    let mut entries = fs::read_dir("Data")?;
    while let Some(x) = entries.next() {
        let path = x?.path();
        let dir = path.to_str().unwrap();
        println!("Looping over file {:?}", dir);
        nphotons += postproc::search_coincidence(dir, &mut ele_vec, &mut cele_vec, &mut cluster_list)?;
    }
    println!("The number of photons is: {}", nphotons);

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
 
    Ok(())
}



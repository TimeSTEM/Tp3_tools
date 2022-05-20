use std::fs::File;
use std::io::prelude::*;
use timepix3::packetlib::*;
use timepix3::tdclib::TdcType;
use rand_distr::{Normal, Distribution};
use rand::distributions::Uniform;
use rand::{thread_rng};

fn main() {

    const ELE_PA: f64 = 6.25 / 1_000f64; //electrons / ns;

    let total_time = 5_000_000_000; //total time in ns;
    let xt = 256; // X spim;
    let yt = 256; //Y spim;
    let pdt: usize = 125; //pixel dwell time in ns;
    let fb = 10_000; //flyback in ns;
    let cur = 1; //electron current, in pA;
    let radius = xt/4; //The radius of the hole created, in pixels;
    let mut t: usize = 0; //The time of the electron events. They will be incremented during the program run;
    let mut counter: usize = 0; //Counter for the electron events;
    let frames = total_time / (xt * yt * pdt + yt * fb); //Number of frames that will be created;
    println!("Number of frames: {}.", frames);

    //Random variables;
    let col = Uniform::new(0.0, xt as f32); //Emplacing in the column. Something between 0 (left pixel) and xt (rightmost pixel);
    let pick = Uniform::new(0.0, 1.0); //Uniform probability from 0-1. This is used to decide if the electron will fall in the ZLP or in the simulated resonance;
    let zlp = Normal::new(128.0, 3.0).unwrap(); //Zero-loss peak. Centered at the pixel 128 and the std deviation is 3 pixels;
    let exc = Normal::new(190.0, 4.0).unwrap(); //The resonance;

    // This closure defines the electron emplacement. 90% of the electrons go to the ZLP.
    let alpha = |prob: f32| {
        if prob > 0.1 {
            zlp.sample(&mut thread_rng())
        } else {
            exc.sample(&mut thread_rng())
        }
    };

    let mut data: Vec<[u8; 16]> = Vec::new(); //Out TPX3 packets;
    let mut line_counter = 0; //The line counter;
    let mut electron: InversePacket; //The InversePacket struct. Defined in the packetlib;
    let mut line_start: InversePacket; //Same but for the TDC.

    for _frame in 0..frames { //Looping in the frames;
        for line in 0..yt { //Looping in the lines;
            line_counter += 1; //Line increment;
            line_start = InversePacket::new_inverse_tdc(t); //Define TDC packet;
            data.push(line_start.create_tdc_array(line_counter, TdcType::TdcOneFallingEdge)); //Push our line begin to our data;
            for _ in 0..((pdt * xt * cur) as f64 * ELE_PA) as usize { //Looping in the number of electrons;
                counter+=1; //Increment in the electron;
                let x = col.sample(&mut rand::thread_rng()) as usize; //X gives you the column in the hyperspectrum
                let disp = if circle(x, line, radius, (xt/2, yt/2)) { //disp gives you the dispersive value of the signal;
                    alpha(pick.sample(&mut rand::thread_rng())) //Inside circle => use the closure alpha;
                } else {
                    zlp.sample(&mut thread_rng()) //Outside circle => falls in the ZLP;
                };
                electron = InversePacket::new_inverse_electron(disp as usize, 128, t+x*pdt); //creating the electron packet;
                data.push(electron.create_electron_array()); //Pushing to our data;
            }
            line_counter += 1; //Line increment;
            line_start = InversePacket::new_inverse_tdc(t + pdt*xt); //This is actually line end;
            data.push(line_start.create_tdc_array(line_counter, TdcType::TdcOneRisingEdge)); //Push the end of a line to our data; 
            t = t + pdt * xt + fb;
        }
    }
    
    let final_vec = data.iter().map(|&x| x).flatten().collect::<Vec<_>>();
    let mut file = File::create("Data/raw000000.tpx3").unwrap();
    file.write_all(&final_vec).expect("Problem exporting data in dummy hyperspectral EELS");

    println!("Total time: {} and Electrons: {}. Ratio is (e/ns) {}", t, counter, counter as f64 / t as f64);
}

fn circle(x: usize, y: usize, radius: usize, c: (usize, usize)) -> bool {
    (x-c.0)*(x-c.0)+(y-c.1)*(y-c.1)<radius*radius
}

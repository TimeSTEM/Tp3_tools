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
    let radius = xt/4;
    let mut t: usize = 0;
    let mut counter: usize = 0;
    let frames = total_time / (xt * yt * pdt + yt * fb);
    println!("Number of frames: {}.", frames);

    //Random variables;
    let col = Uniform::new(0.0, xt as f32); //Emplacing in the column;
    let pick = Uniform::new(0.0, 1.0); //Put it or not in the second Gaussian;
    let zlp = Normal::new(128.0, 3.0).unwrap(); //Zero-loss peak;
    let exc = Normal::new(190.0, 4.0).unwrap(); //Closer-by resonance;

    let alpha = |prob: f32| {
        if prob > 0.1 {
            zlp.sample(&mut thread_rng())
        } else {
            exc.sample(&mut thread_rng())
        }
    };

    let mut data: Vec<[u8; 16]> = Vec::new();
    let mut line_counter = 0;
    let mut electron: InversePacket;
    let mut line_start: InversePacket;

    for _frame in 0..frames {
        for line in 0..yt {
            line_counter += 1;
            line_start = InversePacket::new_inverse_tdc(t);
            data.push(line_start.create_tdc_array(line_counter, TdcType::TdcOneFallingEdge));
            for _ in 0..((pdt * xt * cur) as f64 * ELE_PA) as usize {
                counter+=1;
                let x = col.sample(&mut rand::thread_rng()) as usize; //X gives you the column in the hyperspectrum
                let disp = if circle(x, line, radius, (xt/2, yt/2)) { //disp gives you the dispersive value of the signal
                    alpha(pick.sample(&mut rand::thread_rng()))
                } else {
                    zlp.sample(&mut thread_rng())
                };
                electron = InversePacket::new_inverse_electron(disp as usize, 128, t+x*pdt);
                data.push(electron.create_electron_array());
            }
            line_counter += 1;
            line_start = InversePacket::new_inverse_tdc(t + pdt*xt);
            data.push(line_start.create_tdc_array(line_counter, TdcType::TdcOneRisingEdge));
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

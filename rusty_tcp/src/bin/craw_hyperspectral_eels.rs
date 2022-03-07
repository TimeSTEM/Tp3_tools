use std::fs::File;
use std::io::prelude::*;
use timepix3::packetlib::*;
use timepix3::tdclib::TdcType;
use rand_distr::{Normal, Distribution};
use rand::distributions::Uniform;
use rand::{thread_rng};

fn main() {

    const ELE_PA: f64 = 6.25 / 1_000f64; //electrons / ns;

    let total_time = 60_000_000_000; //total time in ns;
    let xt = 1024; // X spim;
    let yt = 1024; //Y spim;
    let pdt: usize = 1_000; //pixel dwell time in ns;
    let fb = 10_000; //flyback in ns;
    let cur = 1; //electron current, in pA;
    let radius = xt/4;
    let mut t: usize = 0;
    let mut counter: usize = 0;
    let frames = total_time / (xt * yt * pdt);
    println!("{}", frames);

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





    /*

    let normal_01 = Normal::new(128.0, 3.0).unwrap();
    let normal_02 = Normal::new(638.0, 10.0).unwrap();
    let normal_03 = Normal::new(893.0, 7.0).unwrap();
    let uni = Uniform::new(0.0, 1.0);
    //let poisson = Poisson::new(1.0).unwrap(); //~30 pA or 0.2 e-/ns;
    let exp = Exp::new(0.02).unwrap();
    let decay_photon = Exp::new(0.4).unwrap();
    let decay_photon_02 = Exp::new(0.1).unwrap();
    let response = Exp::new(0.2).unwrap();

    let mut electron: InversePacket;
    let mut photon:Option<InversePacket> = None;
    let mut x: usize;
    let mut data: Vec<[u8; 16]> = Vec::new();

    let mut global_time = 0;
    for _ in 0..10_000_000 {
        let dt = exp.sample(&mut rand::thread_rng()) as usize;
        global_time += dt;
        let par = uni.sample(&mut thread_rng());

        if par < 0.1 {
            x = normal_01.sample(&mut rand::thread_rng()) as usize;
        } else if par < 0.75 {
            x = normal_02.sample(&mut rand::thread_rng()) as usize;
            if uni.sample(&mut thread_rng()) < 0.01 {
                let dt_photon = decay_photon.sample(&mut rand::thread_rng());
                photon = Some(InversePacket::new_inverse_tdc(global_time + dt_photon as usize));
            } else {
                photon = None;
            }
        } else {
            x = normal_03.sample(&mut rand::thread_rng()) as usize;
            if uni.sample(&mut thread_rng()) < 0.05 {
                let dt_photon = decay_photon_02.sample(&mut rand::thread_rng());
                let dt_response = response.sample(&mut rand::thread_rng());
                photon = Some(InversePacket::new_inverse_tdc(global_time + dt_photon as usize - dt_response as usize));
            } else {
                photon = None;
            }
        }
        electron = InversePacket::new_inverse_electron(x, 128, global_time);
        data.push(electron.create_electron_array());
        if let Some(ph) = photon {
            data.push(ph.create_tdc_array());
        }
        photon = None;
    }

    let final_vec = data.iter().map(|&x| x).flatten().collect::<Vec<_>>();
    let mut file = File::create("raw000000.tpx3").unwrap();
    file.write_all(&final_vec);
    */

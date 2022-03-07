use std::fs::File;
use std::io::prelude::*;
use timepix3::packetlib::*;
use rand_distr::{Poisson, Exp, Normal, Distribution};
use rand::distributions::Uniform;
use rand::{thread_rng};

fn main() {

    let x = 512; // X spim;
    let y = 512; //Y spim;
    let pdt = 1000 //pixel dwell time in ns;
    let frames = 100; //number of frames;

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




    
}

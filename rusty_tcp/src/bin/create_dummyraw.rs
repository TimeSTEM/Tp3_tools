use std::fs::File;
use std::io::prelude::*;
use timepix3::packetlib::*;
use rand_distr::{Poisson, Exp, Normal, Distribution};
use rand::distributions::Uniform;
use rand::{thread_rng};

fn main() {

    let normal_01 = Normal::new(128.0, 3.0).unwrap();
    let normal_02 = Normal::new(638.0, 10.0).unwrap();
    let uni = Uniform::new(0.0, 1.0);
    let poisson = Poisson::new(1.0).unwrap(); //~30 pA or 0.2 e-/ns;
    let exp = Exp::new(0.2).unwrap();

    let mut electron: InversePacket;
    let mut photon:Option<InversePacket> = None;
    let mut x: usize;
    let mut data: Vec<[u8; 16]> = Vec::new();

    let mut global_time = 0;
    for _ in 0..1_000 {
        let dt = exp.sample(&mut rand::thread_rng()) as usize;
        global_time += dt;
        if uni.sample(&mut thread_rng()) < 0.25 {
            x = normal_01.sample(&mut rand::thread_rng()) as usize;
        } else {
            x = normal_02.sample(&mut rand::thread_rng()) as usize;
            if uni.sample(&mut thread_rng()) > 0.5 {
                photon = Some(InversePacket::new_inverse_tdc(dt));
            }
            else {
                photon = None;
            }
        }
        electron = InversePacket::new_inverse_electron(x, 128, global_time);
        data.push(electron.create_electron_array());
        if let Some(ph) = photon {
            data.push(ph.create_electron_array());
        }
        photon = None;
    }

    let final_vec = data.iter().flatten().collect::<Vec<_>>();



    
}

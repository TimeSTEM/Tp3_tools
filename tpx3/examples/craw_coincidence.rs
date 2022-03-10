use std::fs::File;
use std::io::prelude::*;
use timepix3::packetlib::*;
use timepix3::tdclib::TdcType;
use rand_distr::{Exp, Normal, Distribution};
use rand::distributions::Uniform;
use rand::{thread_rng};

fn main() {

    let normal_01 = Normal::new(128.0, 3.0).unwrap();
    let normal_02 = Normal::new(638.0, 10.0).unwrap();
    let _normal_03 = Normal::new(893.0, 7.0).unwrap();
    let uni = Uniform::new(0.0, 1.0);
    let exp = Exp::new(0.02).unwrap();
    //1 ns lifetime;
    let decay_photon = Exp::new(1.0).unwrap();
    let _decay_photon_02 = Exp::new(0.2).unwrap();
    //3 ns response time
    let response = Exp::new(0.33).unwrap();

    let mut electron: InversePacket;
    let mut photon:Option<InversePacket> = None;
    let mut x: usize;
    let mut data: Vec<[u8; 16]> = Vec::new();
    let mut photon_counter = 0;

    let mut global_time = 0;
    for _ in 0..10_000_000 {
        let dt = exp.sample(&mut rand::thread_rng()) as usize;
        global_time += dt;
        let par = uni.sample(&mut thread_rng());

        //10% probability first gaussian
        if par < 0.1 {
            x = normal_01.sample(&mut rand::thread_rng()) as usize;
        //90% probability second gaussian
        } else {
            x = normal_02.sample(&mut rand::thread_rng()) as usize;
            //1% probability generate a photon in second gaussian
            if uni.sample(&mut thread_rng()) < 0.01 {
                photon_counter += 1;
                let dt_photon = decay_photon.sample(&mut rand::thread_rng());
                let dt_response_fall = response.sample(&mut rand::thread_rng());
                let dt_response_ris = response.sample(&mut rand::thread_rng());
                photon = Some(InversePacket::new_inverse_tdc(global_time + dt_photon as usize - dt_response_fall as usize + dt_response_ris as usize));
            } else {
                photon = None;
            }
        }
        electron = InversePacket::new_inverse_electron(x, 128, global_time);
        data.push(electron.create_electron_array());
        if let Some(ph) = photon {
            data.push(ph.create_tdc_array(photon_counter, TdcType::TdcTwoRisingEdge));
        }
        photon = None;
    }

    let final_vec = data.iter().map(|&x| x).flatten().collect::<Vec<_>>();
    let mut file = File::create("raw000000.tpx3").unwrap();
    file.write_all(&final_vec).expect("Could not write to file.");
}

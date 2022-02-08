use timepix3::packetlib::*;
use rand_distr::{Poisson, Exp, Normal, Distribution};
use rand::distributions::Uniform;
use rand::{thread_rng};

fn main() {
    //let inv = InversePacket::new_inverse_electron(10, 10, 10_000_000_000);
    //let tdc_inv = InversePacket::new_inverse_tdc(10_000_000_000);


    let npoints = 1_000;
    let normal_01 = Normal::new(128.0, 3.0).unwrap();
    let normal_02 = Normal::new(638.0, 10.0).unwrap();
    let uni = Uniform::new(0.0, 1.0);
    let poisson = Poisson::new(1).unwrap(); //~30 pA or 0.2 e-/ns;
    let exp = Exp::new(0.2).unwrap();


    let mut global_time = 0;
    for _ in 0..1_000 {
        let dt = exp.sample(&mut rand::thread_rng()) as usize;
        global_time += dt;
        if uni.sample(&mut thread_rng()) < 0.25 {
            //create_electron_gaussian_01
        } else {
            //create_electron_gaussian_02
            if uni.sample(&mut thread_rng()) > 0.5 {
                //create_photon;
            }
        }
        println!("{} and {}", dt, global_time);
    }


    //let _pack = inv.create_electron_event();
    
}

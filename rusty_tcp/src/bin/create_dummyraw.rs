use timepix3::packetlib::*;
use rand_distr::{Normal, Distribution};
use rand::distributions::Uniform;
use rand::{Rng, thread_rng};

fn main() {
    //let inv = InversePacket::new_inverse_electron(10, 10, 10_000_000_000);
    //let tdc_inv = InversePacket::new_inverse_tdc(10_000_000_000);


    let npoints = 1_000;
    let normal = Normal::new(100.0, 3.0).unwrap();
    let uni = Uniform::new(0.0, 1.0);

    for _ in 0..npoints {
        let v = normal.sample(&mut thread_rng());
        let pp = uni.sample(&mut thread_rng());
        //let pp2 = thread_rng().sample(uni);
        println!("{} and {}", v, pp);
    }

    //inv.test_func();
    //inv.tdc_test_func();
   
    
    



    //let _pack = inv.create_electron_event();
    
}

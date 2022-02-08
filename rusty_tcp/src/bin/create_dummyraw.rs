use timepix3::packetlib::*;

fn main() {
    let inv = InversePacket::new_inverse_electron(10, 10, 10_000_000_000);
    let tdc_inv = InversePacket::new_inverse_tdc(10_000_000_000);

    //inv.test_func();
    //inv.tdc_test_func();
    
    



    //let _pack = inv.create_electron_event();
    
}

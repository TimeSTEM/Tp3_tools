use timepix3::packetlib::*;

fn main() {
    let inv = InversePacket {
        x: 10,
        y: 10,
        time: 10,
        id: 14
    };

    inv.test_func();



    //let _pack = inv.create_electron_event();
    
}

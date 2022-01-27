use timepix3::inverselib::inverselib::*;

fn main() {
    let inv = InvPacketEELS {
        x: 10,
        y: 10,
        time: 10
    };

    let _pack = inv.create_electron_event();
    
}

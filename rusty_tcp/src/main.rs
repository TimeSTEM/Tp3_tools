use timepix3::auxiliar::Settings;
use timepix3::tdclib::{TdcControl, TdcType, PeriodicTdcRef, NonPeriodicTdcRef, NonPeriodicTdcRefMonitor, NoTdcRef};
use timepix3::{modes, message_board};

fn connect_and_loop() {

    let (my_settings, mut pack_sock, mut vec_ns_sock) = Settings::create_settings([192, 168, 199, 11], 8088).unwrap();

    let ns_sock = vec_ns_sock.pop().unwrap();

    match my_settings.mode {
        0 => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack_sock);
            let none_tdc = NoTdcRef::new(TdcType::NoTdc, &mut pack_sock);
            
            modes::build_spectrum(pack_sock, ns_sock, my_settings, frame_tdc, none_tdc);
        },
        1 => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack_sock);
            let laser_tdc = PeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack_sock);
     
            modes::build_spectrum(pack_sock, ns_sock, my_settings, frame_tdc, laser_tdc);
        },
        2 => {
            let spim_tdc = PeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut pack_sock);
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack_sock);

            modes::build_spim(pack_sock, ns_sock, my_settings, spim_tdc, np_tdc);
        },
        3 => {
            let spim_tdc = PeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut pack_sock);
            let laser_tdc = PeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack_sock);
            
            modes::build_spim(pack_sock, ns_sock, my_settings, spim_tdc, laser_tdc);
        },
        4 => {
            let spim_tdc = PeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut pack_sock);
            let pmt_tdc = NonPeriodicTdcRefMonitor::new(TdcType::TdcTwoFallingEdge, &mut pack_sock);
            
            modes::build_spim(pack_sock, ns_sock, my_settings, spim_tdc, pmt_tdc);
        },
        _ => panic!("Unknown mode received."),
    }
}

fn main() {
    loop {
        println!{"Waiting for a new client"};
        //message_board::start_message_board();
        connect_and_loop();
    }
}

use timepix3::auxiliar::Settings;
use timepix3::tdclib::{TdcControl, TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
use timepix3::speclib;
use timepix3::spimlib;
use timepix3::chronolib;

fn connect_and_loop() {
    
    let (my_settings, mut pack_sock, vec_ns_sock) = Settings::create_settings([192, 168, 199, 11], 8088).unwrap();

    match my_settings.mode {
        0 => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack_sock);
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack_sock);
            
            speclib::build_spectrum(pack_sock, vec_ns_sock, my_settings, frame_tdc, np_tdc);
        },
        1 => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack_sock);
            let laser_tdc = PeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack_sock);
     
            speclib::build_spectrum(pack_sock, vec_ns_sock, my_settings, frame_tdc, laser_tdc);
        },
        2 => {
            let spim_tdc = PeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut pack_sock);
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack_sock);

            spimlib::build_spim(pack_sock, vec_ns_sock, my_settings, spim_tdc, np_tdc);
        },
        3 => {
            println!("Mode 3 is empty. No action is taken.");
        },
        4 => {
            println!("Mode 4 is empty. No action is taken.");
        },
        6 => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack_sock);
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack_sock);
            
            chronolib::build_chrono(pack_sock, vec_ns_sock, my_settings, frame_tdc, np_tdc);
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

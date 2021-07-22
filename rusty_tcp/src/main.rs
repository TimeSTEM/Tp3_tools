use std::net::{TcpListener, SocketAddr};
use timepix3::auxiliar::Settings;
use timepix3::tdclib::{TdcType, PeriodicTdcRef, NonPeriodicTdcRef, NonPeriodicTdcRefMonitor, NoTdcRef};
use timepix3::{modes, message_board};

fn connect_and_loop() {

    let addrs = [
        SocketAddr::from(([192, 168, 199, 11], 8088)),
        SocketAddr::from(([127, 0, 0, 1], 8088)),
    ];

    let pack_listener = TcpListener::bind("127.0.0.1:8098").expect("Could not bind to TP3.");
    let ns_listener = TcpListener::bind(&addrs[..]).expect("Could not bind to NS.");
    println!("Packet Tcp socket connected at: {:?}", pack_listener);
    println!("Nionswift Tcp socket connected at: {:?}", ns_listener);

    let (mut pack_sock, packet_addr) = pack_listener.accept().expect("Could not connect to TP3.");
    println!("Localhost TP3 detected at {:?} and {:?}.", packet_addr, pack_sock);
    let (mut ns_sock, ns_addr) = ns_listener.accept().expect("Could not connect to Nionswift.");
    println!("Nionswift connected at {:?} and {:?}.", ns_addr, ns_sock);
    
    let my_settings = Settings::tcp_create_settings(&mut ns_sock);

    match my_settings.mode {
        0 => {
            let frame_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcOneRisingEdge, &mut pack_sock);
            let none_tdc = NoTdcRef::new_ref();
            
            modes::build_spectrum(pack_sock, ns_sock, my_settings, frame_tdc, none_tdc);
        },
        1 => {
            let frame_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcOneRisingEdge, &mut pack_sock);
            let laser_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcTwoFallingEdge, &mut pack_sock);
     
            modes::build_spectrum(pack_sock, ns_sock, my_settings, frame_tdc, laser_tdc);
        },
        2 => {
            let spim_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcOneFallingEdge, &mut pack_sock);
            let np_tdc = NonPeriodicTdcRef::new_ref(TdcType::TdcTwoFallingEdge);

            modes::build_spim(pack_sock, ns_sock, my_settings, spim_tdc, np_tdc);
        },
        3 => {
            let spim_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcOneFallingEdge, &mut pack_sock);
            let laser_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcTwoFallingEdge, &mut pack_sock);
            
            modes::build_spim(pack_sock, ns_sock, my_settings, spim_tdc, laser_tdc);
        },
        4 => {
            let spim_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcOneFallingEdge, &mut pack_sock);
            let pmt_tdc = NonPeriodicTdcRefMonitor::new_ref(TdcType::TdcTwoFallingEdge, 100);
            
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

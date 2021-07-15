use std::io::prelude::*;
use std::thread;
use std::sync::mpsc;
use std::net::{TcpListener, SocketAddr, UdpSocket};
use std::time::Instant;
use timepix3::auxiliar::Settings;
use timepix3::tdclib::{TdcType, PeriodicTdcRef, NonPeriodicTdcRef, NoTdcRef};
use timepix3::modes;

fn connect_and_loop() {

    let addrs = [
        SocketAddr::from(([192, 168, 199, 11], 8088)),
        SocketAddr::from(([127, 0, 0, 1], 8088)),
    ];

    let pack_listener = TcpListener::bind("127.0.0.1:8098").expect("Could not bind to TP3.");
    let ns_listener = TcpListener::bind(&addrs[..]).expect("Could not bind to NS.");
    let ns_udp = UdpSocket::bind(&addrs[..]).expect("Could not bind UDP socket.");
    println!("Packet Tcp socket connected at: {:?}", pack_listener);
    println!("Nionswift Tcp socket connected at: {:?}", ns_listener);
    println!("Udp socket connected at: {:?}", ns_udp);

    let (mut pack_sock, packet_addr) = pack_listener.accept().expect("Could not connect to TP3.");
    println!("Localhost TP3 detected at {:?} and {:?}.", packet_addr, pack_sock);
    let (mut ns_sock, ns_addr) = ns_listener.accept().expect("Could not connect to Nionswift.");
    println!("Nionswift connected at {:?} and {:?}.", ns_addr, ns_sock);
    
    let my_settings = Settings::tcp_create_settings(&mut ns_sock);
    let mut last_ci = 0u8;
    
    let mut buffer_pack_data = vec![0; 16384];

    let start = Instant::now();
    match my_settings.mode {
        0 => {
            let frame_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcOneRisingEdge, &mut pack_sock);
            let ref_tdc = NoTdcRef::new_ref();
            modes::build_spectrum(pack_sock, ns_sock, my_settings, frame_tdc, ref_tdc);
        },
        1 => {
            let frame_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcOneRisingEdge, &mut pack_sock);
            let laser_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcTwoFallingEdge, &mut pack_sock);
     
            modes::build_spectrum(pack_sock, ns_sock, my_settings, frame_tdc, laser_tdc);
        },
        2 => {
            let mut spim_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcOneFallingEdge, &mut pack_sock);
            let (tx, rx) = mpsc::channel();
            
            thread::spawn(move || {
                loop {
                    if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                        if size>0 {
                            let new_data = &buffer_pack_data[0..size];
                            let result = modes::build_spim_data(new_data, &mut last_ci, &my_settings, &mut spim_tdc);
                            tx.send(result).expect("Cannot send data over the thread channel.");
                        } else {println!("Received zero packages from TP3."); break;}
                    }
                }
            });

            loop {
                if let Ok(tl) = rx.recv() {
                    let result = modes::sort_and_append_to_index(tl);
                    if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break;}
                    //if let Err(_) = ns_udp.send_to(&result, "127.0.0.1:9088") {println!("Client disconnected on data (UDP)."); break;};
                } else {break;}
            }
        },
        3 => {
            let mut spim_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcOneFallingEdge, &mut pack_sock);
            let mut laser_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcTwoFallingEdge, &mut pack_sock);

            loop {
                if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                    if size>0 {
                        let new_data = &buffer_pack_data[0..size];
                        let result = modes::build_tr_spim_data(new_data, &mut last_ci, &my_settings, &mut spim_tdc, &mut laser_tdc);
                        if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break;}
                    } else {println!("Received zero packages from TP3."); break;}
                }
            }
        },
        4 => {
            let mut spim_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcOneFallingEdge, &mut pack_sock);
            let mut pmt_tdc = NonPeriodicTdcRef::new_ref(TdcType::TdcTwoFallingEdge);

            loop {
                if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                    if size>0 {
                        let new_data = &buffer_pack_data[0..size];
                        let result = modes::build_tdc_spim_data(new_data, &mut last_ci, &my_settings, &mut spim_tdc, &mut pmt_tdc);
                        if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break;}
                    } else {println!("Received zero packages from TP3."); break;}
                }
            }
            println!("Number of counts in PMT was: {}. Total elapsed time is {:?}.", pmt_tdc.counter, start.elapsed());
        },
        _ => panic!("Unknown mode received."),
    }
    //if let Err(_) = ns_sock.shutdown(Shutdown::Both) {println!("Served not succesfully shutdown.");}
}

fn main() {
    loop {
        println!{"Waiting for a new client"};
        connect_and_loop();
    }
}

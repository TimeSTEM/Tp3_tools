use std::io::prelude::*;
use std::thread;
use std::sync::mpsc;
use std::net::{Shutdown, TcpListener};
use std::time::Instant;
use timepix3::auxiliar::{RunningMode, BytesConfig, Settings};
use timepix3::tdclib::{TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
use timepix3::{modes, misc};

fn connect_and_loop(runmode: RunningMode) {

    let pack_listener = TcpListener::bind("127.0.0.1:8098").expect("Could not connect to packets.");
    let ns_listener = match runmode {
        RunningMode::DebugStem7482 => TcpListener::bind("127.0.0.1:8088").expect("Could not connect to NS in debug."),
        RunningMode::Tp3 => TcpListener::bind("192.168.199.11:8088").expect("Could not connect to NS using TP3."),
    };

    let (mut pack_sock, packet_addr) = pack_listener.accept().expect("Could not connect to TP3.");
    println!("Localhost TP3 detected at {:?}", packet_addr);
    
    let (mut ns_sock, ns_addr) = ns_listener.accept().expect("Could not connect to Nionswift.");
    println!("Nionswift connected at {:?}", ns_addr);
    //let (mut ns_sock02, ns_addr02) = ns_listener.accept().expect("Could not connect to Nionswift auxiliar client.");
    //println!("Nionswift aux. connected at {:?}", ns_addr02);
    
    let my_settings: Settings;
    {
        let mut cam_settings = [0 as u8; 28];
        match ns_sock.read(&mut cam_settings){
            Ok(size) => {
                println!("Received {} bytes from NS.", size);
                let my_config = BytesConfig{data: cam_settings};
                my_settings = my_config.create_settings();
            },
            Err(_) => panic!("Could not read cam initial settings."),
        }
        println!("Received settings is {:?}. Mode is {}.", cam_settings, my_settings.mode);
    }

    let mut last_ci = 0u8;
    
    let mut buffer_pack_data = match runmode {
        //RunningMode::DebugStem7482 => vec![0; 256],
        RunningMode::DebugStem7482 => vec![0; 16384],
        RunningMode::Tp3 => vec![0; 16384],
    };

    let start = Instant::now();
    match my_settings.mode {
        0 => {
            let mut frame_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcOneRisingEdge, &mut pack_sock);
            
            let mut data_array:Vec<u8> = vec![0; (255*!my_settings.bin as usize + 1)*my_settings.bytedepth*1024];
            data_array.push(10);
            
                loop {
                    if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                        if size>0 {
                            let new_data = &buffer_pack_data[0..size];
                            if modes::build_data(new_data, &mut data_array, &mut last_ci, &my_settings, &mut frame_tdc) {
                                let msg = misc::create_header(&my_settings, &frame_tdc);
                                if let Err(_) = ns_sock.write(&msg) {println!("Client disconnected on header."); break;}
                                if let Err(_) = ns_sock.write(&data_array) {println!("Client disconnected on data."); break;}
                                
                                if my_settings.cumul == false {
                                    data_array = vec![0; (255*!my_settings.bin as usize + 1)*my_settings.bytedepth*1024];
                                    data_array.push(10);
                                }

                               if frame_tdc.counter % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter);}
                            }
                        } else {println!("Received zero packages"); break;}
                    }
                }
        },
        1 => {
            let mut frame_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcOneRisingEdge, &mut pack_sock);
            let mut laser_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcTwoFallingEdge, &mut pack_sock);
     
            let mut data_array:Vec<u8> = if my_settings.bin {vec![0; my_settings.bytedepth*1024]} else {vec![0; 256*my_settings.bytedepth*1024]};
            data_array.push(10);
            
            'TRglobal: loop {
                match my_settings.cumul {
                    false => {
                        data_array = if my_settings.bin {vec![0; my_settings.bytedepth*1024]} else {vec![0; 256*my_settings.bytedepth*1024]};
                        data_array.push(10);
                    },
                    true => {},
                }

                loop {
                    if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                        if size>0 {
                            let new_data = &buffer_pack_data[0..size];
                            if modes::tr_build_data(new_data, &mut data_array, &mut last_ci, &my_settings, &mut frame_tdc, &mut laser_tdc) {
                                let msg = misc::create_header(&my_settings, &frame_tdc);
                                if let Err(_) = ns_sock.write(&msg) {println!("Client disconnected on header."); break 'TRglobal;}
                                if let Err(_) = ns_sock.write(&data_array) {println!("Client disconnected on data."); break 'TRglobal;}
                                break;
                            }
                        } else {println!("Received zero packages"); break 'TRglobal;}
                    }
                }
                if frame_tdc.counter % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter);}
            }
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
        5 => {
            let mut data_array:Vec<u8> = vec![0; my_settings.bytedepth*1025*my_settings.xspim_size*my_settings.yspim_size];
            let ping_data: Vec<u8> = vec![0; 4];
            let filepath = match runmode {
                RunningMode::DebugStem7482 => String::from("C:\\Users\\AUAD\\Desktop\\TimePix3\\Slice"),
                RunningMode::Tp3 => String::from("/home/asi/VGs/VG Lumière/TP3/SpimData/Slice"),
            };
            
            let mut spim_tdc = PeriodicTdcRef::tcp_new_ref(TdcType::TdcOneFallingEdge, &mut pack_sock);

            loop {
                if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                    if size>0 {
                        let new_data = &buffer_pack_data[0..size];
                        if let Ok(true) = modes::build_save_spim_data(new_data, &mut data_array, &mut last_ci, &my_settings, &mut spim_tdc, &filepath) {
                            data_array = vec![0; my_settings.bytedepth*1025*my_settings.xspim_size*my_settings.yspim_size];
                        };
                        if let Err(_) = ns_sock.write(&ping_data) {println!("Client disconnected on data."); break;}
                    } else {println!("Received zero packages from TP3."); break;}
                }
            }
        },
        _ => panic!("Unknown mode received."),
    }
    if let Err(_) = ns_sock.shutdown(Shutdown::Both) {println!("Served not succesfully shutdown.");}
}

fn main() {
    loop {
        let myrun = RunningMode::DebugStem7482;
        //let myrun = RunningMode::Tp3;
        println!{"Waiting for a new client"};
        connect_and_loop(myrun);
    }
}

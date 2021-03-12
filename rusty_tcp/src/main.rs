use std::io::prelude::*;
use std::net::{Shutdown, TcpListener};
use std::time::Instant;
use timepix3::auxiliar::{RunningMode, BytesConfig, Settings};
use timepix3::tdclib::{TdcType, PeriodicTdcRef};
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

    let start = Instant::now();
    let mut last_ci = 0u8;
    
    let mut buffer_pack_data: [u8; 16384] = [0; 16384];
    let mut tdc_vec:Vec<(f64, TdcType)> = Vec::new();
            
    loop {
        if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
            if size>0 {
                let new_data = &buffer_pack_data[0..size];
                misc::search_any_tdc(new_data, &mut tdc_vec, &mut last_ci);
                match my_settings.mode {
                    0 | 2 => {if TdcType::check_all_tdcs(&[5, 5, 0, 0], &tdc_vec)==true {break}},
                    1 | 3 => {if TdcType::check_all_tdcs(&[5, 5, 5, 5], &tdc_vec)==true {break}},
                    _ => panic!("Unknown mode."),
                }
            }
        }
    }
    println!("Related TDC have been found. Entering acquisition.");
            
    match my_settings.mode {
        0 => {
            let mut frame_tdc = PeriodicTdcRef::new_ref(&tdc_vec, TdcType::TdcOneRisingEdge);
            
            let mut data_array:Vec<u8> = if my_settings.bin {vec![0; my_settings.bytedepth*1024]} else {vec![0; 256*my_settings.bytedepth*1024]};
            data_array.push(10);
            
            'global: loop {
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
                            if modes::build_data(new_data, &mut data_array, &mut last_ci, &my_settings, &mut frame_tdc) {
                                let msg = misc::create_header(&my_settings, &frame_tdc);
                                if let Err(_) = ns_sock.write(&msg) {println!("Client disconnected on header."); break 'global;}
                                if let Err(_) = ns_sock.write(&data_array) {println!("Client disconnected on data."); break 'global;}
                                break;
                            }
                        } else {println!("Received zero packages"); break 'global;}
                    }
                }
                if frame_tdc.counter % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter);}
            }
        },
        1 => {
            let mut frame_tdc = PeriodicTdcRef::new_ref(&tdc_vec, TdcType::TdcOneRisingEdge);
            let mut laser_tdc = PeriodicTdcRef::new_ref(&tdc_vec, TdcType::TdcTwoFallingEdge);
     
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
            let mut spim_tdc = PeriodicTdcRef::new_ref(&tdc_vec, TdcType::TdcOneFallingEdge);

            loop {
                if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                    if size>0 {
                        let new_data = &buffer_pack_data[0..size];
                        let result = modes::build_spim_data(new_data, &mut last_ci, &my_settings, &mut spim_tdc);
                        if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break;}
                    } else {println!("Received zero packages from TP3."); break;}
                }
            }
        }
        3 => {
            let mut spim_tdc = PeriodicTdcRef::new_ref(&tdc_vec, TdcType::TdcOneFallingEdge);
            let mut laser_tdc = PeriodicTdcRef::new_ref(&tdc_vec, TdcType::TdcTwoFallingEdge);

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
        _ => panic!("Unknown mode received."),
    }
    if let Err(_) = ns_sock.shutdown(Shutdown::Both) {println!("Served not succesfully shutdown.");}
}

fn main() {
    loop {
        //let myrun = RunningMode::DebugStem7482;
        let myrun = RunningMode::Tp3;
        println!{"Waiting for a new client"};
        connect_and_loop(myrun);
    }
}

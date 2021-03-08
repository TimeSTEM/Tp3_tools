use std::io::prelude::*;
use std::net::{Shutdown, TcpListener};
use std::time::Instant;
use timepix3::auxiliar::{RunningMode, BytesConfig, Settings};
use timepix3::tdclib::{TdcType, PeriodicTdcRef};
use timepix3::{spectrum, spectral_image, misc};

fn connect_and_loop(runmode: RunningMode) {

    let mode:u8;

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
                mode = my_config.mode();
                my_settings = my_config.create_settings();
            },
            Err(_) => panic!("Could not read cam initial settings."),
        }
        println!("Received settings is {:?}.", cam_settings);
    }

    let start = Instant::now();
    let mut last_ci = 0u8;
    let mut frame_time:f64;
    let mut counter:usize;
    
    let mut buffer_pack_data: [u8; 16384] = [0; 16384];
    let mut tdc_vec:Vec<(f64, TdcType)> = Vec::new();
            
    loop {
        if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
            if size>0 {
                let new_data = &buffer_pack_data[0..size];
                misc::search_any_tdc(new_data, &mut tdc_vec, &mut last_ci);
                match mode {
                    0 | 1 => {if TdcType::check_all_tdcs(&[3, 3, 0, 0], &tdc_vec)==true {break}},
                    2 => {if TdcType::check_all_tdcs(&[5, 5, 5, 5], &tdc_vec)==true {break}},
                    _ => panic!("Unknown mode."),
                }
            }
        }
    }
    println!("Related TDC have been found. Entering acquisition.");
            
    match mode {
        0 => {
            let tdc = TdcType::TdcOneRisingEdge.associate_value();
            let mut frame_tdc = PeriodicTdcRef::new_ref(&tdc_vec, tdc);
            
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
                            if spectrum::build_data(new_data, &mut data_array, &mut last_ci, &my_settings, &mut frame_tdc) {
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
            let start_tdc_type = TdcType::TdcOneFallingEdge.associate_value();

            let spim_tdc = PeriodicTdcRef::new_ref(&tdc_vec, start_tdc_type);
            println!("Interval time (us) is {:?}. Measured dead time (us) is {:?}. Period (us) is {:?}", spim_tdc.low_time*1.0e6, spim_tdc.high_time*1.0e6, spim_tdc.period*1.0e6);
            
            frame_time = spim_tdc.frame_time;
            counter = spim_tdc.counter;
            
            let mut data_array:Vec<u8> = vec![0; my_settings.bytedepth*1024*my_settings.xspim_size*my_settings.yspim_size];

            'global_spim: loop {
                if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                    if size>0 {
                        let new_data = &buffer_pack_data[0..size];
                        let result = spectral_image::build_spim_data(new_data, &mut last_ci, &mut counter, &mut frame_time, &my_settings, &spim_tdc);
                        //let result = spectral_image::build_save_spim_data(new_data, &mut data_array, &mut last_ci, &mut counter, &mut frame_time, spim_size, yratio, interval, bytedepth, start_tdc_type);
                        if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break 'global_spim;}
                    } else {println!("Received zero packages from TP3."); break 'global_spim;}
                }
            }
        },
        2 => {
            let tdc_frame = TdcType::TdcOneRisingEdge.associate_value();
            let tdc_ref = TdcType::TdcTwoFallingEdge.associate_value();
            
            let mut frame_tdc = PeriodicTdcRef::new_ref(&tdc_vec, tdc_frame);
            let laser_tdc = PeriodicTdcRef::new_ref(&tdc_vec, tdc_ref);
            let mut ref_time: Vec<f64> = spectrum::tr_create_start_vectime2(5, laser_tdc.period, laser_tdc.frame_time);
            println!("Laser periodicity is: {}. First time vectors found were {:?}.", laser_tdc.period, ref_time);
     
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
                            if spectrum::tr_build_data(new_data, &mut data_array, &mut last_ci, &mut ref_time, &my_settings, &mut frame_tdc, &laser_tdc) {
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

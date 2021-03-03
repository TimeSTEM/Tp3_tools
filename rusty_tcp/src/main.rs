use std::io::prelude::*;
use std::net::{Shutdown, TcpListener};
use std::time::Instant;
use timepix3::auxiliar::{RunningMode, BytesConfig, Settings};
use timepix3::tdclib::TdcType;
use timepix3::{spectrum, spectral_image, misc};

fn connect_and_loop(runmode: RunningMode) {

    let bin: bool;
    let bytedepth:usize;
    let cumul: bool;
    let mode:u8;
    let spim_size:(usize, usize);
    let yratio: usize;

    let pack_listener = TcpListener::bind("127.0.0.1:8098").expect("Could not connect to packets.");
    let ns_listener = match runmode {
        RunningMode::DebugStem7482 => TcpListener::bind("127.0.0.1:8088").expect("Could not connect to NS in debug."),
        RunningMode::Tp3 => TcpListener::bind("192.168.199.11:8088").expect("Could not connect to NS using TP3."),
    };

    let (mut pack_sock, packet_addr) = pack_listener.accept().expect("Could not connect to TP3.");
    println!("Localhost TP3 detected at {:?}", packet_addr);
    let (mut ns_sock, ns_addr) = ns_listener.accept().expect("Could not connect to Nionswift.");
    println!("Nionswift connected at {:?}", ns_addr);

    println!("Waiting for config bytes. Instructions:
    28 bytes in total, structured as:
    [0, 1] => Bin (\\x00 for image and \\x01 for software binning);
    [1, 2] => Bytedepth (\\x00 for 8 bit, \\x01 for 16 bit and \\x02 for 32 bit);
    [2, 3] => Cumulation (\\x00 for Focus Mode and \\x01 for Cumul Mode);
    [3, 4] => Mode (\\x00 for Focus/Cumul, \\x01 for SPIM and \\x02 for TR);
    [4, 6] => X spim size. 16 bit depth, big endian mode;
    [6, 8] => Y spim size. 16 bit depth, big endian mode;
    [8, 10] => X scan size. 16 bit depth, big endian mode;
    [10, 12] => Y scan size. 16 bit depth, big endian mode;
    [12, 20] => Time delay (in ns). f64, double endian (>double in C);
    [20, 28] => Time width (in ns). f64, double endian (>double in C);
    ");
    let my_settings: Settings;
    let mut cam_settings = [0 as u8; 28];
    match ns_sock.read(&mut cam_settings){
        Ok(size) => {
            println!("Received {} bytes from NS.", size);
            let my_config = BytesConfig{data: cam_settings};
            bin = my_config.bin();
            bytedepth = my_config.bytedepth();
            cumul = my_config.cumul();
            mode = my_config.mode();
            spim_size = (my_config.xspim_size(), my_config.yspim_size());
            yratio = my_config.spimoverscany();
            my_settings = Settings{bin: my_config.bin(), bytedepth: my_config.bytedepth(), cumul: my_config.cumul(), xspim_size: my_config.xspim_size(), yspim_size: my_config.yspim_size(), xscan_size: my_config.xscan_size(), yscan_size: my_config.yscan_size(), time_delay: my_config.time_delay(), time_width: my_config.time_width(), spimoverscanx: my_config.spimoverscanx(), spimoverscany: my_config.spimoverscany()}
        },
        Err(_) => panic!("Could not read cam initial settings."),
    }
    println!("Received settings is {:?}.", cam_settings);
    
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
                    0 => {if TdcType::check_all_tdcs(&[1, 0, 0, 0], &tdc_vec)==true {break}},
                    1 => {if TdcType::check_all_tdcs(&[3, 3, 0, 0], &tdc_vec)==true {break}},
                    2 => {if TdcType::check_all_tdcs(&[5, 0, 0, 5], &tdc_vec)==true {break}},
                    _ => panic!("Unknown mode."),
                }
            }
        }
    }
    println!("Related TDC have been found. Entering acquisition.");
   
    match mode {
        0 => {
            let tdc_type = TdcType::TdcOneRisingEdge.associate_value();
            
            frame_time = TdcType::last_time_from_tdc(&tdc_vec, tdc_type);
            counter = TdcType::howmany_from_tdc(&tdc_vec, tdc_type);
            
            let mut data_array:Vec<u8> = if bin {vec![0; bytedepth*1024]} else {vec![0; 256*bytedepth*1024]};
            data_array.push(10);
            
            'global: loop {
                match cumul {
                    false => {
                        data_array = if bin {vec![0; bytedepth*1024]} else {vec![0; 256*bytedepth*1024]};
                        data_array.push(10);
                    },
                    true => {},
                }

                loop {
                    if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                        if size>0 {
                            let new_data = &buffer_pack_data[0..size];
                            if spectrum::build_data(new_data, &mut data_array, &mut last_ci, &mut counter, &mut frame_time, &my_settings, tdc_type) {
                                let msg = match bin {
                                    true => misc::create_header(frame_time, counter, bytedepth*1024, bytedepth<<3, 1024, 1),
                                    false => misc::create_header(frame_time, counter, bytedepth*256*1024, bytedepth<<3, 1024, 256),
                                };
                                if let Err(_) = ns_sock.write(&msg) {println!("Client disconnected on header."); break 'global;}
                                if let Err(_) = ns_sock.write(&data_array) {println!("Client disconnected on data."); break 'global;}
                                break;
                            }
                        } else {println!("Received zero packages"); break 'global;}
                    }
                }
                if counter % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, counter);}
            }
        },
        1 => {
            let start_tdc_type = TdcType::TdcOneFallingEdge.associate_value();
            let stop_tdc_type = TdcType::TdcOneRisingEdge.associate_value();

            let dead_time:f64;
            let interval:f64;
            let period:f64;
            {
                let start_line = TdcType::vec_from_tdc(&tdc_vec, start_tdc_type);
                let end_line = TdcType::vec_from_tdc(&tdc_vec, stop_tdc_type);
                dead_time = spectral_image::find_deadtime(&start_line, &end_line);
                interval = spectral_image::find_interval(&start_line, dead_time);
                period = spectral_image::find_period(&start_line);
            }
            println!("Interval time (us) is {:?}. Measured dead time (us) is {:?}", interval*1.0e6, dead_time*1.0e6);
            
            frame_time = TdcType::last_time_from_tdc(&tdc_vec, start_tdc_type);
            counter = TdcType::howmany_from_tdc(&tdc_vec, start_tdc_type);
            
            let mut data_array:Vec<u8> = vec![0; bytedepth*1024*spim_size.0*spim_size.1];

            'global_spim: loop {
                if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                    if size>0 {
                        let new_data = &buffer_pack_data[0..size];
                        let result = spectral_image::build_spim_data(new_data, &mut last_ci, &mut counter, &mut frame_time, spim_size, yratio, interval, period, start_tdc_type);
                        //let result = spectral_image::build_save_spim_data(new_data, &mut data_array, &mut last_ci, &mut counter, &mut frame_time, spim_size, yratio, interval, bytedepth, start_tdc_type);
                        if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break 'global_spim;}
                        //if let Err(_) = nsaux_sock.write(&[1, 2, 3, 4, 5]) {println!("Client disconnected on data."); break 'global_spim;}
                    } else {println!("Received zero packages from TP3."); break 'global_spim;}
                }
            }
        },
        2 => {
            let tdc_frame = TdcType::TdcOneRisingEdge.associate_value();
            let tdc_ref = TdcType::TdcTwoFallingEdge.associate_value();
            
            let all_ref_time = TdcType::vec_from_tdc(&tdc_vec, tdc_ref);
            let period = spectrum::tr_find_period(&all_ref_time);
            let mut ref_time: Vec<f64> = spectrum::tr_create_start_vectime(all_ref_time);
            
            println!("Laser periodicity is: {}. First time vectors found were {:?}.", period, ref_time);
     
            frame_time = TdcType::last_time_from_tdc(&tdc_vec, tdc_frame);
            counter = TdcType::howmany_from_tdc(&tdc_vec, tdc_frame);
    
            let mut data_array:Vec<u8> = if bin {vec![0; bytedepth*1024]} else {vec![0; 256*bytedepth*1024]};
            data_array.push(10);
            
            'TRglobal: loop {
                match cumul {
                    false => {
                        data_array = if bin {vec![0; bytedepth*1024]} else {vec![0; 256*bytedepth*1024]};
                        data_array.push(10);
                    },
                    true => {},
                }

                loop {
                    if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                        if size>0 {
                            let new_data = &buffer_pack_data[0..size];
                            if spectrum::tr_build_data(new_data, &mut data_array, &mut last_ci, &mut counter, &mut frame_time, &mut ref_time, &my_settings, tdc_frame, tdc_ref, period) {
                                let msg = match bin {
                                    true => misc::create_header(frame_time, counter, bytedepth*1024, bytedepth<<3, 1024, 1),
                                    false => misc::create_header(frame_time, counter, bytedepth*256*1024, bytedepth<<3, 1024, 256),
                                };
                                if let Err(_) = ns_sock.write(&msg) {println!("Client disconnected on header."); break 'TRglobal;}
                                if let Err(_) = ns_sock.write(&data_array) {println!("Client disconnected on data."); break 'TRglobal;}
                                break;
                            }
                        } else {println!("Received zero packages"); break 'TRglobal;}
                    }
                }
                if counter % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, counter);}
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

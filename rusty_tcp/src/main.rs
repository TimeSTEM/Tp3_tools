use std::io::prelude::*;
use std::net::{Shutdown, TcpListener};
use std::time::{Instant, Duration};
use timepix3::{RunningMode, Config, TdcType, Packet, spectral_image, tr_spectrum};

fn build_data(data: &[u8], final_data: &mut [u8], last_ci: &mut u8, frame_time: &mut f64, bin: bool, bytedepth: usize, kind: u8) -> usize {

    let mut packet_chunks = data.chunks_exact(8);
    let mut tdc_counter = 0;

    while let Some(x) = packet_chunks.next() {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
            _ => {
                let packet = Packet { chip_index: *last_ci, data: x};
                
                match packet.id() {
                    11 => {
                        let array_pos = match bin {
                            false => packet.x() + 1024*packet.y(),
                            true => packet.x()
                        };
                        Packet::append_to_array(final_data, array_pos, bytedepth);
                    },
                    6 if packet.tdc_type() == kind => {
                        *frame_time = packet.tdc_time();
                        tdc_counter+=1;
                    },
                    7 => {continue;},
                    4 => {continue;},
                    _ => {},
                };
            },
        };
    };
    tdc_counter
}

fn build_spim_data(data: &[u8], last_ci: &mut u8, counter: &mut usize, sltdc: &mut f64, spim: (usize, usize), yratio: usize, interval: f64, tdc_kind: u8) -> Vec<u8> {
    
    let mut packet_chunks = data.chunks_exact(8);
    let mut index_data:Vec<u8> = Vec::new();

    while let Some(x) = packet_chunks.next() {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
            _ => {
                let packet = Packet { chip_index: *last_ci, data: x};
                
                match packet.id() {
                    11 => {
                        let line = (*counter / yratio) % spim.1;
                        let ele_time = packet.electron_time() - 0.000007;
                        if spectral_image::check_if_in(ele_time, sltdc, interval) {
                            let xpos = (spim.0 as f64 * ((ele_time - *sltdc)/interval)) as usize;
                            let array_pos = packet.x() + 1024*spim.0*line + 1024*xpos;
                            Packet::append_to_index_array(&mut index_data, array_pos);
                        }
                    },
                    6 if packet.tdc_type() == tdc_kind => {
                        *sltdc = packet.tdc_time_norm();
                        *counter+=1;
                    },
                    _ => {},
                };
            },
        };
    };
    index_data
}

fn build_time_data(data: &[u8], final_data: &mut [u8], last_ci: &mut u8, frame_time: &mut f64, ref_time: &mut Vec<f64>, bin: bool, bytedepth: usize, frame_tdc: u8, ref_tdc: u8, tdelay: f64, twidth: f64) -> usize {

    let mut packet_chunks = data.chunks_exact(8);
    let mut tdc_counter = 0;

    while let Some(x) = packet_chunks.next() {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
            _ => {
                let packet = Packet { chip_index: *last_ci, data: x};
                
                match packet.id() {
                    11 => {
                        let ele_time = packet.electron_time();
                        
                        if tr_spectrum::check_if_in(ref_time, ele_time, tdelay, twidth) {
                            let array_pos = match bin {
                                false => packet.x() + 1024*packet.y(),
                                true => packet.x()
                            };
                            Packet::append_to_array(final_data, array_pos, bytedepth);
                        }
                    },
                    6 if packet.tdc_type() == frame_tdc => {
                        *frame_time = packet.tdc_time();
                        tdc_counter+=1;
                    },
                    6 if packet.tdc_type() == ref_tdc => {
                        ref_time.remove(0);
                        ref_time.push(packet.tdc_time_norm());
                    },
                    7 => {continue;},
                    4 => {continue;},
                    _ => {},
                };
            },
        };
    };
    tdc_counter
}

fn search_any_tdc(data: &[u8], tdc_vec: &mut Vec<(f64, TdcType)>, last_ci: &mut u8) {
    
    let file_data = data;
    let mut packet_chunks = file_data.chunks_exact(8);

    while let Some(x) = packet_chunks.next() {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
            _ => {
                let packet = Packet { chip_index: *last_ci, data: x};
                
                match packet.id() {
                    11 => {continue;},
                    6 => {
                        let time = packet.tdc_time_norm();
                        let tdc = packet.tdc_type_as_enum().unwrap();
                        tdc_vec.push( (time, tdc) );
                    },
                    7 => {continue;},
                    4 => {continue;},
                    _ => {},
                };
            },
        };
    };
}

                    
fn create_header(time: f64, frame: usize, data_size: usize, bitdepth: usize, width: usize, height: usize) -> Vec<u8> {
    let mut msg: String = String::from("{\"timeAtFrame\":");
    msg.push_str(&(time.to_string()));
    msg.push_str(",\"frameNumber\":");
    msg.push_str(&(frame.to_string()));
    msg.push_str(",\"measurementID:\"Null\",\"dataSize\":");
    msg.push_str(&(data_size.to_string()));
    msg.push_str(",\"bitDepth\":");
    msg.push_str(&(bitdepth.to_string()));
    msg.push_str(",\"width\":");
    msg.push_str(&(width.to_string()));
    msg.push_str(",\"height\":");
    msg.push_str(&(height.to_string()));
    msg.push_str("}\n");
    let s: Vec<u8> = msg.into_bytes();
    s
}

fn connect_and_loop(runmode: RunningMode) {

    let bin: bool;
    let bytedepth:usize;
    let cumul: bool;
    let mode:u8;
    let spim_size:(usize, usize);
    let yratio: usize;
    let tdelay: f64;
    let twidth: f64;

    let pack_listener = TcpListener::bind("127.0.0.1:8098").expect("Could not connect to packets.");
    let ns_listener = match runmode {
        RunningMode::DebugStem7482 => TcpListener::bind("127.0.0.1:8088").expect("Could not connect to NS in debug."),
        RunningMode::Tp3 => TcpListener::bind("192.168.199.11:8088").expect("Could not connect to NS using TP3."),
    };

    let (mut pack_sock, packet_addr) = pack_listener.accept().expect("Could not connect to TP3.");
    println!("Localhost TP3 detected at {:?}", packet_addr);
    let (mut ns_sock, ns_addr) = ns_listener.accept().expect("Could not connect to Nionswift.");
    println!("Nionswift connected at {:?}", ns_addr);
    let (mut nsaux_sock, nsaux_addr) = ns_listener.accept().expect("Could not connect to Nionswift aux.");
    println!("Nionswift [aux] connected at {:?}", nsaux_addr);

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
    let mut cam_settings = [0 as u8; 28];
    match ns_sock.read(&mut cam_settings){
        Ok(size) => {
            println!("Received {} bytes from NS.", size);
            let my_config = Config{data: cam_settings};
            bin = my_config.bin();
            bytedepth = my_config.bytedepth();
            cumul = my_config.cumul();
            mode = my_config.mode();
            spim_size = (my_config.xspim_size(), my_config.yspim_size());
            yratio = my_config.spimoverscany();
            tdelay = my_config.time_delay();
            twidth = my_config.time_width();
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
                println!("{}", size);
                let new_data = &buffer_pack_data[0..size];
                search_any_tdc(new_data, &mut tdc_vec, &mut last_ci);
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
                            let result = build_data(new_data, &mut data_array, &mut last_ci, &mut frame_time, bin, bytedepth, tdc_type);
                            counter += result;
                            
                            if result>0 {
                                let msg = match bin {
                                    true => create_header(frame_time, counter, bytedepth*1024, bytedepth<<3, 1024, 1),
                                    false => create_header(frame_time, counter, bytedepth*256*1024, bytedepth<<3, 1024, 256),
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
            ns_sock.set_write_timeout(Some(Duration::from_millis(10000))).expect("Failed to set write timeout to SPIM.");
            let mut byte_counter = 0;
            let start_tdc_type = TdcType::TdcOneFallingEdge.associate_value();
            let stop_tdc_type = TdcType::TdcOneRisingEdge.associate_value();

            let dead_time:f64;
            let interval:f64;
            {
                let start_line = TdcType::vec_from_tdc(&tdc_vec, start_tdc_type);
                let end_line = TdcType::vec_from_tdc(&tdc_vec, stop_tdc_type);
                dead_time = spectral_image::find_deadtime(&start_line, &end_line);
                interval = spectral_image::find_interval(&start_line, dead_time);
            }
            println!("Interval time (us) is {:?}. Measured dead time (us) is {:?}", interval*1.0e6, dead_time*1.0e6);
            
            frame_time = TdcType::last_time_from_tdc(&tdc_vec, start_tdc_type);
            counter = TdcType::howmany_from_tdc(&tdc_vec, start_tdc_type);

            'global_spim: loop {
                if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                    if size>0 {
                        let new_data = &buffer_pack_data[0..size];
                        let result = build_spim_data(new_data, &mut last_ci, &mut counter, &mut frame_time, spim_size, yratio, interval, start_tdc_type);
                        //if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break 'global_spim;}
                        match ns_sock.write(&result) {
                            Ok(size) => {
                                byte_counter+=size;
                                println!("{} and {} and {}", size, byte_counter, result.len());
                            },
                            Err(e) => {
                                println!("Client disconnected on data. {}", e); break 'global_spim;},
                        }
                        //if let Err(_) = nsaux_sock.write(&[1, 2, 3, 4, 5]) {println!("Client disconnected on data."); break 'global_spim;}
                    } else {println!("Received zero packages"); break 'global_spim;}
                }
            }
        },
        2 => {
            let tdc_frame = TdcType::TdcOneRisingEdge.associate_value();
            let tdc_ref = TdcType::TdcTwoFallingEdge.associate_value();
            
            let all_ref_time = TdcType::vec_from_tdc(&tdc_vec, tdc_ref);
            let mut ref_time: Vec<f64> = tr_spectrum::create_start_vectime(all_ref_time);
     
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
                            let result = build_time_data(new_data, &mut data_array, &mut last_ci, &mut frame_time, &mut ref_time, bin, bytedepth, tdc_frame, tdc_ref, tdelay, twidth);
                            counter += result;
                            
                            if result>0 {
                                let msg = match bin {
                                    true => create_header(frame_time, counter, bytedepth*1024, bytedepth<<3, 1024, 1),
                                    false => create_header(frame_time, counter, bytedepth*256*1024, bytedepth<<3, 1024, 256),
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
        _ => println!("Unknown mode received."),
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

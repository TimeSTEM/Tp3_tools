use std::io::prelude::*;
use std::net::{Shutdown, TcpListener};
use std::time::Instant;
use timepix3::{RunningMode, Config, TdcType, Packet};

fn build_data(data: &[u8], final_data: &mut [u8], last_ci: &mut u8, frame_time: &mut f64, bin: bool, bytedepth: usize, kind: u8) -> usize {

    let mut packet_chunks = data.chunks_exact(8);
    let mut tdc_counter = 0;

    loop {
        match packet_chunks.next() {
            None => break,
            Some(&[84, 80, 88, 51, nci, _, _, _]) => *last_ci = nci,
            Some(x) => {
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

    loop {
        match packet_chunks.next() {
            None => break,
            Some(&[84, 80, 88, 51, nci, _, _, _]) => *last_ci = nci,
            Some(x) => {
                let packet = Packet { chip_index: *last_ci, data: x};
                
                match packet.id() {
                    11 => {
                        let line = (*counter / yratio) % spim.1;

                        let ele_time = packet.electron_time() - 0.000007;
                        let xpos = (spim.0 as f64 * ((ele_time - *sltdc)/interval)) as usize;
                        let array_pos = packet.x() + 1024*spim.0*line + 1024*xpos;
                        if ele_time>*sltdc && ele_time<(*sltdc + interval){
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

fn build_time_data(data: &[u8], final_data: &mut [u8], last_ci: &mut u8, frame_time: &mut f64, bin: bool, bytedepth: usize, frame_tdc: u8, ref_tdc: u8) -> usize {

    let mut packet_chunks = data.chunks_exact(8);
    let mut tdc_counter = 0;

    loop {
        match packet_chunks.next() {
            None => break,
            Some(&[84, 80, 88, 51, nci, _, _, _]) => *last_ci = nci,
            Some(x) => {
                let packet = Packet { chip_index: *last_ci, data: x};
                
                match packet.id() {
                    11 => {
                        let array_pos = match bin {
                            false => packet.x() + 1024*packet.y(),
                            true => packet.x()
                        };
                        Packet::append_to_array(final_data, array_pos, bytedepth);
                    },
                    6 if packet.tdc_type() == frame_tdc => {
                        *frame_time = packet.tdc_time();
                        tdc_counter+=1;
                    },
                    6 if packet.tdc_type() == ref_tdc => {
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

    loop {
        match packet_chunks.next() {
            None => break,
            Some(&[84, 80, 88, 51, nci, _, _, _]) => *last_ci = nci,
            Some(x) => {
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
    let tdelay: f32;

    let pack_listener = TcpListener::bind("127.0.0.1:8098").expect("Could not connect to packets.");
    let ns_listener = match runmode {
        RunningMode::DebugStem7482 => TcpListener::bind("127.0.0.1:8088").expect("Could not connect to NS in debug."),
        RunningMode::Tp3 => TcpListener::bind("192.168.199.11:8088").expect("Could not connect to NS using TP3."),
    };

    let (mut pack_sock, packet_addr) = pack_listener.accept().expect("Could not connect to TP3.");
    println!("Localhost TP3 detected at {:?}", packet_addr);
    let (mut ns_sock, ns_addr) = ns_listener.accept().expect("Could not connect to Nionswift.");
    println!("Nionswift connected at {:?}", ns_addr);

    let mut cam_settings = [0 as u8; 16];
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
        },
        Err(_) => panic!("Could not read cam initial settings."),
    }
    println!("Received settings is {:?}.", cam_settings);
    
    let start = Instant::now();
    let mut counter = 0usize;
    let mut last_ci = 0u8;
    let mut frame_time = 0.0f64;
    let mut buffer_pack_data: [u8; 8192] = [0; 8192];
   
    match mode {
        0 => {
            let tdc_type = TdcType::TdcOneRisingEdge.associate_value();
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
            let mut tdc_vec:Vec<(f64, TdcType)> = Vec::new();
            let start_tdc_type = TdcType::TdcOneFallingEdge.associate_value();
            let stop_tdc_type = TdcType::TdcOneRisingEdge.associate_value();
            let ntdc = 3;

            loop {
                if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                    if size>0 {
                        let new_data = &buffer_pack_data[0..size];
                        search_any_tdc(new_data, &mut tdc_vec, &mut last_ci);
                        if tdc_vec.iter().filter(|(_time, tdct)| tdct.associate_value()==start_tdc_type).count() >= ntdc {
                            break;
                        } 
                    }
                }
            };

            let start_array: Vec<_> = tdc_vec.iter()
                .filter(|(_time, tdct)| tdct.associate_value()==start_tdc_type)
                .map(|(time, _tdct)| time)
                .collect();
            
            let end_array: Vec<_> = tdc_vec.iter()
                .filter(|(_time, tdct)| tdct.associate_value()==stop_tdc_type)
                .map(|(time, _tdct)| time)
                .collect();
            
            frame_time = *tdc_vec.iter()
                .filter(|(_time, tdct)| tdct.associate_value()==start_tdc_type)
                .map(|(time, _tdct)| time)
                .last().unwrap();

            counter = ntdc;

            let dead_time:f64 = if (start_array[1] - end_array[1])>0.0 {start_array[1] - end_array[1]} else {start_array[2] - end_array[1]};
            let interval:f64 = (start_array[2] - start_array[1]) - dead_time;
            println!("Interval time (us) is {:?}. Measured dead time (us) is {:?}", interval*1.0e6, dead_time*1.0e6);

            'global_spim: loop {
                if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                    if size>0 {
                        let new_data = &buffer_pack_data[0..size];
                        let result = build_spim_data(new_data, &mut last_ci, &mut counter, &mut frame_time, spim_size, yratio, interval, start_tdc_type);
                        if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break 'global_spim;}
                    } else {println!("Received zero packages"); break 'global_spim;}
                }
            }
        },
        2 => {
            let tdc_trig = TdcType::TdcOneRisingEdge.associate_value();
            let tdc_ref = TdcType::TdcTwoFallingEdge.associate_value();
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
                            let result = build_time_data(new_data, &mut data_array, &mut last_ci, &mut frame_time, bin, bytedepth, tdc_trig, tdc_ref);
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
    println!("Number of loops were: {}.", counter);
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

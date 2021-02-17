use std::io::prelude::*;
use std::net::{Shutdown, TcpListener};
use std::time::{Duration, Instant};

enum RunningMode {
    DebugStem7482,
    Tp3,
}

struct Config {
    data: [u8; 16],
}

impl Config {
    fn bin(&self) -> bool {
        match self.data[0] {
            0 => {
                println!("Bin is False.");
                false
            },
            1 => {
                println!("Bin is True.");
                true
            },
            _ => panic!("Binning choice must be 0 | 1."),
        }
    }

    fn bytedepth(&self) -> usize {
        match self.data[1] {
            0 => {
                println!("Bitdepth is 8.");
                1
            },
            1 => {
                println!("Bitdepth is 16.");
                2
            },
            2 => {
                println!("Bitdepth is 32.");
                4
            },
            _ => panic!("Bytedepth must be  1 | 2 | 4."),
        }
    }

    fn cumul(&self) -> bool {
        match self.data[2] {
            0 => {
                println!("Cumulation mode is OFF.");
                false
            },
            1 => {
                println!("Cumulation mode is ON.");
                true
            },
            _ => panic!("Cumulation must be 0 | 1."),
        }
    }

    fn is_spim(&self) -> bool {
        match self.data[3] {
            0 => {
                println!("Spim is OFF.");
                false
            },
            1 => {
                println!("Spim is ON.");
                true
                    },
            _ => panic!("Spim config must be 0 | 1."),
        }
    }

    fn xspim_size(&self) -> usize {
        println!("X spim size is {}", self.data[4]);
        self.data[4] as usize
    }
    
    fn yspim_size(&self) -> usize {
        println!("Y spim size is {}", self.data[5]);
        self.data[5] as usize
    }

}

struct Packet {
    chip_index: u8,
    i08: u8,
    i09: u8,
    i10: u8,
    i11: u8,
    i12: u8,
    i13: u8,
    i14: u8,
    i15: u8,
}

impl Packet {
    fn x(&self) -> usize {
        let temp = ((((self.i14 & 224))>>4 | ((self.i15 & 15))<<4) | (((self.i13 & 112)>>4)>>2)) as usize;
        match self.chip_index {
            0 => 255 - temp,
            1 => 255 * 4 - temp,
            2 => 255 * 3 - temp,
            3 => 255 * 2 - temp,
            _ => temp,
        }
    }
    
    fn x_unmod(&self) -> usize {
        !((((self.i14 & 224))>>4 | ((self.i15 & 15))<<4) | (((self.i13 & 112)>>4)>>2)) as usize
    }
    
    fn y(&self) -> usize {
        (   ( ((self.i13 & 128))>>5 | ((self.i14 & 31))<<3 ) | ( (((self.i13 & 112)>>4)) & 3 )   ) as usize
    }

    fn id(&self) -> u8 {
        (self.i15 & 240) >> 4
    }

    fn spidr(&self) -> u16 {
        (self.i08 as u16) | (self.i09 as u16)<<8
    }

    fn ftoa(&self) -> u8 {
        self.i10 & 15
    }

    fn tot(&self) -> u8 {
        (self.i10 & 240)>>4 | (self.i11 & 63)<<4
    }

    fn toa(&self) -> u16 {
        ((self.i11 & 192) as u16)>>6 | (self.i12 as u16)<<2 | ((self.i13 & 15) as u16)<<10
    }

    fn tdc_coarse(&self) -> u64 {
        ((self.i09 & 254) as u64)>>1 | ((self.i10) as u64)<<7 | ((self.i11) as u64)<<15 | ((self.i12) as u64)<<23 | ((self.i13 & 15) as u64)<<31
    }
    
    fn tdc_fine(&self) -> u8 {
        (self.i08 & 224)>>5 | (self.i09 & 1)<<3
    }

    fn tdc_counter(&self) -> u16 {
        ((self.i13 & 240) as u16) >> 4 | (self.i14 as u16) << 4
    }

    fn elec_time(spidr: u16, toa: u16, ftoa: u8) -> f64 {
        let ctoa = (toa<<4) | ((!ftoa as u16) & 15);
        ((spidr as f64) * 25.0 * 16384.0 + (ctoa as f64) * 25.0 / 16.0)/1e9
    }

    fn tdc_time(coarse: u64, fine: u8) -> f64 {
        (coarse as f64) * (1.0/320e6) + (fine as f64) * 260e-12
    }

    fn append_to_array(data: &mut [u8], index:usize, bytedepth: usize) -> bool{
        match bytedepth {
            4 => {
                data[index+3] = data[index+3].wrapping_add(1);
                if data[index+3]==0 {
                    data[index+2] = data[index+2].wrapping_add(1);
                    if data[index+2]==0 {
                        data[index+1] = data[index+1].wrapping_add(1);
                        if data[index+1]==0 {
                            data[index] = data[index].wrapping_add(1);
                        };
                    };
                };
                false
            },
            2 => {
                data[index+1] = data[index+1].wrapping_add(1);
                if data[index+1]==0 {
                    data[index] = data[index].wrapping_add(1);
                    true
                } else {
                    false
                }
            },
            1 => {
                data[index] = data[index].wrapping_add(1);
                if data[index]==0 {
                    true
                } else {
                    false
                }
            },
            _ => {panic!("Bytedepth must be 1 | 2 | 4.");},
        }
    }
}


fn build_data(data: &[u8], bin: bool, final_data: &mut [u8], last_ci: u8, bytedepth: usize) -> (u8, bool, f64) {
    
    let bin = bin;
    let mut packet_chunks = data.chunks_exact(8);
    let mut has_tdc: bool = false;
    let mut time = 0.0f64;

    let mut ci: u8 = last_ci;
    loop {
        match packet_chunks.next() {
            None => break,
            Some(&[84, 80, 88, 51, nci, _, _, _]) => ci = nci,
            Some(x) => {
                let packet = Packet {
                    chip_index: ci,
                    i08: x[0],
                    i09: x[1],
                    i10: x[2],
                    i11: x[3],
                    i12: x[4],
                    i13: x[5],
                    i14: x[6],
                    i15: x[7],
                };
                
                match packet.id() {
                    11 => {
                        let array_pos = match bin {
                            false => bytedepth*packet.x() + bytedepth*1024*packet.y(),
                            true => bytedepth*packet.x()
                        };
                        Packet::append_to_array(final_data, array_pos, bytedepth);
                    },
                    6 => {
                        time = Packet::tdc_time(packet.tdc_coarse(), packet.tdc_fine());
                        if has_tdc == true {println!("already tdc")};
                        has_tdc = true;
                    },
                    7 => {continue;},
                    4 => {continue;},
                    _ => {},
                };
            },
        };
    };
    (ci, has_tdc, time)
}

fn build_spim_data(data: &[u8], final_data: &mut [u8], last_ci: u8, bytedepth: usize, counter: usize, last_tdc: f64, xspim: usize, yspim: usize, interval: f64) -> (u8, bool, f64) {
    
    let line = counter % yspim;
    let max_value = bytedepth*xspim*yspim*1024;
    let mut packet_chunks = data.chunks_exact(8);
    let mut has_tdc: bool = false;
    let mut time = 0.0f64;
    let mut ele_time;

    let mut ci: u8 = last_ci;
    loop {
        match packet_chunks.next() {
            None => break,
            Some(&[84, 80, 88, 51, nci, _, _, _]) => ci = nci,
            Some(x) => {
                let packet = Packet {
                    chip_index: ci,
                    i08: x[0],
                    i09: x[1],
                    i10: x[2],
                    i11: x[3],
                    i12: x[4],
                    i13: x[5],
                    i14: x[6],
                    i15: x[7],
                };
                
                match packet.id() {
                    11 => {
                        ele_time = Packet::elec_time(packet.spidr(), packet.toa(), packet.ftoa());
                        let xpos = (xspim as f64 * ((ele_time - last_tdc)/interval)) as usize;
                        let array_pos = bytedepth * (packet.x() + 1024*xspim*line + 1024*xpos);
                        let array_pos = if array_pos > max_value {array_pos - max_value} else {array_pos};
                        Packet::append_to_array(final_data, array_pos, bytedepth);
                    },
                    6 => {
                        time = Packet::tdc_time(packet.tdc_coarse(), packet.tdc_fine());
                        time = time - (time / (26843545600.0 * 1e-9)).floor() * 26843545600.0 * 1e-9;
                        if has_tdc == true {println!("tdc already true");}
                        has_tdc = true;
                    },
                    7 => {continue;},
                    4 => {continue;},
                    _ => {},
                };
            },
        };
    };
    (ci, has_tdc, time)
}

fn search_next_tdc(data: &[u8], last_ci: u8) -> (u8, bool, f64) {
    
    let file_data = data;
    let mut packet_chunks = file_data.chunks_exact(8);
    let mut has_tdc: bool = false;
    let mut time = 0.0f64;

    let mut ci: u8 = last_ci;
    loop {
        match packet_chunks.next() {
            None => break,
            Some(&[84, 80, 88, 51, nci, _, _, _]) => ci = nci,
            Some(x) => {
                let packet = Packet {
                    chip_index: ci,
                    i08: x[0],
                    i09: x[1],
                    i10: x[2],
                    i11: x[3],
                    i12: x[4],
                    i13: x[5],
                    i14: x[6],
                    i15: x[7],
                };
                
                match packet.id() {
                    11 => {continue;},
                    6 => {
                        time = Packet::tdc_time(packet.tdc_coarse(), packet.tdc_fine());
                        let tdc_counter = packet.tdc_counter();
                        println!("Tdc {} @ {}", tdc_counter, time);
                        has_tdc = true;
                    },
                    7 => {continue;},
                    4 => {continue;},
                    _ => {},
                };
            },
        };
    };
    (ci, has_tdc, time)
}

fn create_header(time: f64, frame: usize, data_size: usize, bitdepth: usize, width: usize, height: usize, xspim: usize, yspim: usize) -> Vec<u8> {
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
    msg.push_str(",\"xspim\":");
    msg.push_str(&(xspim.to_string()));
    msg.push_str(",\"yspim\":");
    msg.push_str(&(yspim.to_string()));
    msg.push_str("}\n");
    let s: Vec<u8> = msg.into_bytes();
    s
}

fn compress(data: &[u8]) {
    let mut iter = data.iter();
}

fn connect_and_loop(runmode: RunningMode) {

    let bin: bool;
    let bytedepth:usize;
    let cumul: bool;
    let is_spim:bool;
    let xspim;
    let yspim;

    let pack_listener = TcpListener::bind("127.0.0.1:8098").expect("Could not connect to packets.");
    let ns_listener = match runmode {
        RunningMode::DebugStem7482 => TcpListener::bind("127.0.0.1:8088").expect("Could not connect to NS in debug."),
        RunningMode::Tp3 => TcpListener::bind("192.168.199.11:8088").expect("Could not connect to NS using TP3."),
    };

    if let Ok((packet_socket, packet_addr)) = pack_listener.accept() {
        println!("Localhost TP3 detected at {:?}", packet_addr);
        if let Ok((ns_socket, ns_addr)) = ns_listener.accept() {
            println!("Nionswift connected at {:?}", ns_addr);

            let mut pack_sock = packet_socket;
            let mut ns_sock = ns_socket;
            
            let mut cam_settings = [0 as u8; 16];
            match ns_sock.read(&mut cam_settings){
                Ok(_) => {
                    let my_config = Config{data: cam_settings};
                    bin = my_config.bin();
                    bytedepth = my_config.bytedepth();
                    cumul = my_config.cumul();
                    is_spim = my_config.is_spim();
                    xspim = my_config.xspim_size();
                    yspim = my_config.yspim_size();
                },
                Err(_) => panic!("Could not read cam initial settings."),
            }
            
            pack_sock.set_read_timeout(Some(Duration::from_micros(1_000))).expect("Could not set packets socket read timeout.");
            ns_sock.set_read_timeout(Some(Duration::from_micros(100))).expect("Could not set NS socket read timeout.");
            println!("Received settings is {:?}.", cam_settings);
            
            let start = Instant::now();
            let mut counter = 0usize;
            let mut last_ci = 0u8;
            let mut frame_time:f64;
            let mut has_tdc:bool;
            let mut buffer_pack_data: [u8; 8192] = [0; 8192];
           
            match is_spim {
                false => {
                    assert_eq!(xspim, 1); assert_eq!(yspim, 1);
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
                            let size = pack_sock.read(&mut buffer_pack_data).expect("Could not read TPX3.");
                            if size>0 {
                                let new_data = &buffer_pack_data[0..size];
                                let result = build_data(new_data, bin, &mut data_array, last_ci, bytedepth);
                                last_ci = result.0;
                                has_tdc = result.1;
                                
                                if has_tdc==true {
                                    frame_time = result.2;
                                    counter+=1;
                                    let msg = match bin {
                                        true => create_header(frame_time, counter, bytedepth*1024, bytedepth<<3, 1024, 1, 1, 1),
                                        false => create_header(frame_time, counter, bytedepth*256*1024, bytedepth<<3, 1024, 256, 1, 1),
                                    };

                                    if let Err(_) = ns_sock.write(&msg) {println!("Client disconnected on header."); break 'global;}
                                    if let Err(_) = ns_sock.write(&data_array) {println!("Client disconnected on data."); break 'global;}

                                    break;
                                }
                            } else {println!("Received zero packages"); break 'global;}
                        }
                        if counter % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, counter);}
                    }
                    println!("Number of loops were: {}.", counter);
                    ns_sock.shutdown(Shutdown::Both).expect("Shutdown call failed");
                },
                true => {
                    
                    let mut val:[f64; 2] = [0.0, 0.0];


                    for i in 0..2 {
                        let result = loop {
                            let size = pack_sock.read(&mut buffer_pack_data).expect("Could not read TPX3.");
                            if size>0 {
                                let new_data = &buffer_pack_data[0..size];
                                let result = search_next_tdc(new_data, last_ci);
                                let has_tdc = result.1;

                                if has_tdc == true {
                                    break result;
                                } 
                            }
                        };
                        val[i] = result.2;
                        last_ci = result.0;
                    }
                    
                    frame_time = val[1];
                    let interval:f64 = val[1] - val[0];
                    
                    let mut spim_data_array:Vec<u8> = vec![0; bytedepth*1024*xspim*yspim];
                    spim_data_array.push(10);
                    
                    'global_spim: loop {
                        loop {
                            let size = pack_sock.read(&mut buffer_pack_data).expect("Could not read TPX3.");
                            if size>0 {
                                let new_data = &buffer_pack_data[0..size];
                                let result = build_spim_data(new_data, &mut spim_data_array, last_ci, bytedepth, counter, frame_time, xspim, yspim, interval);
                                last_ci = result.0;
                                has_tdc = result.1;
                                
                                if has_tdc==true {
                                    counter+=1;
                                    frame_time = result.2;
                                    
                                    if counter%yspim==0 {
                                        let msg = create_header(frame_time, counter, bytedepth*1024*xspim*yspim, bytedepth<<3, 1024, 1, xspim, yspim);
                                        if let Err(_) = ns_sock.write(&msg) {println!("Client disconnected on header."); break 'global_spim;}
                                        if let Err(_) = ns_sock.write(&spim_data_array) {println!("Client disconnected on data."); break 'global_spim;}
                                        break;
                                    }
                                }
                            } else {println!("Received zero packages"); break 'global_spim;}
                        }
                        if counter % (xspim*yspim) == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, counter);}
                    }
                    println!("Number of loops were: {}.", counter);
                    ns_sock.shutdown(Shutdown::Both).expect("Shutdown call failed");
                },
            }
        }
    }
}

fn main() {
    loop {
        let myrun = RunningMode::DebugStem7482;
        //let myrun = RunningMode::Tp3;
        println!{"Waiting for a new client"};
        connect_and_loop(myrun);
    }
}

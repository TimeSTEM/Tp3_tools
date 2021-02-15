use std::io::prelude::*;
use std::{fs, io};
use std::io::BufReader;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::time::{Duration, Instant};
use std::{thread, time};
use std::sync::mpsc;

enum RunningMode {
    debug_stem7482,
    debug_cheetah,
    tp3,
}

struct config {
    data: [u8; 8],
}

impl config {
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

    fn spimsize(&self) -> usize {
        println!("Spim size is {}", self.data[4]);
        self.data[4] as usize
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

    fn elec_time(spidr: u16, toa: u16, ftoa: u8) -> f64 {
        let ctoa = (toa<<4) | ((!ftoa as u16) & 15);
        (spidr as f64) * 25.0 * 16384.0 + (ctoa as f64) * 25.0 / 16.0
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
    
    let file_data = data;
    let bin = bin;
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

fn build_spim_data(data: &[u8], bin: bool, final_data: &mut [u8], last_ci: u8, bytedepth: usize) -> (u8, bool, f64) {
    
    let file_data = data;
    let bin = bin;
    let mut packet_chunks = file_data.chunks_exact(8);
    let mut has_tdc: bool = false;
    let mut time = 0.0f64;
    let mut ele_time = 0.0f64;

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
                        ele_time = Packet::elec_time(packet.spidr(), packet.toa(), packet.ftoa());
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

fn connect_and_loop(runmode: RunningMode) {

    let mut bin: bool = true;
    let mut bytedepth = 2usize;
    let mut cumul: bool = false;
    let mut is_spim:bool = false;
    let mut spimsize = 0usize;

    let pack_listener = TcpListener::bind("127.0.0.1:8098").unwrap();
    
    let ns_listener = match runmode {
        RunningMode::debug_stem7482 => TcpListener::bind("127.0.0.1:8088").unwrap(),
        RunningMode::tp3 | RunningMode::debug_cheetah=> TcpListener::bind("192.168.199.11:8088").unwrap(),
    };

    if let Ok((packet_socket, packet_addr)) = pack_listener.accept() {
        println!("Localhost TP3 detected at {:?}", packet_addr);
        if let Ok((ns_socket, ns_addr)) = ns_listener.accept() {
            println!("Nionswift connected at {:?}", ns_addr);

            let mut pack_sock = packet_socket;
            let mut ns_sock = ns_socket;
            
            let mut cam_settings = [0 as u8; 8];
            if let Ok(_) = ns_sock.read(&mut cam_settings){
                let my_config = config{data: cam_settings};
                bin = my_config.bin();
                bytedepth = my_config.bytedepth();
                cumul = my_config.cumul();
                is_spim = my_config.is_spim();
                spimsize = my_config.spimsize();
            };
            
            pack_sock.set_read_timeout(Some(Duration::from_micros(1_000))).unwrap();
            ns_sock.set_read_timeout(Some(Duration::from_micros(100))).unwrap();
            println!("Received settings is {:?}.", cam_settings);
            
            let start = Instant::now();
           
            match is_spim {
                false => {
                    
                    let mut counter = 0usize;
                    let last_ci = 0u8;
                    let mut buffer_pack_data: [u8; 64000] = [0; 64000];
                    
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
                                    //build_spim_data(new_data, bin, &mut spim_data_array, last_ci, bytedepth);
                                    let result = build_data(new_data, bin, &mut data_array, last_ci, bytedepth);
                                    let last_ci = result.0;
                                    let has_tdc = result.1;
                                    let frame_time = result.2;
                                    
                                    if has_tdc==true {
                                        let msg = create_header(frame_time, counter, bytedepth*1024*(256-255*(bin as usize)), bytedepth<<3, 1024, 256 - 255*(bin as usize), spimsize, spimsize);
                                        counter+=1;
                                        match ns_sock.write(&msg) {
                                            Ok(_) => {},
                                            Err(_) => {
                                                println!("Client {} disconnected on header. Waiting a new one.", ns_addr);
                                                break 'global;
                                            },
                                        }
                                        match ns_sock.write(&data_array) {
                                            Ok(_) => {},
                                            Err(_) => {
                                                println!("Client {} disconnected on data. Waiting a new one.", ns_addr);
                                                break 'global;
                                            },
                                        }
                                        break;
                                    }; 
                                } else {
                                    println!("Received zero packages");
                                    break 'global;
                                }
                            }
                        }
                        if counter % 100 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}", elapsed);}
                    }
                    println!("Number of loops were: {}.", counter);
                    ns_sock.shutdown(Shutdown::Both).expect("Shutdown call failed");
                },
                true => {
       
                    let mut counter = 0usize;
                    let last_ci = 0u8;
                    let mut buffer_pack_data: [u8; 64000] = [0; 64000];
                    
                    let mut spim_data_array:Vec<u8> = vec![0; bytedepth*1024*spimsize*spimsize];
                    spim_data_array.push(10);
                    
                    'global_spim: loop {
                        
                        loop {
                            if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                                if size>0 {
                                    let new_data = &buffer_pack_data[0..size];
                                    let result = build_spim_data(new_data, bin, &mut spim_data_array, last_ci, bytedepth);
                                    let last_ci = result.0;
                                    let has_tdc = result.1;
                                    let frame_time = result.2;
                                    
                                    if has_tdc==true {
                                        let msg = create_header(frame_time, counter, bytedepth*1024*spimsize*spimsize, bytedepth<<3, 1024, 1, spimsize, spimsize);
                                        counter+=1;
                                        
                                        match ns_sock.write(&msg) {
                                            Ok(_) => {},
                                            Err(_) => {
                                                println!("Client {} disconnected on header. Waiting a new one.", ns_addr);
                                                break 'global_spim;
                                            },
                                        }
                                        match ns_sock.write(&spim_data_array) {
                                            Ok(_) => {},
                                            Err(_) => {
                                                println!("Client {} disconnected on data. Waiting a new one.", ns_addr);
                                                break 'global_spim;
                                            },
                                        }
                                        break;
                                    }; 
                                } else {
                                    println!("Received zero packages");
                                    break 'global_spim;
                                }
                            }
                        }
                        if counter % 100 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}", elapsed);}
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
        let myrun = RunningMode::debug_stem7482;
        //let myrun = RunningMode::tp3;
        println!{"Waiting for a new client"};
        connect_and_loop(myrun);
    }
}

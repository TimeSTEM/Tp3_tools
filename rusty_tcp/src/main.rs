use std::io::prelude::*;
use std::net::{Shutdown, TcpListener};
use std::time::{Duration, Instant};

enum RunningMode {
    DebugStem7482,
    Tp3,
}

enum TdcType {
    TdcOneRisingEdge,
    TdcOneFallingEdge,
    TdcTwoRisingEdge,
    TdcTwoFallingEdge,
}

impl TdcType {
    fn associate_value(&self) -> u8 {
        match *self {
            TdcType::TdcOneRisingEdge => 15,
            TdcType::TdcOneFallingEdge => 10,
            TdcType::TdcTwoRisingEdge => 14,
            TdcType::TdcTwoFallingEdge => 11,
        }
    }

    fn associate_string(&self) -> &str {
        match *self {
            TdcType::TdcOneRisingEdge => "One_Rising",
            TdcType::TdcOneFallingEdge => "One_Falling",
            TdcType::TdcTwoRisingEdge => "Two_Rising",
            TdcType::TdcTwoFallingEdge => "Two_Falling",
        }
    }

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
        (self.data[4] as usize)<<8 | (self.data[5] as usize)
    }
    
    fn yspim_size(&self) -> usize {
        (self.data[6] as usize)<<8 | (self.data[7] as usize)
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

    fn tot(&self) -> u16 {
        ((self.i10 & 240) as u16)>>4 | ((self.i11 & 63) as u16)<<4
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

    fn tdc_type(&self) -> u8 {
        self.i15 & 15 
    }
    
    fn tdc_type_as_enum(&self) -> Result<TdcType, &str> {
        match self.i15 & 15 {
            15 => Ok(TdcType::TdcOneRisingEdge),
            10 => Ok(TdcType::TdcOneFallingEdge),
            14 => Ok(TdcType::TdcTwoRisingEdge),
            11 => Ok(TdcType::TdcTwoFallingEdge),
            _ => Err("Bad TDC receival"),
        }
    }

    fn is_tdc_type_oneris(&self) -> Result<bool, &str> {
        match self.i15 & 15 {
            15 => Ok(true),
            10 | 14 | 11 => Ok(false),
            _ => Err("Bad TDC receival"),
        }
    }
    
    fn is_tdc_type_onefal(&self) -> Result<bool, &str> {
        match self.i15 & 15 {
            10 => Ok(true),
            15 | 14 | 11 => Ok(false),
            _ => Err("Bad TDC receival"),
        }
    }
    
    fn is_tdc_type_tworis(&self) -> Result<bool, &str> {
        match self.i15 & 15 {
            14 => Ok(true),
            10 | 15 | 11 => Ok(false),
            _ => Err("Bad TDC receival"),
        }
    }

    fn is_tdc_type_twofal(&self) -> Result<bool, &str> {
        match self.i15 & 15 {
            11 => Ok(true),
            10 | 14 | 15 => Ok(false),
            _ => Err("Bad TDC receival"),
        }
    }
    
    fn ctoa(toa: u16, ftoa: u8) -> u32 {
        ((toa as u32) << 4) | (!(ftoa as u32) & 15)
    }

    fn elec_time(spidr: u16, toa: u16, ftoa: u8) -> f64 {
        let ctoa = ((toa as u32 )<<4) | (!(ftoa as u32) & 15);
        ((spidr as f64) * 25.0 * 16384.0 + (ctoa as f64) * 25.0 / 16.0) / 1e9
    }

    fn tdc_time(coarse: u64, fine: u8) -> f64 {
        (coarse as f64) * (1.0/320e6) + (fine as f64) * 260e-12
    }

    fn append_to_array(data: &mut [u8], index:usize, bytedepth: usize) -> bool{
        let index = index * bytedepth;
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

    fn append_to_index_array(data: &mut Vec<u8>, index: usize) {
        let val0 = ((index & 4_278_190_080)>>24) as u8;
        let val1 = ((index & 16_711_680)>>16) as u8;
        let val2 = ((index & 65_280)>>8) as u8;
        let val3 = (index & 255) as u8;
        data.push(val0);
        data.push(val1);
        data.push(val2);
        data.push(val3);
    }
}


fn build_data(data: &[u8], final_data: &mut [u8], bin: bool, last_ci: u8, bytedepth: usize, kind: u8) -> (u8, f64, usize) {
    
    let bin = bin;
    let mut packet_chunks = data.chunks_exact(8);
    let mut has_tdc: bool = false;
    let mut time = 0.0f64;
    let mut tdc_counter = 0;

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
                            false => packet.x() + 1024*packet.y(),
                            true => packet.x()
                        };
                        Packet::append_to_array(final_data, array_pos, bytedepth);
                    },
                    6 if packet.tdc_type() == kind => {
                        time = Packet::tdc_time(packet.tdc_coarse(), packet.tdc_fine());
                        tdc_counter+=1;
                    },
                    7 => {continue;},
                    4 => {continue;},
                    _ => {},
                };
            },
        };
    };
    (ci, time, tdc_counter)
}

fn build_spim_data(data: &[u8], last_ci: u8, bytedepth: usize, line_number: usize, last_tdc: f64, xspim: usize, yspim: usize, interval: f64, tdc_kind: u8) -> (u8, f64, usize, Vec<u8>) {
    
    let line = line_number % yspim;
    let max_value = xspim*yspim*1024;
    let mut packet_chunks = data.chunks_exact(8);
    let mut time = 0.0f64;
    let mut ele_time;
    let mut tdc_counter = 0;
    let mut index_data:Vec<u8> = Vec::new();

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
                        let mut array_pos = (packet.x() + 1024*xspim*line + 1024*xpos);
                        while array_pos>=max_value {
                            array_pos -= max_value;
                        }
                        Packet::append_to_index_array(&mut index_data, array_pos);
                    },
                    6 if packet.tdc_type() == tdc_kind => {
                        time = Packet::tdc_time(packet.tdc_coarse(), packet.tdc_fine());
                        time = time - (time / (26843545600.0 * 1e-9)).floor() * 26843545600.0 * 1e-9;
                        if (time - last_tdc) > 1.1*interval {
                            println!("{} and {} and {}", time, last_tdc, packet.tdc_counter());
                        }
                        tdc_counter+=1;
                    },
                    _ => {continue;},
                };
            },
        };
    };
    (ci, time, tdc_counter, index_data)
}

fn search_any_tdc(data: &[u8], tdc_vec: &mut Vec<(f64, TdcType)>) -> u8 {
    
    let file_data = data;
    let mut packet_chunks = file_data.chunks_exact(8);
    let mut ci: u8 = 0;

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
                        let time = Packet::tdc_time(packet.tdc_coarse(), packet.tdc_fine());
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
    ci
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

fn create_spim_header(line: u8, column: u8) -> Vec<u8> {
    let mut msg: String = String::from("{\"linePosition\":");
    msg.push_str(&(line.to_string()));
    msg.push_str(",\"colPosition\":");
    msg.push_str(&(column.to_string()));
    let s: Vec<u8> = msg.into_bytes();
    s
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
            let mut buffer_pack_data: [u8; 200] = [0; 200];
           
            match is_spim {
                false => {
                    assert_eq!(xspim, 1); assert_eq!(yspim, 1);
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
                                    let result = build_data(new_data, &mut data_array, bin, last_ci, bytedepth, tdc_type);
                                    last_ci = result.0;
                                    counter += result.2;
                                    
                                    if result.2>0 {
                                        frame_time = result.1;
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
                    println!("Number of loops were: {}.", counter);
                    ns_sock.shutdown(Shutdown::Both).expect("Shutdown call failed");
                },
                true => {
                    let mut tdc_vec:Vec<(f64, TdcType)> = Vec::new();
                    let start_tdc_type = TdcType::TdcOneFallingEdge.associate_value();
                    let stop_tdc_type = TdcType::TdcOneRisingEdge.associate_value();
                    let ntdc = 3;

                    loop {
                        if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                            if size>0 {
                                let new_data = &buffer_pack_data[0..size];
                                last_ci = search_any_tdc(new_data, &mut tdc_vec);
                                if tdc_vec.iter().filter(|(time, tdct)| tdct.associate_value()==start_tdc_type).count() >= ntdc {
                                    break;
                                } 
                            }
                        }
                    };

                    let start_array: Vec<_> = tdc_vec.iter()
                        .filter(|(time, tdct)| tdct.associate_value()==start_tdc_type)
                        .map(|(time, tdct)| time)
                        .collect();
                    
                    frame_time = *tdc_vec.iter()
                        .filter(|(time, tdct)| tdct.associate_value()==start_tdc_type)
                        .map(|(time, tdct)| time)
                        .last().unwrap();
                    
                    let end_array: Vec<_> = tdc_vec.iter()
                        .filter(|(time, tdct)| tdct.associate_value()==stop_tdc_type)
                        .map(|(time, tdct)| time)
                        .collect();

                    let dead_time:f64 = if (start_array[0] - end_array[0])>0.0 {start_array[0] - end_array[0]} else {start_array[1] - end_array[0]};
                    let interval:f64 = (start_array[2] - start_array[1]) - dead_time;
                    println!("Interval time (us) is {:?}. Measured dead time (us) is {:?}", interval*1.0e6, dead_time*1.0e6);

                    'global_spim: loop {
                        loop {
                            if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                                if size>0 {
                                    let new_data = &buffer_pack_data[0..size];
                                    let result = build_spim_data(new_data, last_ci, bytedepth, counter, frame_time, xspim, yspim, interval, start_tdc_type);
                                    last_ci = result.0;
                                    counter+=result.2;
                                    if let Err(_) = ns_sock.write(&result.3) {println!("Client disconnected on data."); break 'global_spim;}
                                    if result.2>0 {frame_time = result.1;}
                                } else {println!("Received zero packages"); break 'global_spim;}
                            }
                        }
                        let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, counter);
                    }
                    println!("Number of loops were: {}.", counter);
                    //ns_sock.shutdown(Shutdown::Both).expect("Shutdown call failed");
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

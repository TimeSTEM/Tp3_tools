//!`timepix3` is a collection of tools to run and analyze the detector TimePix3 in live conditions. This software is
//!intented to be run in a different computer in which the data will be shown. Raw data is supossed to
//!be collected via a socket in localhost and be sent to a client prefentiably using a 10 Gbit/s
//!Ethernet.

pub mod auxiliar;
pub mod tdclib;
pub mod packetlib;

///`modes` is a module containing tools to live acquire frames and spectral images.
pub mod modes {
    use crate::packetlib::{Packet, PacketEELS, PacketDiff as Pack};
    use crate::auxiliar::Settings;
    use crate::tdclib::{TdcControl, PeriodicTdcRef, NonPeriodicTdcRef};
    use std::time::Instant;
    use std::net::TcpStream;
    use std::io::{Read, Write};

    const SPIM_PIXELS: usize = 1025;
    const VIDEO_TIME: f64 = 0.000007;
    const CLUSTER_TIME: f64 = 50.0e-09;
    //const CAM_DESIGN: (usize, usize) = (1024, 256);
    const CAM_DESIGN: (usize, usize) = (512, 512);

    ///Returns a vector containing a list of indexes in which events happened. Uses a single TDC at
    ///the beggining of each scan line.
    pub fn build_spim_data(data: &[u8], last_ci: &mut u8, settings: &Settings, line_tdc: &mut PeriodicTdcRef) -> Vec<(f64, usize, usize)> {
        let mut packet_chunks = data.chunks_exact(8);
        let mut timelist:Vec<(f64, usize, usize)> = Vec::new();
        let interval = line_tdc.low_time;
        let period = line_tdc.period;

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
                _ => {
                    let packet = Pack { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        11 => {
                            if let Some(x) = packet.x() {
                                let ele_time = packet.electron_time() - VIDEO_TIME;
                                if let Some(backline) = spim_check_if_in(ele_time, line_tdc.time, interval, period) {
                                    let line = (((line_tdc.counter - backline) / settings.spimoverscany) % settings.yspim_size) * SPIM_PIXELS * settings.xspim_size;
                                    let xpos = (settings.xspim_size as f64 * ((ele_time - (line_tdc.time - (backline as f64)*period))/interval)) as usize * SPIM_PIXELS;
                                    let array_pos = x + line + xpos;
                                    timelist.push((ele_time, x, array_pos));
                                }
                            }
                            
                        },
                        6 if packet.tdc_type() == line_tdc.tdctype => {
                            line_tdc.upt(packet.tdc_time_norm());
                        },
                        _ => {},
                    };
                },
            };
        };
        timelist
    }
    
    ///Returns a vector containing a list of indexes in which events happened for a time-resolved (TR) measurement.
    ///Uses one TDC for the beginning of a scan line and another one for the TR.
    pub fn build_tr_spim_data(data: &[u8], last_ci: &mut u8, settings: &Settings, line_tdc: &mut PeriodicTdcRef, ref_tdc: &mut PeriodicTdcRef) -> Vec<u8> {
        let mut packet_chunks = data.chunks_exact(8);
        let mut index_data:Vec<u8> = Vec::new();
        let interval = line_tdc.low_time;
        let period = line_tdc.period;

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
                _ => {
                    let packet = Pack { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        11 => {
                            if let Some(x) = packet.x() {
                                let mut ele_time = packet.electron_time();
                                if let Some(_backtdc) = tr_check_if_in(ele_time, ref_tdc.time, ref_tdc.period, settings) {
                                    ele_time -= VIDEO_TIME;
                                    if let Some(backline) = spim_check_if_in(ele_time, line_tdc.time, interval, period) {
                                        let line = ((line_tdc.counter - backline) / settings.spimoverscany) % settings.yspim_size;
                                        let xpos = (settings.xspim_size as f64 * ((ele_time - (line_tdc.time - (backline as f64)*period))/interval)) as usize;
                                        let array_pos = x + SPIM_PIXELS*settings.xspim_size*line + SPIM_PIXELS*xpos;
                                        append_to_index_array(&mut index_data, array_pos);
                                    }
                                }
                            }
                            
                        },
                        6 if packet.tdc_type() == line_tdc.tdctype => {
                            line_tdc.upt(packet.tdc_time_norm());
                        },
                        6 if packet.tdc_type() == ref_tdc.tdctype => {
                            ref_tdc.upt(packet.tdc_time_norm());
                        },
                        _ => {},
                    };
                },
            };
        };
        index_data
    }

    ///Returns a vector containing a list of indexes in which TDC events happened. Uses one TDC
    ///referred to the beginning of a new scan line and a second Non Periodic TDC to use as pixel
    ///counter.
    pub fn build_tdc_spim_data(data: &[u8], last_ci: &mut u8, settings: &Settings, line_tdc: &mut PeriodicTdcRef, ref_tdc: &mut NonPeriodicTdcRef) -> Vec<u8> {
        let mut packet_chunks = data.chunks_exact(8);
        let mut index_data:Vec<u8> = Vec::new();
        let interval = line_tdc.low_time;
        let period = line_tdc.period;

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
                _ => {
                    let packet = Pack { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        11 => {
                            /*
                            if let Some(x) = packet.x() {
                                let mut ele_time = packet.electron_time();
                                ele_time -= VIDEO_TIME;
                                if let Some(backline) = spim_check_if_in(ele_time, line_tdc.time, interval, period) {
                                    let line = ((line_tdc.counter - backline) / settings.spimoverscany) % settings.yspim_size;
                                    let xpos = (settings.xspim_size as f64 * ((ele_time - (line_tdc.time - (backline as f64)*period))/interval)) as usize;
                                    let array_pos = packet.x().unwrap() + SPIM_PIXELS*settings.xspim_size*line + SPIM_PIXELS*xpos;
                                    append_to_index_array(&mut index_data, array_pos);
                                }
                            }
                            */
                            
                        },  
                        6 if packet.tdc_type() == line_tdc.tdctype => {
                            line_tdc.upt(packet.tdc_time_norm());
                        },
                        6 if packet.tdc_type() == ref_tdc.tdctype => {
                            let tdc_time = packet.tdc_time_norm();
                            ref_tdc.upt(tdc_time);
                            let tdc_time = tdc_time - VIDEO_TIME;
                            if let Some(backline) = spim_check_if_in(tdc_time, line_tdc.time, interval, period) {
                                let line = ((line_tdc.counter - backline) / settings.spimoverscany) % settings.yspim_size;
                                let xpos = (settings.xspim_size as f64 * ((tdc_time - (line_tdc.time - (backline as f64)*period))/interval)) as usize;
                                let array_pos = (SPIM_PIXELS-1) + SPIM_PIXELS*settings.xspim_size*line + SPIM_PIXELS*xpos;
                                append_to_index_array(&mut index_data, array_pos);
                            }
                        },
                        _ => {},
                    };
                },
            };
        };
        index_data
    }
 






    pub fn build_spectrum<T: TdcControl, U: TdcControl>(mut pack_sock: TcpStream, mut ns_sock: TcpStream, my_settings: Settings, mut frame_tdc: T, _ref_tdc: U) {
        
        let start = Instant::now();
        let mut last_ci = 0u8;
        let mut buffer_pack_data = vec![0; 16384];
        let mut data_array:Vec<u8> = vec![0; (255*!my_settings.bin as usize + 1)*my_settings.bytedepth*1024];
        data_array.push(10);
        
            loop {
                if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                    if size>0 {
                        let new_data = &buffer_pack_data[0..size];
                            if build_data(new_data, &mut data_array, &mut last_ci, &my_settings, &mut frame_tdc) {
                            let msg = create_header(&my_settings, &frame_tdc);
                            if let Err(_) = ns_sock.write(&msg) {println!("Client disconnected on header."); break;}
                            if let Err(_) = ns_sock.write(&data_array) {println!("Client disconnected on data."); break;}
                            
                            if my_settings.cumul == false {
                                data_array = vec![0; (255*!my_settings.bin as usize + 1)*my_settings.bytedepth*1024];
                                data_array.push(10);
                            }

                           if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());}
                        }
                    } else {println!("Received zero packages"); break;}
                }
            }
    }

    ///Returns a frame using a periodic TDC as reference.
    fn build_data<T: TdcControl>(data: &[u8], final_data: &mut [u8], last_ci: &mut u8, settings: &Settings, tdc: &mut T) -> bool {

        let mut packet_chunks = data.chunks_exact(8);
        let mut has = false;

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
                _ => {
                    //let packet = Packet { chip_index: *last_ci, data: x};
                    //let packet_test = match CAM_DESIGN {
                    //    (512, 512) => Box::new(PacketDiff { chip_index: *last_ci, data: x}) as Box<Packet>,
                    //    (1024, 256) => Box::new(PacketEELS { chip_index: *last_ci, data: x}) as Box<Packet>,
                    //    _ => panic!("***Lib***: Packet must be (512, 512) or (1024, 256)."),
                    //};
                    let packet_test = Pack { chip_index: *last_ci, data: x};
                    
                    match packet_test.id() {
                        11 => {
                            if let (Some(x), Some(y)) = (packet_test.x(), packet_test.y()) {
                                let array_pos = match settings.bin {
                                    false => x + CAM_DESIGN.0*y,
                                    true => x
                                };
                                append_to_array(final_data, array_pos, settings.bytedepth);
                                
                            }
                        },
                        6 if packet_test.tdc_type() == tdc.id() => {
                            tdc.upt(packet_test.tdc_time());
                            has = true;
                        },
                        _ => {},
                    };
                },
            };
        };
        has
    }

    ///Returns a frame using a periodic TDC as reference and a second TDC to discriminate in time.
    pub fn tr_build_data(data: &[u8], final_data: &mut [u8], last_ci: &mut u8, settings: &Settings, frame_tdc: &mut PeriodicTdcRef, ref_tdc: &mut PeriodicTdcRef) -> bool {
        let mut packet_chunks = data.chunks_exact(8);
        let mut has = false;


        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
                _ => {
                    let packet = Pack { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        11 => {
                            if let (Some(x), Some(y)) = (packet.x(), packet.y()) {
                                let ele_time = packet.electron_time();
                                if let Some(_backtdc) = tr_check_if_in(ele_time, ref_tdc.time, ref_tdc.period, settings) {
                                    let array_pos = match settings.bin {
                                        false => x + 1024*y,
                                        true => x
                                    };
                                    append_to_array(final_data, array_pos, settings.bytedepth);
                                }
                            }
                        },
                        6 if packet.tdc_type() == frame_tdc.tdctype => {
                            frame_tdc.upt(packet.tdc_time());
                            has = true;
                        },
                        6 if packet.tdc_type() == ref_tdc.tdctype => {
                            ref_tdc.upt(packet.tdc_time_norm());
                        },
                        _ => {},
                    };
                },
            };
        };
        has
    }
    



    fn tr_check_if_in(ele_time: f64, tdc: f64, period: f64, settings: &Settings) -> Option<usize> {
        let mut eff_tdc = tdc;
        let mut counter = 0;
        while ele_time < eff_tdc {
            counter+=1;
            eff_tdc = eff_tdc - period;
        }
        
        if counter>5 {return None}
        
        if ele_time > eff_tdc + settings.time_delay && ele_time < eff_tdc + settings.time_delay + settings.time_width {
            Some(counter)
        } else {
            None
        }
    }
    
    fn spim_check_if_in(ele_time: f64, start_line: f64, interval: f64, period: f64) -> Option<usize> {
        let mut new_start_line = start_line;
        let mut counter = 0;
        while ele_time < new_start_line {
            counter+=1;
            new_start_line = new_start_line - period;
        }
        
        if counter>5 {return None}
        
        if ele_time > new_start_line && ele_time < new_start_line + interval {
            Some(counter)
        } else {
            None
        }
    }
    
    ///Append a single electron to a given size array. Used mainly for frame based.
    fn append_to_array(data: &mut [u8], index:usize, bytedepth: usize) {
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
            },
            2 => {
                data[index+1] = data[index+1].wrapping_add(1);
                if data[index+1]==0 {
                    data[index] = data[index].wrapping_add(1);
                }
            },
            1 => {
                data[index] = data[index].wrapping_add(1);
            },
            _ => {panic!("Bytedepth must be 1 | 2 | 4.");},
        }
    }
    
    pub fn sort_and_append_to_index(mut tl: Vec<(f64, usize, usize)>) -> Vec<u8> {
        let mut index_array: Vec<u8> = Vec::new();
        if let Some(val) = tl.get(0) {
            let mut last = val.clone();
            tl.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
            for tp in tl {
                if tp.0>last.0+CLUSTER_TIME || (tp.1 as isize - last.1 as isize).abs() < 2 {
                    append_to_index_array(&mut index_array, tp.2);
                }
                last = tp;
            }
        }
        index_array
    }
    
    ///Append a single electron to a index list. Used mainly for spectral image, where a list of
    ///indexes is passed to client computer. Always push indexes using 32 bits.
    fn append_to_index_array(data: &mut Vec<u8>, index: usize) {
        data.push(((index & 4_278_190_080)>>24) as u8);
        data.push(((index & 16_711_680)>>16) as u8);
        data.push(((index & 65_280)>>8) as u8);
        data.push((index & 255) as u8);
        }

    ///Create header, used mainly for frame based spectroscopy.
    fn create_header<T: TdcControl>(set: &Settings, tdc: &T) -> Vec<u8> {
        let mut msg: String = String::from("{\"timeAtFrame\":");
        msg.push_str(&(tdc.time().to_string()));
        msg.push_str(",\"frameNumber\":");
        msg.push_str(&(tdc.counter().to_string()));
        msg.push_str(",\"measurementID:\"Null\",\"dataSize\":");
        match set.bin {
            true => { msg.push_str(&((set.bytedepth*1024).to_string()))},
            false => { msg.push_str(&((set.bytedepth*1024*256).to_string()))},
        }
        msg.push_str(",\"bitDepth\":");
        msg.push_str(&((set.bytedepth<<3).to_string()));
        msg.push_str(",\"width\":");
        msg.push_str(&(CAM_DESIGN.0.to_string()));
        msg.push_str(",\"height\":");
        match set.bin {
            true=>{msg.push_str(&(1.to_string()))},
            false=>{msg.push_str(&(CAM_DESIGN.1.to_string()))},
        }
        msg.push_str("}\n");

        let s: Vec<u8> = msg.into_bytes();
        s
    }

}

///`misc` or `miscelaneous` is a module containing shared tools between modes.
pub mod misc {
    use crate::tdclib::{PeriodicTdcRef};
    use crate::auxiliar::Settings;
    
    ///Create header, used mainly for frame based spectroscopy.
    pub fn create_header(set: &Settings, tdc: &PeriodicTdcRef) -> Vec<u8> {
        let mut msg: String = String::from("{\"timeAtFrame\":");
        msg.push_str(&(tdc.time.to_string()));
        msg.push_str(",\"frameNumber\":");
        msg.push_str(&(tdc.counter.to_string()));
        msg.push_str(",\"measurementID:\"Null\",\"dataSize\":");
        match set.bin {
            true => { msg.push_str(&((set.bytedepth*1024).to_string()))},
            false => { msg.push_str(&((set.bytedepth*1024*256).to_string()))},
        }
        msg.push_str(",\"bitDepth\":");
        msg.push_str(&((set.bytedepth<<3).to_string()));
        msg.push_str(",\"width\":");
        msg.push_str(&(1024.to_string()));
        msg.push_str(",\"height\":");
        match set.bin {
            true=>{msg.push_str(&(1.to_string()))},
            false=>{msg.push_str(&(256.to_string()))},
        }
        msg.push_str("}\n");

        let s: Vec<u8> = msg.into_bytes();
        s
    }
}

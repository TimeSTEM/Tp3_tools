//!`timepix3` is a collection of tools to run and analyze the detector TimePix3 in live conditions. This software is
//!intented to be run in a different computer in which the data will be shown. Raw data is supossed to
//!be collected via a socket in localhost and be sent to a client prefentiably using a 10 Gbit/s
//!Ethernet.

pub mod auxiliar;
pub mod tdclib;
pub mod packetlib;

///`spectral_image` is a module containing tools to live acquire spectral images.
pub mod modes {
    use crate::packetlib::Packet;
    use crate::auxiliar::Settings;
    use crate::tdclib::PeriodicTdcRef;
    use crate::misc;
    
    ///Returns a vector containing a list of indexes in which events happened.
    pub fn build_spim_data(data: &[u8], last_ci: &mut u8, settings: &Settings, line_tdc: &mut PeriodicTdcRef) -> Vec<u8> {
        let mut packet_chunks = data.chunks_exact(8);
        let mut index_data:Vec<u8> = Vec::new();
        let interval = line_tdc.low_time;
        let period = line_tdc.period;

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
                _ => {
                    let packet = Packet { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        11 => {
                            let ele_time = packet.electron_time() - 0.000007;
                            if let Some(backline) = spim_check_if_in(ele_time, line_tdc.time, interval, period) {
                                let line = ((line_tdc.counter - backline) / settings.spimoverscany) % settings.yspim_size;
                                let xpos = (settings.xspim_size as f64 * ((ele_time - (line_tdc.time - (backline as f64)*period))/interval)) as usize;
                                let array_pos = packet.x() + 1024*settings.xspim_size*line + 1024*xpos;
                                misc::append_to_index_array(&mut index_data, array_pos);
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
        index_data
    }
    
    ///Returns a vector containing a list of indexes in which events happened for a TR measurement..
    pub fn build_tr_spim_data(data: &[u8], last_ci: &mut u8, settings: &Settings, line_tdc: &mut PeriodicTdcRef, ref_tdc: &mut PeriodicTdcRef) -> Vec<u8> {
        let mut packet_chunks = data.chunks_exact(8);
        let mut index_data:Vec<u8> = Vec::new();
        let interval = line_tdc.low_time;
        let period = line_tdc.period;

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
                _ => {
                    let packet = Packet { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        11 => {
                            let mut ele_time = packet.electron_time();
                            if let Some(_backtdc) = tr_check_if_in(ele_time, ref_tdc.time, ref_tdc.period, settings) {
                                ele_time -= 0.000007;
                                if let Some(backline) = spim_check_if_in(ele_time, line_tdc.time, interval, period) {
                                    let line = ((line_tdc.counter - backline) / settings.spimoverscany) % settings.yspim_size;
                                    let xpos = (settings.xspim_size as f64 * ((ele_time - (line_tdc.time - (backline as f64)*period))/interval)) as usize;
                                    let array_pos = packet.x() + 1024*settings.xspim_size*line + 1024*xpos;
                                    misc::append_to_index_array(&mut index_data, array_pos);
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
    

    pub fn build_data(data: &[u8], final_data: &mut [u8], last_ci: &mut u8, settings: &Settings, tdc: &mut PeriodicTdcRef) -> bool {

        let mut packet_chunks = data.chunks_exact(8);
        let mut has = false;

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
                _ => {
                    let packet = Packet { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        11 => {
                            let array_pos = match settings.bin {
                                false => packet.x() + 1024*packet.y(),
                                true => packet.x()
                            };
                            misc::append_to_array(final_data, array_pos, settings.bytedepth);
                        },
                        6 if packet.tdc_type() == tdc.tdctype => {
                            tdc.upt(packet.tdc_time());
                            has = true;
                        },
                        _ => {},
                    };
                },
            };
        };
        has
    }

    pub fn tr_build_data(data: &[u8], final_data: &mut [u8], last_ci: &mut u8, settings: &Settings, frame_tdc: &mut PeriodicTdcRef, ref_tdc: &mut PeriodicTdcRef) -> bool {
        let mut packet_chunks = data.chunks_exact(8);
        let mut has = false;

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
                _ => {
                    let packet = Packet { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        11 => {
                            let ele_time = packet.electron_time();
                            if let Some(_backtdc) = tr_check_if_in(ele_time, ref_tdc.time, ref_tdc.period, settings) {
                                let array_pos = match settings.bin {
                                    false => packet.x() + 1024*packet.y(),
                                    true => packet.x()
                                };
                                misc::append_to_array(final_data, array_pos, settings.bytedepth);
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
    
}

///`misc` or `miscelaneous` is a module containing shared tools between modes.
pub mod misc {
    use crate::tdclib::TdcType;
    use crate::packetlib::Packet;
    use crate::auxiliar::Settings;
    use crate::tdclib::PeriodicTdcRef;
    
    pub fn search_any_tdc(data: &[u8], tdc_vec: &mut Vec<(f64, TdcType)>, last_ci: &mut u8) {
        
        let file_data = data;
        let mut packet_chunks = file_data.chunks_exact(8);

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
                _ => {
                    let packet = Packet { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        6 => {
                            let time = packet.tdc_time_norm();
                            let tdc = TdcType::associate_value_to_enum(packet.tdc_type()).unwrap();
                            tdc_vec.push( (time, tdc) );
                        },
                        _ => {},
                    };
                },
            };
        };
    }

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

    pub fn append_to_array(data: &mut [u8], index:usize, bytedepth: usize) {
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

    pub fn put_all_to_zero(data: &mut [u8]) {
        for val in data {
            *val = 0;
        }
    }
    
    pub fn append_to_index_array(data: &mut Vec<u8>, index: usize) {
        data.push(((index & 4_278_190_080)>>24) as u8);
        data.push(((index & 16_711_680)>>16) as u8);
        data.push(((index & 65_280)>>8) as u8);
        data.push((index & 255) as u8);
    }
}

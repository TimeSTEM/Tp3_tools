//!`timepix3` is a collection of tools to run and analyze the detector TimePix3 in live conditions. This software is
//!intented to be run in a different computer in which the data will be shown. Raw data is supossed to
//!be collected via a socket in localhost and be sent to a client prefentiably using a 10 Gbit/s
//!Ethernet.

pub mod auxiliar;
pub mod tdclib;
pub mod packetlib;

///`spectral_image` is a module containing tools to live acquire spectral images.
pub mod spectral_image {
    use crate::packetlib::Packet;
    use crate::misc;
    use std::fs;
    
    ///Returns a vector containing a list of indexes in which events happened.
    pub fn build_spim_data(data: &[u8], last_ci: &mut u8, counter: &mut usize, sltdc: &mut f64, spim: (usize, usize), yratio: usize, interval: f64, period: f64, tdc_kind: u8) -> Vec<u8> {
        let mut packet_chunks = data.chunks_exact(8);
        let mut index_data:Vec<u8> = Vec::new();

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
                _ => {
                    let packet = Packet { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        11 => {
                            let ele_time = packet.electron_time() - 0.000007;
                            if let Some(backline) = place_pixel(ele_time, sltdc, interval, period) {
                                let line = ((*counter - backline) / yratio) % spim.1;
                                let xpos = (spim.0 as f64 * ((ele_time - (*sltdc - (backline as f64)*period))/interval)) as usize;
                                let array_pos = packet.x() + 1024*spim.0*line + 1024*xpos;
                                misc::append_to_index_array(&mut index_data, array_pos);
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
    
    pub fn build_save_spim_data(data: &[u8], final_data: &mut [u8], last_ci: &mut u8, counter: &mut usize, sltdc: &mut f64, spim: (usize, usize), yratio: usize, interval: f64, bytedepth: usize, tdc_kind: u8) -> Vec<u8> {
        let mut packet_chunks = data.chunks_exact(8);
        let mut index_data:Vec<u8> = Vec::new();
        let per = spim.1 * 10;

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
                _ => {
                    let packet = Packet { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        11 => {
                            let ele_time = packet.electron_time() - 0.000007;
                            if check_if_in(ele_time, sltdc, interval) {
                                let line = (*counter / yratio) % spim.1;
                                let xpos = (spim.0 as f64 * ((ele_time - *sltdc)/interval)) as usize;
                                let array_pos = packet.x() + 1024*spim.0*line + 1024*xpos;
                                misc::append_to_index_array(&mut index_data, array_pos);
                                misc::append_to_array(final_data, array_pos, bytedepth);
                            }
                        },
                        6 if packet.tdc_type() == tdc_kind => {
                            *sltdc = packet.tdc_time_norm();
                            *counter+=1;
                            if *counter % per == 0 {
                                let image = *counter / per;
                                let mut filename = String::from("slice");
                                filename.push_str(&(image.to_string()));
                                filename.push_str(".dat");
                                fs::write(filename, &*final_data);
                                misc::put_all_to_zero(final_data);
                            }
                        },
                        _ => {},
                    };
                },
            };
        };
        index_data
    }

    ///Returns the deadtime between consecutive scan lines.
    pub fn find_deadtime(start_line: &[f64], end_line: &[f64]) -> f64 {
        if (start_line[1] - end_line[1])>0.0 {start_line[1] - end_line[1]} else {start_line[2] - end_line[1]}
    }

    ///Returns the effective time interval between lines.
    pub fn find_interval(start_line: &[f64], deadtime: f64) -> f64 {
        (start_line[2] - start_line[1]) - deadtime
    }

    pub fn find_period(start_line: &[f64]) -> f64 {
        start_line[2] - start_line[1]
    }

    ///Checks if event is in the appropriate time interval to be counted.
    fn check_if_in(ele_time: f64, start_line: &f64, interval: f64) -> bool {
        if ele_time>*start_line && ele_time<(*start_line + interval) {
        true
        } else {false}
    }

    fn place_pixel(ele_time: f64, start_line: &f64, interval: f64, period: f64) -> Option<usize> {
        let mut new_start_line = *start_line;
        let mut counter = 0;
        while ele_time < new_start_line {
            counter+=1;
            new_start_line = new_start_line - period;
        }
        
        if counter>3 {return None}
        
        if ele_time > new_start_line && ele_time < new_start_line + interval {
            Some(counter)
        } else {
            None
        }

    }
}



///`spectrum` is a module containing tools to live acquire frame-based spectra. Uses one tdc to
///define frame.
pub mod spectrum {
    use crate::packetlib::Packet;
    use crate::misc;
    
    pub fn build_data(data: &[u8], final_data: &mut [u8], last_ci: &mut u8, counter: &mut usize, frame_time: &mut f64, bin: bool, bytedepth: usize, kind: u8) -> bool {

        let mut packet_chunks = data.chunks_exact(8);
        let mut has = false;

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
                            misc::append_to_array(final_data, array_pos, bytedepth);
                        },
                        6 if packet.tdc_type() == kind => {
                            *frame_time = packet.tdc_time();
                            *counter+=1;
                            has = true;
                        },
                        _ => {},
                    };
                },
            };
        };
        has
    }

    pub fn tr_build_data(data: &[u8], final_data: &mut [u8], last_ci: &mut u8, counter: &mut usize, frame_time: &mut f64, ref_time: &mut Vec<f64>, bin: bool, bytedepth: usize, frame_tdc: u8, ref_tdc: u8, tdelay: f64, twidth: f64, period: f64) -> bool {
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
                            if tr_check_if_in(ref_time, ele_time, tdelay, twidth) {
                                let array_pos = match bin {
                                    false => packet.x() + 1024*packet.y(),
                                    true => packet.x()
                                };
                                misc::append_to_array(final_data, array_pos, bytedepth);
                            }
                        },
                        6 if packet.tdc_type() == frame_tdc => {
                            *frame_time = packet.tdc_time();
                            *counter+=1;
                            has = true;
                        },
                        6 if packet.tdc_type() == ref_tdc => {
                            let time = packet.tdc_time_norm();
                            ref_time.remove(0);
                            ref_time.pop().unwrap();
                            ref_time.push(time);
                            ref_time.push(time+period);
                        },
                        _ => {},
                    };
                },
            };
        };
        has
    }
    
    fn tr_check_if_in(time_vec: &Vec<f64>, time: f64, delay: f64, width: f64) -> bool {
        for val in time_vec {
            if time>val+delay && time<val+delay+width {
                return true
            }
        }
        false
    }

    pub fn tr_create_start_vectime(mut at: Vec<f64>) -> Vec<f64> {
        let rec = at.pop().unwrap();
        let br = at.pop().unwrap();
        let bbr = at.pop().unwrap();
        let bbbr = at.pop().unwrap();
        let interval = rec - br;
        let ref_time:Vec<f64> = [bbbr, bbr, br, rec, rec+interval].to_vec();
        ref_time
    }
    
    pub fn tr_find_period(at: &[f64]) -> f64 {
        at[1] - at[0]
    }

}




///`misc` or `miscelaneous` is a module containing shared tools between modes.
pub mod misc {
    use crate::tdclib::TdcType;
    use crate::packetlib::Packet;
    
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

    pub fn create_header(time: f64, frame: usize, data_size: usize, bitdepth: usize, width: usize, height: usize) -> Vec<u8> {
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

    pub fn append_to_array(data: &mut [u8], index:usize, bytedepth: usize) -> bool{
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

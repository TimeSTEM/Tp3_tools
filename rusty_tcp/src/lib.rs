//!`timepix3` is a collection of tools to run and analyze the detector TimePix3 in live conditions. This software is
//!intented to be run in a different computer in which the data will be shown. Raw data is supossed to
//!be collected via a socket in localhost and be sent to a client prefentiably using a 10 Gbit/s
//!Ethernet.

pub mod auxiliar;
pub mod tdclib;
pub mod packetlib;

pub mod spectral_image {
    use crate::packetlib::Packet;
    
    pub fn build_spim_data(data: &[u8], last_ci: &mut u8, counter: &mut usize, sltdc: &mut f64, spim: (usize, usize), yratio: usize, interval: f64, tdc_kind: u8) -> Vec<u8> {
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
                            if check_if_in(ele_time, sltdc, interval) {
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

    pub fn find_deadtime(start_line: &[f64], end_line: &[f64]) -> f64 {
        if (start_line[1] - end_line[1])>0.0 {start_line[1] - end_line[1]} else {start_line[2] - end_line[1]}
    }

    pub fn find_interval(start_line: &[f64], deadtime: f64) -> f64 {
        (start_line[2] - start_line[1]) - deadtime
    }

    pub fn check_if_in(ele_time: f64, start_line: &f64, interval: f64) -> bool {
        if ele_time>*start_line && ele_time<(*start_line + interval) {
        true
        } else {false}
    }
}

pub mod tr_spectrum {
    use crate::packetlib::Packet;

    pub fn build_time_data(data: &[u8], final_data: &mut [u8], last_ci: &mut u8, frame_time: &mut f64, ref_time: &mut Vec<f64>, bin: bool, bytedepth: usize, frame_tdc: u8, ref_tdc: u8, tdelay: f64, twidth: f64) -> usize {
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
                            
                            if check_if_in(ref_time, ele_time, tdelay, twidth) {
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
                        _ => {},
                    };
                },
            };
        };
        tdc_counter
    }

    pub fn check_if_in(time_vec: &Vec<f64>, time: f64, delay: f64, width: f64) -> bool {
        for val in time_vec {
            if time>val+delay && time<val+delay+width {
                return true
            }
        }
        false
    }

    pub fn create_start_vectime(mut at: Vec<f64>) -> Vec<f64> {
        let ref_time:Vec<f64> = [at.pop().unwrap(), at.pop().unwrap()].to_vec();
        ref_time
    }
}

pub mod spectrum {
    use crate::packetlib::Packet;
    
    pub fn build_data(data: &[u8], final_data: &mut [u8], last_ci: &mut u8, frame_time: &mut f64, bin: bool, bytedepth: usize, kind: u8) -> usize {

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
                        _ => {},
                    };
                },
            };
        };
        tdc_counter
    }
}

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

}

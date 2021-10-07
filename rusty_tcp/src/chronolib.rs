///`modes` is a module containing tools to live acquire frames and spectral images.
use crate::packetlib::{Packet, PacketEELS as Pack};
use crate::auxiliar::Settings;
use crate::tdclib::{TdcControl, PeriodicTdcRef};
use std::time::Instant;
use std::net::TcpStream;
use std::io::{Read, Write};

const CAM_DESIGN: (usize, usize) = Pack::chip_array();
const BUFFER_SIZE: usize = 16384 * 3;

///Reads timepix3 socket and writes in the output socket a header and a full frame (binned or not). A periodic tdc is mandatory in order to define frame time. Chrono Mode.
pub fn build_chrono<T: TdcControl, V: Read>(mut pack_sock: V, mut vec_ns_sock: Vec<TcpStream>, my_settings: Settings, mut frame_tdc: PeriodicTdcRef, mut ref_tdc: T) {

    let start = Instant::now();
    let mut last_ci = 0usize;
    let mut buffer_pack_data = vec![0; BUFFER_SIZE];
    let mut data_array:Vec<u8> = vec![0; my_settings.xspim_size*my_settings.bytedepth*CAM_DESIGN.0];
    data_array.push(10);

    while let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
        if size == 0 {println!("Timepix3 sent zero bytes."); break;}
        let new_data = &buffer_pack_data[0..size];
        if build_chrono_data(new_data, &mut data_array, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
            let msg = create_header(&my_settings, &frame_tdc);
            if let Err(_) = vec_ns_sock[0].write(&msg) {println!("Client disconnected on header."); break;}
            if let Err(_) = vec_ns_sock[0].write(&data_array) {println!("Client disconnected on data."); break;}
            if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());}
        }
    }
}

fn build_chrono_data<T: TdcControl>(data: &[u8], final_data: &mut [u8], last_ci: &mut usize, settings: &Settings, frame_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) -> bool {

    let mut packet_chunks = data.chunks_exact(8);
    let mut has = false;
    
    while let Some(x) = packet_chunks.next() {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci as usize,
            _ => {
                let packet = Pack { chip_index: *last_ci, data: x};
                
                match packet.id() {
                    11 if ref_tdc.period().is_none() => {
                        let line = frame_tdc.counter() % settings.xspim_size;
                        let array_pos = packet.x() + line * CAM_DESIGN.0;
                        append_to_array(final_data, array_pos, settings.bytedepth);
                    },
                    6 if packet.tdc_type() == frame_tdc.id() => {
                        frame_tdc.upt(packet.tdc_time(), packet.tdc_counter());
                        if frame_tdc.counter() % 5 == 0 {
                            has = true;
                        }
                    },
                    6 if packet.tdc_type() == ref_tdc.id() => {
                        ref_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
                        if ref_tdc.period().is_none() {
                            let line = frame_tdc.counter() % settings.xspim_size;
                            let array_pos = CAM_DESIGN.0-1 + line * CAM_DESIGN.0;
                            append_to_array(final_data, array_pos, settings.bytedepth);
                        }   
                    },
                    _ => {},
                };
            },
        };
    };
    has
}

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

fn create_header<T: TdcControl>(set: &Settings, tdc: &T) -> Vec<u8> {
    let mut msg: String = String::from("{\"timeAtFrame\":");
    msg.push_str(&(tdc.time().to_string()));
    msg.push_str(",\"frameNumber\":");
    msg.push_str(&(tdc.counter().to_string()));
    msg.push_str(",\"measurementID:\"Null\",\"dataSize\":");
    msg.push_str(&((set.xspim_size*set.bytedepth*CAM_DESIGN.0).to_string()));
    msg.push_str(",\"bitDepth\":");
    msg.push_str(&((set.bytedepth<<3).to_string()));
    msg.push_str(",\"width\":");
    msg.push_str(&(CAM_DESIGN.0.to_string()));
    msg.push_str(",\"height\":");
    msg.push_str(&(set.xspim_size.to_string()));
    msg.push_str("}\n");

    let s: Vec<u8> = msg.into_bytes();
    s
}

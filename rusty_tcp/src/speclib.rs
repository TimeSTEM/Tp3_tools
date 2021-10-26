///`modes` is a module containing tools to live acquire frames and spectral images.
use crate::packetlib::{Packet, PacketEELS as Pack};
use crate::auxiliar::Settings;
use crate::tdclib::{TdcControl, PeriodicTdcRef};
use std::time::Instant;
use std::io::{Read, Write};

const CAM_DESIGN: (usize, usize) = Pack::chip_array();
const BUFFER_SIZE: usize = 16384 * 2;

/*
pub fn build_spectrum_thread<T, V>(mut pack_sock: V, mut vec_ns_sock: Vec<TcpStream>, my_settings: Settings, mut frame_tdc: PeriodicTdcRef, mut ref_tdc: T) 
    where T: 'static + Send + TdcControl,
          V: 'static + Send + Read
{
    
    let (tx, rx) = mpsc::channel();
    let start = Instant::now();
    let mut last_ci = 0usize;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let len: usize = ((CAM_DESIGN.1-1)*!my_settings.bin as usize + 1)*my_settings.bytedepth*CAM_DESIGN.0;
    //let mut data_array: Vec<u8> = vec![0; ((CAM_DESIGN.1-1)*!my_settings.bin as usize + 1)*my_settings.bytedepth*CAM_DESIGN.0];
    let mut data_array = vec![0; len + 1];
    data_array[len] = 10;

    thread::spawn(move || {
        while let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                if size == 0 {println!("Timepix3 sent zero bytes."); break;}
                let new_data = &buffer_pack_data[0..size];
                if build_data(new_data, &mut data_array, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
                    let msg = create_header(&my_settings, &frame_tdc);
                    tx.send((data_array.clone(), msg)).expect("could not send data in the thread channel.");
                    if my_settings.cumul == false {
                        data_array = vec![0; len + 1];
                        data_array[len] = 10;
                    };
                    if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());}
                 }
        }
    });

    let mut ns_sock = vec_ns_sock.pop().expect("Could not pop nionswift main socket.");
    for (result, msg) in rx {
        if let Err(_) = ns_sock.write(&msg) {println!("Client disconnected on data."); break;}
        if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break;}
    }
    println!("Total elapsed time is: {:?}.", start.elapsed());
}
*/



///Reads timepix3 socket and writes in the output socket a header and a full frame (binned or not). A periodic tdc is mandatory in order to define frame time.
pub fn build_spectrum<T: TdcControl, V: Read, U: Write>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut frame_tdc: PeriodicTdcRef, mut ref_tdc: T) {

    let start = Instant::now();
    let mut last_ci = 0usize;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let len: usize = ((CAM_DESIGN.1-1)*!my_settings.bin as usize + 1)*my_settings.bytedepth*CAM_DESIGN.0;
    let mut data_array:Vec<u8> = vec![0; len + 1];
    data_array[len] = 10;

    while let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
        if size == 0 {println!("Timepix3 sent zero bytes."); break;}
        let new_data = &buffer_pack_data[0..size];
        if build_data(new_data, &mut data_array, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
            let msg = create_header(&my_settings, &frame_tdc);
            if let Err(_) = ns_sock.write(&msg) {println!("Client disconnected on header."); break;}
            if let Err(_) = ns_sock.write(&data_array) {println!("Client disconnected on data."); break;}
            if my_settings.cumul == false {
                data_array.iter_mut().for_each(|x| *x = 0);
                data_array[len] = 10;
            }
            if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());
            };
        }
    }
    println!("Total elapsed time is: {:?}.", start.elapsed());
}

fn build_data<T: TdcControl>(data: &[u8], final_data: &mut [u8], last_ci: &mut usize, settings: &Settings, frame_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) -> bool {

    let mut has = false;

    if data.len() % 8 != 0 {
        println!("Data was not multiple of 8. Rejecting lenght of: {}", data.len());
        return false
    }

    let array_pos = |pack: &Pack| {
        match settings.bin {
            true => pack.x(),
            false => pack.x() + CAM_DESIGN.0 * pack.y(),
        }
    };

    data.chunks_exact(8).for_each( |x| {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci as usize,
            _ => {
                let packet = Pack { chip_index: *last_ci, data: x};
                
                match packet.id() {
                    11 => {
                        if ref_tdc.period().is_none() {
                            append_to_array(final_data, array_pos(&packet), settings.bytedepth);
                        } else {
                            if tr_check_if_in(packet.electron_time(), ref_tdc.time(), ref_tdc.period().unwrap(), settings) {
                                append_to_array(final_data, array_pos(&packet), settings.bytedepth);
                            }
                        }
                    },
                    6 if packet.tdc_type() == frame_tdc.id() => {
                        frame_tdc.upt(packet.tdc_time(), packet.tdc_counter());
                        has = true;
                    },
                    6 if packet.tdc_type() == ref_tdc.id() => {
                        ref_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
                        if ref_tdc.period().is_none() {
                            append_to_array(final_data, CAM_DESIGN.0-1, settings.bytedepth);
                        }   
                    },
                    _ => {},
                };
            },
        };
    });
    has
}

fn tr_check_if_in(ele_time: usize, tdc: usize, period: usize, settings: &Settings) -> bool {
    let eff_tdc = if tdc > ele_time {
        let xper = (tdc - ele_time) / period + 1;
        tdc - xper * period
    } else {
        tdc
    };

    if ele_time > eff_tdc + settings.time_delay && ele_time < eff_tdc + settings.time_delay + settings.time_width {
        true
    } else {
        false
    }
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
    match set.bin {
        true => { msg.push_str(&((set.bytedepth*CAM_DESIGN.0).to_string()))},
        false => { msg.push_str(&((set.bytedepth*CAM_DESIGN.0*CAM_DESIGN.1).to_string()))},
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

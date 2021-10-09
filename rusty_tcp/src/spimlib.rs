use crate::packetlib::{Packet, PacketEELS};
use crate::auxiliar::Settings;
use crate::tdclib::{TdcControl, PeriodicTdcRef};
use std::net::TcpStream;
use std::time::Instant;
use std::io::{Read, Write};
use std::sync::mpsc;
use std::thread;
//use rayon::prelude::*;

const VIDEO_TIME: f64 = 0.000005;
const CLUSTER_TIME: f64 = 50.0e-09;
const SPIM_PIXELS: usize = 1025;
const BUFFER_SIZE: usize = 16384 * 4;
const UNIQUE_BYTE: usize = 1;
const INDEX_BYTE: usize = 4;

/// Possible outputs to build spim data. `usize` does not implement cluster detection. `(f64,
/// usize, usize, u8)` performs cluster detection. This could reduce the data flux but will
/// cost processing time.
pub struct Output<T>{
    data: Vec<T>,
}

impl<T> Output<T> {
    fn upt(&mut self, new_data: T) {
        self.data.push(new_data);
    }

    fn check(&self) -> bool {
        self.data.iter().next().is_some()
    }
}

impl Output<(f64, usize, usize, u8)> {
    fn build_output(mut self) -> Vec<u8> {
        let mut index_array: Vec<usize> = Vec::new();
        if let Some(val) = self.data.get(0) {
            let mut last = val.clone();
            self.data.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
            for tp in self.data {
                if (tp.0>last.0+CLUSTER_TIME || (tp.1 as isize - last.1 as isize).abs() > 2) || tp.3==6 {
                    index_array.push(tp.2);
                }
                last = tp;
            }
        }
        event_counter(index_array)
    }
}

impl Output<usize> {

    fn build_output(self) -> Vec<u8> {
        event_counter(self.data)
    }
}

impl Output<(usize, f64)> {
    fn build_output(self, set: &Settings, spim_tdc: &PeriodicTdcRef) -> Vec<u8> {
        let my_iter = self.data.into_iter()
            .filter(|&(x, dt)| {(dt / spim_tdc.period).fract() < spim_tdc.low_time / spim_tdc.period && dt > 0.0})
            .map(|(x, dt)| {
                 let ratio = dt / spim_tdc.period;
                 (x, ratio as usize, ratio.fract())
            })
            .map(|(x, r, rin)| {
                let val = ((r/set.spimoverscany) % set.yspim_size * set.xspim_size + (set.xspim_size as f64 * rin * (spim_tdc.period / spim_tdc.low_time)) as usize) * SPIM_PIXELS + x;
                vec![val, 0]
            })
            .flatten();
            //.collect::<Vec<usize>>();

    //event_counter(my_iter)
    vec![0]
    }
}



fn event_counter(mut my_vec: Vec<usize>) -> Vec<u8> {
    my_vec.sort_unstable();
    let mut unique:Vec<u8> = Vec::new();
    let mut index:Vec<u8> = Vec::new();
    let mut counter:usize = 1;
    if my_vec.len() > 0 {
        let mut last = my_vec[0];
        for val in my_vec {
            if last == val {
                //counter.wrapping_add(1);
                counter+=1;
            } else {
                append_to_index_array(&mut unique, counter, UNIQUE_BYTE);
                append_to_index_array(&mut index, last, INDEX_BYTE);
                counter = 1;
            }
            last = val;
        }
        append_to_index_array(&mut unique, counter, UNIQUE_BYTE);
        append_to_index_array(&mut index, last, INDEX_BYTE);
    }
    //let sum_unique = unique.iter().map(|&x| x as usize).sum::<usize>();
    //let mmax_unique = unique.iter().map(|&x| x as usize).max().unwrap();
    //let indexes_len = index.len();

    //let mut header_unique:Vec<u8> = String::from("{StartUnique}").into_bytes();
    let header_unique:Vec<u8> = vec![123, 83, 116, 97, 114, 116, 85, 110, 105, 113, 117, 101, 125];
    //let mut header_indexes:Vec<u8> = String::from("{StartIndexes}").into_bytes();
    let header_indexes:Vec<u8> = vec![123, 83, 116, 97, 114, 116, 73, 110, 100, 101, 120, 101, 115, 125];

    let vec = header_unique.into_iter()
        .chain(unique.into_iter())
        .chain(header_indexes.into_iter())
        .chain(index.into_iter())
        .collect::<Vec<u8>>();
    //println!("Total len with unique: {}. Total len only indexes (older): {}. Max unique is {}. Improvement is {}", vec.len(), sum_unique * 4, mmax_unique, sum_unique as f64 * 4.0 / vec.len() as f64);
    vec
}
    
///Reads timepix3 socket and writes in the output socket a list of frequency followed by a list of unique indexes. First TDC must be a periodic reference, while the second can be nothing, periodic tdc or a non periodic tdc.
pub fn build_spim<T, V>(mut pack_sock: V, mut vec_ns_sock: Vec<TcpStream>, my_settings: Settings, mut spim_tdc: PeriodicTdcRef, mut ref_tdc: T)
    where T: 'static + Send + TdcControl,
          V: 'static + Send + Read
{
    //let (tx, rx):(std::sync::mpsc::Sender<Output<usize>>, std::sync::mpsc::Receiver<Output<usize>>) = mpsc::channel();
    let (tx, rx) = mpsc::channel();
    let mut last_ci = 0usize;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let mut counter = 0;
    

    thread::spawn(move || {
        while let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
            if size == 0 {println!("Timepix3 sent zero bytes."); break;}
            if let Some(result) = test_build_spim_data(&buffer_pack_data[0..size], &mut last_ci, &my_settings, &mut spim_tdc, &mut ref_tdc) {
                if let Err(_) = tx.send(result) {println!("Cannot send data over the thread channel."); break;}
            }
        }
    });
    
    let start = Instant::now();
    let mut ns_sock = vec_ns_sock.pop().expect("Could not pop nionswift main socket.");
    for tl in rx {
        let result = tl.build_output(&my_settings, &spim_tdc);
        //let result = tl.build_output();
        //if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break;}
        //counter += 1;
        //if counter % 100 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, counter)}
    }

    let elapsed = start.elapsed(); 
    println!("Total elapsed time is: {:?}.", elapsed);

}

///Reads timepix3 socket and writes in the output socket a list of frequency followed by a list of unique indexes. First TDC must be a periodic reference, while the second can be nothing, periodic tdc or a non periodic tdc. This is a single thread function.
pub fn stbuild_spim<T, V>(mut pack_sock: V, mut vec_ns_sock: Vec<TcpStream>, my_settings: Settings, mut spim_tdc: PeriodicTdcRef, mut ref_tdc: T)
    where T: 'static + Send + TdcControl,
          V: 'static + Send + Read
{
    let mut last_ci = 0usize;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let mut counter = 0;

    let start = Instant::now();
    let mut ns_sock = vec_ns_sock.pop().expect("Could not pop nionswift main socket.");
    while let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
        if size == 0 {println!("Timepix3 sent zero bytes."); break;}
        if let Some(result) = build_spim_data(&buffer_pack_data[0..size], &mut last_ci, &my_settings, &mut spim_tdc, &mut ref_tdc) {
            let result = result.build_output();
            if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break;}
            counter += 1;
            if counter % 100 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, counter)}
        }
    }
}

fn test_build_spim_data<T: TdcControl>(data: &[u8], last_ci: &mut usize, settings: &Settings, line_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) -> Option<Output<(usize, f64)>> {

    let mut packet_chunks = data.chunks_exact(8);
    let mut list = Output{ data: Vec::new() };

    while let Some(x) = packet_chunks.next() {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci as usize,
            _ => {
                let packet = PacketEELS { chip_index: *last_ci, data: x};
                let id = packet.id();
                match id {
                    11 if ref_tdc.period().is_none() => {
                        let ele_time = packet.electron_time() - VIDEO_TIME;
                        list.upt((packet.x(), ele_time - line_tdc.begin_frame))
                    },
                    6 if packet.tdc_type() == line_tdc.id() => {
                        line_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
                        if  (line_tdc.counter / 2) % (settings.yspim_size * settings.spimoverscany) == 0 {
                            line_tdc.begin_frame = line_tdc.time();
                        }
                    },
                    6 if (packet.tdc_type() == ref_tdc.id() && ref_tdc.period().is_none())=> {
                        let tdc_time = packet.tdc_time_norm();
                        ref_tdc.upt(tdc_time, packet.tdc_counter());
                        let tdc_time = tdc_time - VIDEO_TIME;
                        list.upt((SPIM_PIXELS-1, tdc_time - line_tdc.begin_frame))
                    },
                    _ => {},
                };
            },
        };
    };
    if list.check() {Some(list)}
    else {None}
}

    
fn build_spim_data<T: TdcControl>(data: &[u8], last_ci: &mut usize, settings: &Settings, line_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) -> Option<Output<usize>> {
    //let first_index = data.chunks_exact(8).enumerate().filter(|(i, x)| x[0..4] == [84, 80, 88, 51] && i>&(0)).map(|(i, _)| 8*i).next().unwrap();
    //let half_index = data.chunks_exact(8).enumerate().filter(|(i, x)| x[0..4] == [84, 80, 88, 51] && i>&(data.len()/16)).map(|(i, _)| 8*i).next().unwrap();

    let mut packet_chunks = data.chunks_exact(8);
    let mut list = Output{ data: Vec::new() };
    let interval = line_tdc.low_time;
    let begin_frame = line_tdc.begin_frame;
    let period = line_tdc.period;

    while let Some(x) = packet_chunks.next() {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci as usize,
            _ => {
                let packet = PacketEELS { chip_index: *last_ci, data: x};
                let id = packet.id();
                match id {
                    11 if ref_tdc.period().is_none() => {
                        let ele_time = packet.electron_time() - VIDEO_TIME;
                        if let Some(array_pos) = spim_detector(ele_time, begin_frame, interval, period, settings) {
                            //list.upt((ele_time, x, array_pos+x, id));
                            list.upt(array_pos+packet.x());
                        }
                    },
                    11 if ref_tdc.period().is_some() => {
                        let mut ele_time = packet.electron_time();
                        if let Some(_backtdc) = tr_check_if_in(ele_time, ref_tdc.time(), ref_tdc.period().unwrap(), settings) {
                            ele_time -= VIDEO_TIME;
                            if let Some(backline) = spim_check_if_in(ele_time, line_tdc.time(), interval, period) {
                                let line = (((line_tdc.counter() as isize - backline) as usize / settings.spimoverscany) % settings.yspim_size) * SPIM_PIXELS * settings.xspim_size;
                                let xpos = (settings.xspim_size as f64 * ((ele_time - (line_tdc.time() - (backline as f64)*period))/interval)) as usize * SPIM_PIXELS;
                                let array_pos = packet.x() + line + xpos;
                                // This is OUTDATED
                                //list.upt((ele_time, packet.x(), array_pos, id));
                                list.upt(array_pos+packet.x());
                            }
                        }
                    },
                    6 if packet.tdc_type() == line_tdc.id() => {
                        line_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
                        //if ( (packet.tdc_counter() as usize + 4096 - line_tdc.counter_offset) / 2) % (settings.yspim_size * settings.spimoverscany) == 0 {
                        if  (line_tdc.counter / 2) % (settings.yspim_size * settings.spimoverscany) == 0 {
                            line_tdc.begin_frame = line_tdc.time();
                        }
                    },
                    6 if (packet.tdc_type() == ref_tdc.id() && ref_tdc.period().is_some())=> {
                        ref_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
                    },
                    6 if (packet.tdc_type() == ref_tdc.id() && ref_tdc.period().is_none())=> {
                        let tdc_time = packet.tdc_time_norm();
                        ref_tdc.upt(tdc_time, packet.tdc_counter());
                        let tdc_time = tdc_time - VIDEO_TIME;
                        if let Some(array_pos) = spim_detector(tdc_time, begin_frame, interval, period, settings) {
                            //list.upt((tdc_time, SPIM_PIXELS-1, array_pos+SPIM_PIXELS-1, id));
                            list.upt(array_pos+SPIM_PIXELS-1);
                        }
                    },
                    _ => {},
                };
            },
        };
    };
    if list.check() {Some(list)}
    else {None}
}


fn tr_check_if_in(ele_time: f64, tdc: f64, period: f64, settings: &Settings) -> Option<usize> {
    let mut eff_tdc = tdc;
    let mut counter = 0;
    while ele_time < eff_tdc {
        counter+=1;
        eff_tdc = eff_tdc - period;
    }
    
    if ele_time > eff_tdc + settings.time_delay && ele_time < eff_tdc + settings.time_delay + settings.time_width {
        Some(counter)
    } else {
        None
    }
}

fn spim_detector(ele_time: f64, begin: f64, interval: f64, period: f64, set: &Settings) -> Option<usize>{
    let ratio = (ele_time - begin) / period; //0 to next complete frame
    let ratio_inline = ratio.fract(); //from 0.0 to 1.0
    if ratio_inline > interval / period || ratio_inline.is_sign_negative() { //Removes electrons in line return or before last tdc
        None
    } else {
        let line = (ratio as usize / set.spimoverscany) % set.yspim_size; //multiple of yspim_size
        let xpos = (set.xspim_size as f64 * ratio_inline / (interval / period)) as usize; //absolute position in the horizontal line. Division by interval/period re-escales the X.
        let result = (line * set.xspim_size + xpos) * SPIM_PIXELS; //total array position
        Some(result)
    }
}

fn spim_check_if_in(ele_time: f64, start_line: f64, interval: f64, period: f64) -> Option<isize> {
    let mut new_start_line = start_line;
    let mut counter = 0;

    while ele_time < new_start_line {
        counter+=1;
        new_start_line = new_start_line - period;
    }

    if ele_time > new_start_line && ele_time < new_start_line + interval {
        Some(counter)
    } else {
        None
    }
}

fn append_to_index_array(data: &mut Vec<u8>, index: usize, bytedepth: usize) {
    match bytedepth {
        4 => {
            data.push(((index & 4_278_190_080)>>24) as u8);
            data.push(((index & 16_711_680)>>16) as u8);
            data.push(((index & 65_280)>>8) as u8);
            data.push((index & 255) as u8);
        },
        2 => {
            data.push(((index & 65_280)>>8) as u8);
            data.push((index & 255) as u8);
        },
        1 => {
            data.push((index & 255) as u8);
        },
        _ => {panic!("Bytedepth must be 1 | 2 | 4.");},
    }
}

fn transform_32index(index: usize) -> Vec<u8> {
    vec![ ((index & 4_278_190_080)>>24) as u8, ((index & 16_711_680)>>16) as u8, ((index & 65_280)>>8) as u8, (index & 255) as u8]
    }



pub fn debug_multithread(my_pack: [u8; 24]) {
    thread::spawn( move || {
    //    let size = my_pack.len();
        debug_build_spim_data(&my_pack);
    });
}


pub fn debug_build_spim_data(my_pack: &[u8]) {
    //Electron Packets (0 and >500 ms)
    //[2, 0, 109, 131, 230, 16, 101, 178]
    //[197, 4, 199, 0, 51, 167, 17, 180]
    //
    //Tdc Packets (0 and >500 ms)
    //[64, 188, 207, 130, 5, 128, 200, 111]
    //[96, 70, 153, 115, 31, 32, 120, 111]

    let mut packet_chunks = my_pack.chunks_exact(8);
    let mut list = Output{ data: Vec::new() };
    let interval:f64 = 61.45651;// * 10.0e-6;
    let mut begin_frame = 0.0;
    let period:f64 = 90.080465;// * 10.0-6;
    let mut tdc_counter = 0;

    let ref_tdc_period:Option<f64> = None;
    let mut last_ci = 0;

    let settings = Settings::create_debug_settings();

    while let Some(x) = packet_chunks.next() {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => last_ci = nci as usize,
            _ => {
                let packet = PacketEELS { chip_index: last_ci, data: x};
                
                let id = packet.id();
                match id {
                    11 if ref_tdc_period.is_none() => {
                        let ele_time = packet.electron_time() - VIDEO_TIME;
                        if let Some(array_pos) = spim_detector(ele_time, begin_frame, interval, period, &settings) {
                            list.upt(array_pos+packet.x());
                        }
                    },
                    6 if packet.tdc_type() == 15 => {
                        tdc_counter += 1;
                        if  (tdc_counter / 2) % (512) == 0 {
                            begin_frame = packet.tdc_time_norm()
                        }
                    },
                    6 if (packet.tdc_type() == 11 && ref_tdc_period.is_none())=> {
                        let tdc_time = packet.tdc_time_norm();
                        let tdc_time = tdc_time - VIDEO_TIME;
                        if let Some(array_pos) = spim_detector(tdc_time, begin_frame, interval, period, &settings) {
                            list.upt(array_pos+SPIM_PIXELS-1);
                        }
                    },
                    _ => {},
                };
            },
        };
    }
    if list.check() {Some(list);}
}


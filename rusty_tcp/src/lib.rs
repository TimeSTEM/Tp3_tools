//!`timepix3` is a collection of tools to run and analyze the detector TimePix3 in live conditions. This software is
//!intented to be run in a different computer in which the data will be shown. Raw data is supossed to
//!be collected via a socket in localhost and be sent to a client prefentiably using a 10 Gbit/s
//!Ethernet.

pub mod auxiliar;
pub mod tdclib;
pub mod packetlib;

///`modes` is a module containing tools to live acquire frames and spectral images.
pub mod modes {
    use crate::packetlib::{Packet, PacketEELS as Pack, PacketDiffraction};
    use crate::auxiliar::Settings;
    use crate::tdclib::{TdcControl, PeriodicTdcRef};
    use std::time::Instant;
    use std::net::TcpStream;
    use std::io::{Read, Write};
    use std::sync::mpsc;
    use std::thread;

    const VIDEO_TIME: f64 = 0.000005;
    const CLUSTER_TIME: f64 = 50.0e-09;
    const CAM_DESIGN: (usize, usize) = Pack::chip_array();
    const SPIM_PIXELS: usize = 1025;
    const BUFFER_SIZE: usize = 16384 * 3;
    const UNIQUE_BYTE: usize = 2;
    const INDEX_BYTE: usize = 4;


    pub struct Output<T>{
        data: Vec<T>,
    }

    impl<T> Output<T> {
        fn upt(&mut self, new_data: T) {
            self.data.push(new_data);
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

    /*
    impl Output<usize> {

        fn build_output(self) -> Vec<u8> {
            event_counter(self.data)
        }
    }
    */
    
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
        //let indexes_len = index.len();
        //println!("{:?}", unique);

        //let mut header_unique:Vec<u8> = String::from("{StartUnique}").into_bytes();
        let header_unique:Vec<u8> = vec![123, 83, 116, 97, 114, 116, 85, 110, 105, 113, 117, 101, 125];
        //let mut header_indexes:Vec<u8> = String::from("{StartIndexes}").into_bytes();
        let header_indexes:Vec<u8> = vec![123, 83, 116, 97, 114, 116, 73, 110, 100, 101, 120, 101, 115, 125];

        let vec = header_unique.into_iter()
            .chain(unique.into_iter())
            .chain(header_indexes.into_iter())
            .chain(index.into_iter())
            .collect::<Vec<u8>>();
        //println!("Total len with unique: {}. Total len only indexes: {}", vec.len(), sum_unique * indexes_len);
        vec
    }
        

    pub fn build_spim<T: 'static + TdcControl + Send>(mut pack_sock: TcpStream, mut vec_ns_sock: Vec<TcpStream>, my_settings: Settings, mut spim_tdc: PeriodicTdcRef, mut ref_tdc: T) {
        let (tx, rx) = mpsc::channel();
        let mut last_ci = 0usize;
        let mut buffer_pack_data = vec![0; BUFFER_SIZE];

        thread::spawn( move || {
            while let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                if size == 0 {println!("Timepix3 sent zero bytes."); break;}
                let new_data = &buffer_pack_data[0..size];
                if let Some(result) = build_spim_data(new_data, &mut last_ci, &my_settings, &mut spim_tdc, &mut ref_tdc) {
                    if let Err(_) = tx.send(result) {println!("Cannot send data over the thread channel."); break;}
                }
            }
        });

        thread::spawn( move || {
            let mut ns_sock = vec_ns_sock.pop().expect("Could not pop nionswift main socket.");
            for tl in rx {
                let result = tl.build_output();
                if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break;}
            }
        });
    }

        
    ///Returns a vector containing a list of indexes in which events happened. Uses a single TDC at
    ///the beggining of each scan line.
    //fn build_spim_data<T: TdcControl>(data: &[u8], last_ci: &mut usize, settings: &Settings, line_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) -> Option<Vec<usize>> {
    fn build_spim_data<T: TdcControl>(data: &[u8], last_ci: &mut usize, settings: &Settings, line_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) -> Option<Output<(f64, usize, usize, u8)>> {
        let mut packet_chunks = data.chunks_exact(8);
        let mut list = Output{ data: Vec::new()};
        //let mut index_array:Vec<usize> = Vec::new();
        let interval = line_tdc.low_time;
        let begin = line_tdc.begin;
        let period = line_tdc.period;

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci as usize,
                _ => {
                    let packet = Pack { chip_index: *last_ci, data: x};
                    
                    let id = packet.id();
                    match id {
                        11 if ref_tdc.period().is_none() => {
                            if let Some(x) = packet.x() {
                                let ele_time = packet.electron_time() - VIDEO_TIME;
                                if let Some(array_pos) = spim_detector(ele_time, begin, interval, period, settings) {
                                    list.upt((ele_time, x, array_pos+x, id));
                                    //timelist.push((ele_time, x, array_pos+x, id));
                                    //index_array.push(array_pos+x);
                                }
                            }
                        },
                        11 if ref_tdc.period().is_some() => {
                            if let Some(x) = packet.x() {
                                let mut ele_time = packet.electron_time();
                                if let Some(_backtdc) = tr_check_if_in(ele_time, ref_tdc.time(), ref_tdc.period().unwrap(), settings) {
                                    ele_time -= VIDEO_TIME;
                                    if let Some(backline) = spim_check_if_in(ele_time, line_tdc.time(), interval, period) {
                                        let line = (((line_tdc.counter() as isize - backline) as usize / settings.spimoverscany) % settings.yspim_size) * SPIM_PIXELS * settings.xspim_size;
                                        let xpos = (settings.xspim_size as f64 * ((ele_time - (line_tdc.time() - (backline as f64)*period))/interval)) as usize * SPIM_PIXELS;
                                        let array_pos = x + line + xpos;
                                        list.upt((ele_time, x, array_pos, id));
                                    }
                                }
                            }
                        },
                        6 if packet.tdc_type() == line_tdc.id() => {
                            line_tdc.upt(packet.tdc_time_norm());
                            if (packet.tdc_counter() as usize / 2) % (settings.yspim_size * settings.spimoverscany) == 0 {
                                line_tdc.begin = line_tdc.time();
                            }
                        },
                        6 if (packet.tdc_type() == ref_tdc.id() && ref_tdc.period().is_some())=> {
                            ref_tdc.upt(packet.tdc_time_norm());
                        },
                        6 if (packet.tdc_type() == ref_tdc.id() && ref_tdc.period().is_none())=> {
                            let tdc_time = packet.tdc_time_norm();
                            ref_tdc.upt(tdc_time);
                            let tdc_time = tdc_time - VIDEO_TIME;
                            if let Some(array_pos) = spim_detector(tdc_time, begin, interval, period, settings) {
                                list.upt((tdc_time, SPIM_PIXELS-1, array_pos+SPIM_PIXELS-1, id));
                            }
                        },
                        _ => {},
                    };
                },
            };
        };
        Some(list)
    }
    








    pub fn build_spectrum_thread<T: 'static + TdcControl + Send>(mut pack_sock: TcpStream, mut ns_sock: TcpStream, my_settings: Settings, mut frame_tdc: PeriodicTdcRef, mut ref_tdc: T) {
        
        let (tx, rx) = mpsc::channel();
        let start = Instant::now();
        let mut last_ci = 0usize;
        let mut buffer_pack_data = vec![0; BUFFER_SIZE];
        let mut data_array:Vec<u8> = vec![0; ((CAM_DESIGN.1-1)*!my_settings.bin as usize + 1)*my_settings.bytedepth*CAM_DESIGN.0];
        data_array.push(10);

        thread::spawn(move || {
            loop {
                if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                    if size>0 {
                        let new_data = &buffer_pack_data[0..size];
                            if build_data(new_data, &mut data_array, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
                                let msg = create_header(&my_settings, &frame_tdc);
                                tx.send((data_array.clone(), msg)).expect("could not send data in the thread channel.");
                                if my_settings.cumul == false {
                                    data_array = vec![0; ((CAM_DESIGN.1-1)*!my_settings.bin as usize + 1)*my_settings.bytedepth*CAM_DESIGN.0];
                                    data_array.push(10);
                                };
                                if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());}
                            }
                    }
                }
            }
        });

        loop {
            if let Ok((result, msg)) = rx.recv() {
                if let Err(_) = ns_sock.write(&msg) {println!("Client disconnected on data."); break;}
                if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break;}
            } else {break;}
        }
    }


    
    pub fn build_spectrum<T: TdcControl>(mut pack_sock: TcpStream, mut vec_ns_sock: Vec<TcpStream>, my_settings: Settings, mut frame_tdc: PeriodicTdcRef, mut ref_tdc: T) {

        let start = Instant::now();
        let mut last_ci = 0usize;
        let mut buffer_pack_data = vec![0; 16384];
        let mut data_array:Vec<u8> = vec![0; ((CAM_DESIGN.1-1)*!my_settings.bin as usize + 1)*my_settings.bytedepth*CAM_DESIGN.0];
        data_array.push(10);

        while let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
            if size == 0 {println!("Timepix3 sent zero bytes."); break;}
            let new_data = &buffer_pack_data[0..size];
            if build_data(new_data, &mut data_array, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
                let msg = create_header(&my_settings, &frame_tdc);
                if let Err(_) = vec_ns_sock[0].write(&msg) {println!("Client disconnected on header."); break;}
                if let Err(_) = vec_ns_sock[0].write(&data_array) {println!("Client disconnected on data."); break;}
                if my_settings.cumul == false {
                    data_array = vec![0; ((CAM_DESIGN.1-1)*!my_settings.bin as usize + 1)*my_settings.bytedepth*CAM_DESIGN.0];
                    data_array.push(10);
                };
                if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());}
            }
        }
    }

    ///Returns a frame using a periodic TDC as reference.
    fn build_data<T: TdcControl>(data: &[u8], final_data: &mut [u8], last_ci: &mut usize, settings: &Settings, frame_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) -> bool {

        let mut packet_chunks = data.chunks_exact(8);
        let mut has = false;
        
        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci as usize,
                _ => {
                    let packet = Pack { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        11 if ref_tdc.period().is_none() => {
                            if let (Some(x), Some(y)) = (packet.x(), packet.y()) {
                                let array_pos = match settings.bin {
                                    false => x + CAM_DESIGN.0*y,
                                    true => x
                                };
                                append_to_array(final_data, array_pos, settings.bytedepth);
                                
                            }
                        },
                        11 if ref_tdc.period().is_some() => {
                            if let (Some(x), Some(y)) = (packet.x(), packet.y()) {
                                if let Some(_backtdc) = tr_check_if_in(packet.electron_time(), ref_tdc.time(), ref_tdc.period().unwrap(), settings) {
                                    let array_pos = match settings.bin {
                                        false => x + CAM_DESIGN.0*y,
                                        true => x
                                    };
                                    append_to_array(final_data, array_pos, settings.bytedepth);
                                }
                            }
                        },
                        6 if packet.tdc_type() == frame_tdc.id() => {
                            frame_tdc.upt(packet.tdc_time());
                            has = true;
                        },
                        6 if packet.tdc_type() == ref_tdc.id() => {
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
    
    /*
    fn sort_and_append_to_unique_index(mut tl: Vec<(f64, usize, usize, u8)>) -> Vec<u8> {
        let mut index_array: Vec<usize> = Vec::new();
        if let Some(val) = tl.get(0) {
            let mut last = val.clone();
            tl.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
            for tp in tl {
                if (tp.0>last.0+CLUSTER_TIME || (tp.1 as isize - last.1 as isize).abs() > 2) || tp.3==6 {
                    index_array.push(tp.2);
                }
                last = tp;
            }
        }
        event_counter(index_array)
    }
    */
    
    fn sort_and_append_to_index(mut tl: Vec<(f64, usize, usize, u8)>) -> Vec<u8> {
        let mut index_array: Vec<u8> = Vec::new();
        if let Some(val) = tl.get(0) {
            let mut last = val.clone();
            tl.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
            for tp in tl {
                if (tp.0>last.0+CLUSTER_TIME || (tp.1 as isize - last.1 as isize).abs() > 2) || tp.3==6 {
                    append_to_index_array(&mut index_array, tp.2, 4);
                }
                last = tp;
            }
        }
        index_array
    }
    
    ///Append a single electron to a index list. Used mainly for spectral image, where a list of
    ///indexes is passed to client computer. Always push indexes using 32 bits.
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
    

    ///Create header, used mainly for frame based spectroscopy.
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
    

}

pub mod message_board {
    use std::fs;
    use std::net::{TcpListener, TcpStream};
    use std::io::{Read, Write};

    pub fn start_message_board() {
        //let (mut mb_sock, mb_addr) = mb_listener.accept().expect("Could not connect to Message Board.");
        
        let mb_listener = TcpListener::bind("127.0.0.1:9098").expect("Could not bind to Message Board.");
        for stream in mb_listener.incoming() {
            let stream = stream.unwrap();
            handle_connection(stream);
        }
    }

    fn handle_connection(mut stream: TcpStream) {
        let mut buffer = [0; 1024];
        stream.read(&mut buffer).unwrap();
        let contents = fs::read_to_string("page.html").unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            contents.len(),
            contents
        );
        stream.write(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    }

}
                     

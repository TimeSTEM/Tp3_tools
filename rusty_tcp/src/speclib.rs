///`modes` is a module containing tools to live acquire frames and spectral images.
use crate::packetlib::{Packet, PacketEELS as Pack};
use crate::auxiliar::Settings;
use crate::tdclib::{TdcControl, PeriodicTdcRef};
use crate::errorlib::Tp3ErrorKind;
use std::time::Instant;
use std::io::{Read, Write};
//use rayon::prelude::*;

const CAM_DESIGN: (usize, usize) = Pack::chip_array();
const BUFFER_SIZE: usize = 16384 * 2;
//const EXC: (usize, usize) = (20, 5); //This controls how TDC vec reduces. (20, 5) means if correlation is got in the time index >20, the first 5 items are erased.

pub trait SpecKind {
    
    fn add_electron_hit(&mut self, index: usize, pack: &Pack, settings: &Settings);
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, ref_tdc: &mut T);
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef);
    fn is_ready(&self) -> bool;
    fn build_output(&self) -> &[u8];
    fn reset_or_else(&mut self, settings: &Settings);
    fn new(settings: &Settings) -> Self;
}

pub struct Live {
    data: Vec<u8>,
    len: usize,
    is_ready: bool,
}

impl SpecKind for Live {
    
    fn add_electron_hit(&mut self, index: usize, _pack: &Pack, settings: &Settings) {
        append_to_array(&mut self.data, index, settings.bytedepth);
    }

    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        append_to_array(&mut self.data, CAM_DESIGN.0-1, settings.bytedepth);
    }

    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = true;
    }

    fn is_ready(&self) -> bool {
        self.is_ready
    }

    fn build_output(&self) -> &[u8] {
        &self.data
    }

    fn reset_or_else(&mut self, settings: &Settings) {
        self.is_ready = false;
        if settings.cumul == false {
            self.data.iter_mut().for_each(|x| *x = 0);
            self.data[self.len] = 10;
        }
    }

    fn new(settings: &Settings) -> Self {
        let len: usize = ((CAM_DESIGN.1-1)*!settings.bin as usize + 1)*settings.bytedepth*CAM_DESIGN.0;
        let mut temp_vec = vec![0; len + 1];
        temp_vec[len] = 10;
        Live{ data: temp_vec, len: len, is_ready: false}
    }
}

pub struct Correlation {
    pub data: Vec<u8>,
    pub xdata: Vec<usize>,
    pub tdata: Vec<usize>,
    pub tphoton: Vec<usize>,
    pub len: usize,
    pub is_ready: bool,
}

impl SpecKind for Correlation {
    
    fn add_electron_hit(&mut self, index: usize, pack: &Pack, settings: &Settings) {
        append_to_array(&mut self.data, index, settings.bytedepth);
        self.xdata.push(pack.x());
        self.tdata.push(pack.electron_time());
    }

    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        append_to_array(&mut self.data, CAM_DESIGN.0-1, settings.bytedepth);
        self.tphoton.push(pack.tdc_time());
    }

    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = true;
    }

    fn is_ready(&self) -> bool {
        self.is_ready
    }

    fn build_output(&self) -> &[u8] {
        &self.data
    }

    fn reset_or_else(&mut self, settings: &Settings) {
        self.is_ready = false;
        if settings.cumul == false {
            self.data.iter_mut().for_each(|x| *x = 0);
            self.data[self.len] = 10;
        }
    }

    fn new(settings: &Settings) -> Self {
        let len: usize = ((CAM_DESIGN.1-1)*!settings.bin as usize + 1)*settings.bytedepth*CAM_DESIGN.0;
        let mut temp_vec = vec![0; len + 1];
        temp_vec[len] = 10;
        Correlation { data: temp_vec, xdata: Vec::new(), tdata: Vec::new(), tphoton: Vec::new(), len: len, is_ready: false}
    }
}

/*
impl Correlation {

    fn check(&mut self, value: usize) -> Option<usize> {
        let veclen = self.tphoton.len().min(200);
        let result = self.tphoton[0..veclen].iter().map(|&x| x).enumerate().filter(|(_, t)| *t < value + 50).next();
        match result {
            Some((index, pht)) => {
                if index>EXC.0 && self.tphoton.len() > index + 100 {
                    self.tphoton = self.tphoton.iter().map(|&x| x).skip(index - EXC.1).collect();
                }
                Some(pht)
            },
            None => None,
        }
    }
}
        /*
        let veclen = self.tdc.len().min(2*MIN_LEN);
        let result = self.tdc[0..veclen].iter().cloned().enumerate().filter(|(_, x)| (x-value).abs()<TIME_WIDTH).next();
        match result {
            Some((index, pht)) => {
                if index>EXC.0 && self.tdc.len()>index+MIN_LEN{
                    self.tdc = self.tdc.iter().cloned().skip(index-EXC.1).collect();
                }
                Some(pht)
            },
            None => None,
        }
        */
*/

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
pub fn build_spectrum<T: TdcControl, V: Read, U: Write>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut frame_tdc: PeriodicTdcRef, mut ref_tdc: T) -> Result<(), Tp3ErrorKind> {
    
    let mut last_ci = 0usize;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    
    let mut list = Live::new(&my_settings);
    let start = Instant::now();

    //while let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
    while let Ok(()) = pack_sock.read_exact(&mut buffer_pack_data) {
        //if size == 0 {println!("Timepix3 sent zero bytes."); break;}
        //if build_data(&buffer_pack_data[0..size], &mut list, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
        if build_data(&buffer_pack_data, &mut list, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
            let msg = create_header(&my_settings, &frame_tdc);
            if let Err(_) = ns_sock.write(&msg) {println!("Client disconnected on header."); break;}
            if let Err(_) = ns_sock.write(list.build_output()) {println!("Client disconnected on data."); break;}
            list.reset_or_else(&my_settings);
            if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());};
        }
    }
    println!("Total elapsed time is: {:?}.", start.elapsed());
    Ok(())

}

fn build_data<T: TdcControl>(data: &[u8], final_data: &mut Live, last_ci: &mut usize, settings: &Settings, frame_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) -> bool {

    let mut has = false;
    
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
                        final_data.add_electron_hit(array_pos(&packet), &packet, settings);
                    },
                    6 if packet.tdc_type() == frame_tdc.id() => {
                        final_data.upt_frame(&packet, frame_tdc);
                        has = true;
                    },
                    6 if packet.tdc_type() == ref_tdc.id() => {
                        final_data.add_tdc_hit(&packet, settings, ref_tdc);
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
    
    if bytedepth == 4 {
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
    } else if bytedepth == 2 {
        data[index+1] = data[index+1].wrapping_add(1);
        if data[index+1]==0 {
            data[index] = data[index].wrapping_add(1);
        }
    } else if bytedepth == 1 {
        data[index] = data[index].wrapping_add(1);
    }
    
    

    /*
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
    */
    

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

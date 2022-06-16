//!`spimlib` is a collection of tools to set hyperspectral EELS acquisition.

use crate::packetlib::{Packet, PacketEELS};
use crate::auxiliar::{Settings, misc::TimepixRead};
use crate::tdclib::{TdcControl, PeriodicTdcRef, isi_box, isi_box::{IsiBoxTools, IsiBoxHand}};
use crate::errorlib::Tp3ErrorKind;
use std::time::Instant;
use crate::isi_box_new;
use std::io::{Write};
use std::sync::mpsc;
use std::thread;
use std::convert::TryInto;
use crate::auxiliar::value_types::*;
//use rayon::prelude::*;

pub const VIDEO_TIME: TIME = 3200;
pub const SPIM_PIXELS: POSITION = 1025 + 16;
const BUFFER_SIZE: usize = 16384 * 2;


///This is little endian
fn as_bytes<T>(v: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u8,
            v.len() * std::mem::size_of::<T>())
    }
}

///`SpimKind` is the main trait that measurement types must obey. Custom measurements must all
///implement these methods.
pub trait SpimKind {
    type MyOutput;

    fn data(&self) -> &Vec<Self::MyOutput>;
    fn add_electron_hit(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef);
    fn add_tdc_hit<T: TdcControl>(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef, ref_tdc: &mut T);
    fn upt_line(&self, packet: &PacketEELS, settings: &Settings, line_tdc: &mut PeriodicTdcRef);
    fn check(&self) -> bool;
    fn build_output(&self, set: &Settings, spim_tdc: &PeriodicTdcRef) -> Vec<POSITION>;
    fn copy_empty(&self) -> Self;
    fn clear(&mut self);
    fn new() -> Self;
}

#[inline]
pub fn get_return_spimindex(x: POSITION, dt: TIME, spim_tdc: &PeriodicTdcRef, xspim: POSITION, yspim: POSITION) -> Option<POSITION> {
    let val = dt % spim_tdc.period;
    let xspim = xspim;
    let yspim = yspim;
    if val >= spim_tdc.low_time {
        let mut r = (dt / spim_tdc.period) as POSITION; //how many periods -> which line to put.
        let rin = ((xspim as TIME * (val-spim_tdc.low_time)) / spim_tdc.high_time) as POSITION; //Column correction. Maybe not even needed.
            
            if r > (yspim-1) {
                if r > 4096 {return None;} //This removes overflow electrons. See add_electron_hit
                r %= yspim;
            }
            
            let index = (r * xspim + rin) * SPIM_PIXELS + x;
        
            Some(index)
        } else {
            None
        }
}

#[inline]
pub fn get_spimindex(x: POSITION, dt: TIME, spim_tdc: &PeriodicTdcRef, xspim: POSITION, yspim: POSITION) -> Option<POSITION> {
    let val = dt % spim_tdc.period;
    if val < spim_tdc.low_time {
        let mut r = (dt / spim_tdc.period) as POSITION; //how many periods -> which line to put.
        let rin = ((xspim as TIME * val) / spim_tdc.low_time) as POSITION; //Column correction. Maybe not even needed.
        //let rin = xspim  * (val / spim_tdc.low_time) as POSITION; //Column correction. Maybe not even needed.
            
            if r > (yspim-1) {
                if r > 4096 {return None;} //This removes overflow electrons. See add_electron_hit
                r %= yspim;
            }
            
            let index = (r * xspim + rin) * SPIM_PIXELS + x;
        
            Some(index)
        } else {
            None
        }
}

#[inline]
pub fn get_complete_spimindex(x: POSITION, dt: TIME, spim_tdc: &PeriodicTdcRef, xspim: POSITION, yspim: POSITION) -> POSITION {
    let val = dt % spim_tdc.period;
    let xspim = xspim;
    let yspim = yspim;
        
    let mut r = (dt / spim_tdc.period) as POSITION; //how many periods -> which line to put.
    let rin = ((xspim as TIME * val) / spim_tdc.low_time) as POSITION; //Column correction. Maybe not even needed.
            
        if r > (yspim-1) {
            r %= yspim;
        }
            
        let index = (r * xspim + rin) * SPIM_PIXELS + x;
        
        index
}

///`Live` is the only current implemented measurement. It outputs list of indices (max `u32`) that
///must be incremented.
pub struct Live {
    data: Vec<(POSITION, TIME)>,
}

impl SpimKind for Live {
    type MyOutput = (POSITION, TIME);

    fn data(&self) -> &Vec<(POSITION, TIME)> {
        &self.data
    }

    #[inline]
    fn add_electron_hit(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef) {
        let mut ele_time = packet.electron_time();
        if ele_time < line_tdc.begin_frame + VIDEO_TIME {
            let factor = (line_tdc.begin_frame + VIDEO_TIME - ele_time) / (line_tdc.period*line_tdc.ticks_to_frame.unwrap() as TIME) + 1;
            ele_time += line_tdc.period*line_tdc.ticks_to_frame.unwrap() as TIME * factor;
        }
        self.data.push((packet.x(), ele_time - line_tdc.begin_frame - VIDEO_TIME)); //This added the overflow.
    }
    
    fn add_tdc_hit<T: TdcControl>(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef, ref_tdc: &mut T) {
        let tdc_time = packet.tdc_time_norm();
        ref_tdc.upt(tdc_time, packet.tdc_counter());
        if tdc_time > line_tdc.begin_frame + VIDEO_TIME {
            self.data.push((SPIM_PIXELS-1, tdc_time - line_tdc.begin_frame - VIDEO_TIME))
        }
    }

    fn upt_line(&self, packet: &PacketEELS, _settings: &Settings, line_tdc: &mut PeriodicTdcRef) {
        line_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
    }

    fn check(&self) -> bool {
        self.data.get(0).is_some()
    }

    #[inline]
    fn build_output(&self, set: &Settings, spim_tdc: &PeriodicTdcRef) -> Vec<POSITION> {

        //First step is to find the index of the (X, Y) of the spectral image in a flattened way
        //(last index is X*Y). The line value is thus multiplied by the spim size in the X
        //direction. The column must be between [0, X]. So we have, for the position:
        //
        //index = line * xspim + column
        //
        //To find the actuall index value, one multiply this value by the number of signal pixels
        //(the spectra) because every spatial point has SPIM_PIXELS channels.
        //
        //index = index * SPIM_PIXELS
        //
        //With this, we place every electron in the first channel of the signal dimension. We must
        //thus add the pixel address to correct reconstruct the spectral image
        //
        //index = index + x
        
        
        let my_vec = self.data.iter()
            .filter_map(|&(x, dt)| {
                get_spimindex(x, dt, spim_tdc, set.xspim_size, set.yspim_size)
            }).collect::<Vec<POSITION>>();
        
        //let my_vec = self.data.iter()
        //    .map(|&(x, dt)| get_complete_spimindex(x, dt, spim_tdc, set.xspim_size, set.yspim_size))
        //    .collect::<Vec<POSITION>>();


        my_vec
    }

    fn clear(&mut self) {
        self.data.clear();
    }

    fn copy_empty(&self) -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8) }
    }

    fn new() -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8) }
    }
}

///Reads timepix3 socket and writes in the output socket a list of frequency followed by a list of unique indexes. First TDC must be a periodic reference, while the second can be nothing, periodic tdc or a non periodic tdc.
pub fn build_spim<V, T, W, U>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut spim_tdc: PeriodicTdcRef, mut ref_tdc: T, meas_type: W) -> Result<(), Tp3ErrorKind>
    where V: 'static + Send + TimepixRead,
          T: 'static + Send + TdcControl,
          W: 'static + Send + SpimKind,
          U: 'static + Send + Write,
{
    let (tx, rx) = mpsc::channel();
    let mut last_ci = 0;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let mut list = meas_type.copy_empty();

    thread::spawn(move || {
        while let Ok(size) = pack_sock.read_timepix(&mut buffer_pack_data) {
            build_spim_data(&mut list, &buffer_pack_data[0..size], &mut last_ci, &my_settings, &mut spim_tdc, &mut ref_tdc);
            if tx.send(list).is_err() {println!("Cannot send data over the thread channel."); break;}
            list = meas_type.copy_empty();
        }
    });
 
    let start = Instant::now();
    for tl in rx {
        let result = tl.build_output(&my_settings, &spim_tdc);
        if ns_sock.write(as_bytes(&result)).is_err() {println!("Client disconnected on data."); break;}
    }

    let elapsed = start.elapsed(); 
    println!("Total elapsed time is: {:?}.", elapsed);
    Ok(())
}

pub fn build_spim_isi<V, T, W, U>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut spim_tdc: PeriodicTdcRef, mut ref_tdc: T, meas_type: W) -> Result<(), Tp3ErrorKind>
    where V: 'static + Send + TimepixRead,
          T: 'static + Send + TdcControl,
          W: 'static + Send + SpimKind,
          U: 'static + Send + Write,
{
    let (tx, rx) = mpsc::channel();
    let mut last_ci = 0;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let mut list = meas_type.copy_empty();
    
    let mut handler = isi_box_new!(spim);
    handler.bind_and_connect();
    handler.configure_scan_parameters(my_settings.xspim_size.try_into().unwrap(), my_settings.yspim_size.try_into().unwrap(), spim_tdc.pixel_time(my_settings.xspim_size).try_into().unwrap());
    handler.configure_measurement_type(false);
    handler.start_threads();
    
    thread::spawn(move || {
        while let Ok(size) = pack_sock.read_timepix(&mut buffer_pack_data) {
            build_spim_data(&mut list, &buffer_pack_data[0..size], &mut last_ci, &my_settings, &mut spim_tdc, &mut ref_tdc);
            if tx.send(list).is_err() {println!("Cannot send data over the thread channel."); break;}
            list = meas_type.copy_empty();
        }
    });
 
    let start = Instant::now();
    for tl in rx {
        let result = tl.build_output(&my_settings, &spim_tdc);
        let x = handler.get_data();
        if ns_sock.write(as_bytes(&result)).is_err() {println!("Client disconnected on data."); break;}
        if x.len() > 0 {
            if ns_sock.write(as_bytes(&x)).is_err() {println!("Client disconnected on data."); break;}
        }
    }

    handler.stop_threads();
    let elapsed = start.elapsed(); 
    println!("Total elapsed time is: {:?}.", elapsed);
    Ok(())
}

fn build_spim_data<T: TdcControl, W: SpimKind>(list: &mut W, data: &[u8], last_ci: &mut u8, settings: &Settings, line_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) {

    data.chunks_exact(8).for_each(|x| {
        match *x {
            [84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
            _ => {
                let packet = PacketEELS { chip_index: *last_ci, data: x.try_into().unwrap()};
                let id = packet.id();
                match id {
                    11 => {
                        list.add_electron_hit(&packet, line_tdc);
                    },
                    6 if packet.tdc_type() == line_tdc.id() => {
                        list.upt_line(&packet, settings, line_tdc);
                    },
                    6 if packet.tdc_type() == ref_tdc.id()=> {
                        list.add_tdc_hit(&packet, line_tdc, ref_tdc);
                    },
                    _ => {},
                };
            },
        };
    });
}

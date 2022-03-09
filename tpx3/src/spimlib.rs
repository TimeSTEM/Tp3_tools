use crate::packetlib::{Packet, PacketEELS};
use crate::auxiliar::{Settings, misc::TimepixRead};
use crate::tdclib::{TdcControl, PeriodicTdcRef};
use crate::errorlib::Tp3ErrorKind;
use std::time::Instant;
use std::io::{Write};
use std::sync::mpsc;
use std::thread;
use std::convert::TryInto;
//use rayon::prelude::*;

const VIDEO_TIME: usize = 5000;
const SPIM_PIXELS: usize = 1025;
const BUFFER_SIZE: usize = 16384 * 2;

pub trait SpimKind {
    type MyOutput;

    fn data(&self) -> &Vec<Self::MyOutput>;
    fn add_electron_hit(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef);
    fn add_tdc_hit<T: TdcControl>(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef, ref_tdc: &mut T);
    fn upt_line(&self, packet: &PacketEELS, settings: &Settings, line_tdc: &mut PeriodicTdcRef);
    fn check(&self) -> bool;
    fn build_output(&self, set: &Settings, spim_tdc: &PeriodicTdcRef) -> Vec<usize>;
    fn copy_empty(&self) -> Self;
    fn new() -> Self;
}


#[inline]
pub fn get_spimindex(x: usize, dt: usize, spim_tdc: &PeriodicTdcRef, xspim: usize, yspim: usize) -> Option<usize> {
    let val = dt % spim_tdc.period;
    if val < spim_tdc.low_time {
        let mut r = dt / spim_tdc.period; //how many periods -> which line to put.
        let rin = xspim * val / spim_tdc.low_time; //Column correction. Maybe not even needed.
            
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

pub struct Live {
    data: Vec<(usize, usize)>,
}

impl SpimKind for Live {
    type MyOutput = (usize, usize);

    fn data(&self) -> &Vec<(usize, usize)> {
        &self.data
    }

    #[inline]
    fn add_electron_hit(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef) {
        let ele_time = packet.electron_time();
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
    fn build_output(&self, set: &Settings, spim_tdc: &PeriodicTdcRef) -> Vec<usize> {

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
            }).collect::<Vec<usize>>();

        my_vec
    }

    fn copy_empty(&self) -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8) }
    }

    fn new() -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8) }
    }
}

fn as_bytes(v: &[usize]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u8,
            v.len() * std::mem::size_of::<usize>())
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
    let mut last_ci = 0usize;
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

fn build_spim_data<T: TdcControl, W: SpimKind>(list: &mut W, data: &[u8], last_ci: &mut usize, settings: &Settings, line_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) {

    data.chunks_exact(8).for_each(|x| {
        match *x {
            [84, 80, 88, 51, nci, _, _, _] => *last_ci = nci as usize,
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

fn append_to_index_array(data: &mut Vec<u8>, index: usize) {
    //Big Endian
    data.push(((index >> 24 ) & 0xff) as u8);
    data.push(((index >> 16 ) & 0xff) as u8);
    data.push(((index >> 8 ) & 0xff) as u8);
    data.push((index & 0xff) as u8);
    
    //Little Endian
    //data.push((index & 0xff) as u8);
    //index = index >> 4;
    //data.push((index & 0xff) as u8);
    //index = index >> 4;
    //data.push((index & 0xff) as u8);
    //index = index >> 4;
    //data.push((index & 0xff) as u8);

}

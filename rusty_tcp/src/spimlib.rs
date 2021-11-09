use crate::packetlib::{Packet, PacketEELS};
use crate::auxiliar::{Settings, misc::TimepixRead};
use crate::tdclib::{TdcControl, PeriodicTdcRef};
use crate::errorlib::Tp3ErrorKind;
use std::time::Instant;
use std::io::{Write};
use std::sync::mpsc;
use std::thread;
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
    fn build_output(&self, set: &Settings, spim_tdc: &PeriodicTdcRef) -> Vec<u8>;
    fn copy_empty(&self) -> Self;
    fn new() -> Self;
}

pub struct Live {
    data: Vec<(usize, usize)>,
}

impl SpimKind for Live {
    type MyOutput = (usize, usize);

    fn data(&self) -> &Vec<(usize, usize)> {
        &self.data
    }

    fn add_electron_hit(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef) {
        let ele_time = packet.electron_time();
        if ele_time > line_tdc.begin_frame + VIDEO_TIME {
            self.data.push((packet.x(), ele_time - line_tdc.begin_frame - VIDEO_TIME));
        }
    }
    
    fn add_tdc_hit<T: TdcControl>(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef, ref_tdc: &mut T) {
        let tdc_time = packet.tdc_time_norm();
        ref_tdc.upt(tdc_time, packet.tdc_counter());
        if tdc_time > line_tdc.begin_frame + VIDEO_TIME {
            self.data.push((SPIM_PIXELS-1, tdc_time - line_tdc.begin_frame - VIDEO_TIME))
        }
    }

    fn upt_line(&self, packet: &PacketEELS, settings: &Settings, line_tdc: &mut PeriodicTdcRef) {
        line_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
        if (line_tdc.counter() / 2) % (settings.yspim_size * settings.spimoverscany) == 0 {
            line_tdc.begin_frame = line_tdc.time();
        }
    }

    fn check(&self) -> bool {
        self.data.iter().next().is_some()
    }

    fn build_output(&self, set: &Settings, spim_tdc: &PeriodicTdcRef) -> Vec<u8> {

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
        
        

        let mut my_vec: Vec<u8> = Vec::with_capacity(BUFFER_SIZE / 2);
        self.data.iter()
            .filter_map(|&(x, dt)| {
                let val = dt % spim_tdc.period;
                if val < spim_tdc.low_time {
                
                    let mut r = dt / spim_tdc.period; //how many periods -> which line to put.
                    let rin = set.xspim_size * val / spim_tdc.low_time; //Column
            
                    
                    if r > (set.yspim_size-1) {
                        r = r % set.yspim_size
                    }
                    
                    let index = (r * set.xspim_size + rin) * SPIM_PIXELS + x;
                    

                    //Some([((index >> 24 ) & 0xff) as u8, ((index >> 16 ) & 0xff) as u8, ((index >> 8 ) & 0xff) as u8, (index & 0xff) as u8])
                    
                
                    Some(index)
                } else {
                    None
                }
            })
            .for_each(|index| {
                //for val in index.iter() {
                //    my_vec.push(*val);
                //}
                append_to_index_array(&mut my_vec, index);
            });

    my_vec
    }

    fn copy_empty(&self) -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8) }
    }

    fn new() -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8) }
    }
}

/*
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
*/

/*
const CLUSTER_TIME: f64 = 50.0e-09;
const UNIQUE_BYTE: usize = 1;
const INDEX_BYTE: usize = 4;

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
*/

/*
impl Output<(usize, usize)> {
    fn build_output(self, set: &Settings, spim_tdc: &PeriodicTdcRef) -> Vec<u8> {
        let mut my_vec: Vec<u8> = Vec::new();

        self.data.iter()
            .filter_map(|&(x, dt)| if dt % spim_tdc.period < spim_tdc.low_time {
                let r = dt / spim_tdc.period;
                let rin = dt % spim_tdc.period;
                let index = ((r/set.spimoverscany) % set.yspim_size * set.xspim_size + (set.xspim_size * rin / spim_tdc.low_time)) * SPIM_PIXELS + x;
                Some(index) 
            } else {None}
            )
            .for_each(|index| {
                append_to_index_array(&mut my_vec, index);
            });

    my_vec
    }
}
*/

/*
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
*/
    
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
            if let Err(_) = tx.send(list) {println!("Cannot send data over the thread channel."); break;}
            list = meas_type.copy_empty();
        }
    });
 
    let start = Instant::now();
    for tl in rx {
        let result = tl.build_output(&my_settings, &spim_tdc);
        if let Err(_) = ns_sock.write(&result) {println!("Client disconnected on data."); break;}
    }

    let elapsed = start.elapsed(); 
    println!("Total elapsed time is: {:?}.", elapsed);

    Ok(())
}


fn build_spim_data<T: TdcControl, W: SpimKind>(list: &mut W, data: &[u8], last_ci: &mut usize, settings: &Settings, line_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) {

    data.chunks_exact(8).for_each(|x| {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci as usize,
            _ => {
                let packet = PacketEELS { chip_index: *last_ci, data: x};
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
    //data.push(((index & 4_278_190_080)>>24) as u8);
    //data.push(((index & 16_711_680)>>16) as u8);
    //data.push(((index & 65_280)>>8) as u8);
    //data.push((index & 255) as u8);
 
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

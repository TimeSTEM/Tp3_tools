//!`spimlib` is a collection of tools to set hyperspectral EELS acquisition.

use crate::packetlib::{Packet, PacketEELS, packet_change};
use crate::auxiliar::{Settings, misc::TimepixRead};
use crate::tdclib::{TdcControl, PeriodicTdcRef, isi_box::{IsiBoxHand, IsiBoxType}};
use crate::errorlib::Tp3ErrorKind;
use std::time::Instant;
use std::io::Write;
use std::sync::mpsc;
use std::thread;
use crate::auxiliar::value_types::*;
use crate::constlib::*;
use rayon::prelude::*;

pub const VIDEO_TIME: TIME = 3200;
pub const SPIM_PIXELS: POSITION = 1025 + 200;
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
    type InputData;
    //type Output;

    fn data(&self) -> &Vec<Self::InputData>;
    fn add_electron_hit(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef);
    fn add_tdc_hit<T: TdcControl>(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef, ref_tdc: &mut T);
    fn upt_line(&self, packet: &PacketEELS, settings: &Settings, line_tdc: &mut PeriodicTdcRef);
    fn check(&self) -> bool;
    fn build_output(&mut self, set: &Settings, spim_tdc: &PeriodicTdcRef) -> &[u8];
    fn copy_empty(&mut self) -> Self;
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

//This recovers the position of the probe given the TDC and the electron ToA
#[inline]
pub fn get_positional_index(dt: TIME, spim_tdc: &PeriodicTdcRef, xspim: POSITION, yspim: POSITION) -> Option<POSITION> {
    let val = dt % spim_tdc.period;
    if val < spim_tdc.low_time {
        let mut r = (dt / spim_tdc.period) as POSITION; //how many periods -> which line to put.
        let rin = ((xspim as TIME * val) / spim_tdc.low_time) as POSITION; //Column correction. Maybe not even needed.
            
            if r > (yspim-1) {
                if r > 4096 {return None;} //This removes overflow electrons. See add_electron_hit
                r %= yspim;
            }
            
            let index = r * xspim + rin;
        
            Some(index)
        } else {
            None
        }
}

//This recovers the position of the probe given the TDC and the electron ToA
#[inline]
pub fn get_4d_complete_positional_index(number_of_masks: u8, mask: u8, dt: TIME, spim_tdc: &PeriodicTdcRef, xspim: POSITION, yspim: POSITION) -> Option<u32> {
    
    /*
    let mask = match mask {
        Some(val) => val,
        None => return None,
    };
    */
    
    let val = dt % spim_tdc.period;
    if val < spim_tdc.low_time {
        let mut r = (dt / spim_tdc.period) as POSITION; //how many periods -> which line to put.
        let rin = ((xspim as TIME * val) / spim_tdc.low_time) as POSITION; //Column correction. Maybe not even needed.
            
            if r > (yspim-1) {
                if r > 4096 {return None;} //This removes overflow electrons. See add_electron_hit
                r %= yspim;
            }
            
            let index = r * xspim + rin;
        
            Some( (index * number_of_masks as u32) + mask as u32)
        } else {
            None
        }
}


#[inline]
pub fn correct_or_not_etime(mut ele_time: TIME, line_tdc: &PeriodicTdcRef) -> TIME {
    if ele_time < line_tdc.begin_frame + VIDEO_TIME {
        let factor = (line_tdc.begin_frame + VIDEO_TIME - ele_time) / (line_tdc.period*line_tdc.ticks_to_frame.unwrap() as TIME) + 1;
        ele_time += line_tdc.period*line_tdc.ticks_to_frame.unwrap() as TIME * factor;
    }
    ele_time
}

///It outputs list of indices (max `u32`) that
///must be incremented. This is Hyperspectral Imaging
pub struct Live {
    data: Vec<(POSITION, TIME)>,
    data_out: Vec<u32>,
}

///It outputs a list of index to be used to reconstruct a channel in 4D STEM
pub struct Live4D<T> {
    data: Vec<(TIME, u8)>,
    data_out: Vec<u32>,
    channels: Vec<Live4DChannel<T>>,
    on_mask: [bool; MAX_CHANNELS],
}

///It outputs a frame-based to be used to reconstruct a channel in 4D STEM
pub struct LiveFrame4D<T> {
    data: Vec<(TIME, u8)>,
    data_out: Vec<u32>,
    channels: Vec<Live4DChannel<T>>,
    on_mask: [bool; MAX_CHANNELS],
}

macro_rules! impl_mask_selection {
    ($l: ty) => {
        impl $l {
            pub fn number_of_masks(&self) -> u8 {
                8
            }

            pub fn grab_mask<R: std::io::Read>(mut array: R) -> Result<Live4DChannel<u8>, Tp3ErrorKind> {
                let mut mask = [0_u8; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize];
                let mut total_size = 0;
                while let Ok(size) = array.read(&mut mask) {
                    total_size += size;
                }
                if total_size != (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize {
                    return Err(Tp3ErrorKind::STEM4DCouldNotSetMask);
                }
                println!("***4D STEM***: Mask received. Number of bytes read is {}.", total_size);
                Ok(Live4DChannel::new(mask))
            }

            pub fn create_mask<R: std::io::Read>(&mut self, array: R) -> Result<(), Tp3ErrorKind> {
                Ok(self.channels.push(LiveFrame4D::grab_mask(array)?))
            }
            
            pub fn replace_mask<R: std::io::Read>(&mut self, channel: usize, array: R) -> Result<(), Tp3ErrorKind> {
                Ok(self.channels[channel] = LiveFrame4D::grab_mask(array)?)
            }

            pub fn get_mask_values(&self, x:POSITION, y: POSITION) -> Option<u32> {
                let mut mask_value = 0u32;
                if !((x > DETECTOR_LIMITS.0.0) && (x < DETECTOR_LIMITS.0.1) && (y > DETECTOR_LIMITS.1.0) && (y < DETECTOR_LIMITS.1.1)) {
                    return None;
                }
                let index = y * DETECTOR_SIZE.0 + x;
                for (channel_number, channel) in self.channels.iter().enumerate() {
                    if channel.mask[index as usize] > 0 {
                        mask_value = mask_value | (1 << channel_number);
                    }
                }
                if mask_value == 0 { 
                    return None; 
                }
                Some(mask_value)
            }

            pub fn collect_mask_values(&mut self, x: POSITION, y: POSITION) {
                //let mut channel_vec = Vec::new();
                //let mut channel_vec: [u8; 8] = [0; 8];
                if !((x > DETECTOR_LIMITS.0.0) && (x < DETECTOR_LIMITS.0.1) && (y > DETECTOR_LIMITS.1.0) && (y < DETECTOR_LIMITS.1.1)) {
                    //return None;
                    return;
                }
                
                let index = |x: u32, y: u32| -> usize
                {
                    (y * DETECTOR_SIZE.0 + x) as usize
                };

                self.on_mask
                    .iter_mut()
                    .zip(self.channels.iter())
                    .for_each(|(mask, channel)| *mask = channel.mask[index(x, y)] > 0);

            }
        }
    }
}

impl_mask_selection!(Live4D<u8>);
impl_mask_selection!(LiveFrame4D<u8>);

pub struct Live4DChannel<T> {
    mask: [T; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize],
}

impl<T> Live4DChannel<T> {
    fn new(array: [T; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize]) -> Self {
        Self {mask: array}
    }
}

impl Live4DChannel<u8> {
    fn new_standard() -> Self {
        let array = [1; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize];
        Self {mask: array}
    }
    fn new_circle(center: (u32, u32), radius: u32, start_value: u8, value: u8) -> Self {
        let mut array = [start_value; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize];
        for x in 0..DETECTOR_SIZE.0 {
            for y in 0..DETECTOR_SIZE.1 {
                if (x as i32 - center.0 as i32) * (x as i32 - center.0 as i32) + (y as i32 - center.1 as i32) * (y as i32 - center.1 as i32) < (radius * radius) as i32 {
                    let index = (y * DETECTOR_SIZE.1 + x) as usize;
                    array[index] = value;
                }
            }
        }
        Self {mask:array}
    }
}

impl SpimKind for Live {
    type InputData = (POSITION, TIME);

    fn data(&self) -> &Vec<(POSITION, TIME)> {
        &self.data
    }
    #[inline]
    fn add_electron_hit(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef) {
        let ele_time = correct_or_not_etime(packet.electron_time(), line_tdc);
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
    fn build_output(&mut self, set: &Settings, spim_tdc: &PeriodicTdcRef) -> &[u8] {

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
        
        
        self.data_out = self.data.iter()
            .filter_map(|&(x, dt)| {
                get_spimindex(x, dt, spim_tdc, set.xspim_size, set.yspim_size)
            }).collect::<Vec<u32>>();
        
        /*
        let temp = &mut self.data_out;
        let my_vec = self.data.iter()
            .filter_map(|&(x, dt)| {
                get_spimindex(x, dt, spim_tdc, set.xspim_size, set.yspim_size)
            }).for_each(|x| temp.push(x));
        */
        
        as_bytes(&self.data_out)
    }
    fn clear(&mut self) {
        self.data.clear();
    }
    fn copy_empty(&mut self) -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8) , data_out: Vec::new()}
    }
    fn new() -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new()}
    }
}

impl SpimKind for Live4D<u8> {
    type InputData = (TIME, u8);

    fn data(&self) -> &Vec<(TIME, u8)> {
        &self.data
    }
    #[inline]
    fn add_electron_hit(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef) {
        let ele_time = correct_or_not_etime(packet.electron_time(), line_tdc);
        self.collect_mask_values(packet.x(), packet.y());


        let temp_data = &mut self.data;
        self.on_mask
            .iter()
            .enumerate()
            .filter(|(_channel_index, val)| **val == true)
            .for_each(|(channel_index, _val)| {
                temp_data.push((ele_time - line_tdc.begin_frame - VIDEO_TIME, channel_index as u8)); //This added the overflow.
            });

    }
    fn add_tdc_hit<T: TdcControl>(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef, ref_tdc: &mut T) {
        /*
        let tdc_time = packet.tdc_time_norm();
        ref_tdc.upt(tdc_time, packet.tdc_counter());
        if tdc_time > line_tdc.begin_frame + VIDEO_TIME {
            self.data.push((SPIM_PIXELS-1, tdc_time - line_tdc.begin_frame - VIDEO_TIME))
        }
        */
    }
    fn upt_line(&self, packet: &PacketEELS, _settings: &Settings, line_tdc: &mut PeriodicTdcRef) {
        line_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
    }
    fn check(&self) -> bool {
        self.data.get(0).is_some()
    }
    #[inline]
    fn build_output(&mut self, set: &Settings, spim_tdc: &PeriodicTdcRef) -> &[u8] {

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
        
        let number_of_masks = self.number_of_masks();
        let my_vec = self.data.iter()
            //.filter_map(|&(x, y, dt, channel)| {
            .filter_map(|&(dt, channel)| {
                get_4d_complete_positional_index(number_of_masks, channel, dt, spim_tdc, set.xspim_size, set.yspim_size)})
            .collect::<Vec<u32>>();

        as_bytes(&self.data_out)
    }
    fn clear(&mut self) {
        self.data.clear();
    }
    fn copy_empty(&mut self) -> Self {
        Live4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), channels: std::mem::take(&mut self.channels), on_mask: [false; MAX_CHANNELS]}
    }
    fn new() -> Self {
        Live4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), channels: vec![Live4DChannel::new_circle((128, 128), 8, 1, 0), Live4DChannel::new_circle((128, 128), 8, 0, 1)], on_mask: [false; MAX_CHANNELS]}
        //Live4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), channels: vec![Live4DChannel::new_standard()]}
    }
}

impl SpimKind for LiveFrame4D<u8> {
    type InputData = (TIME, u8);

    fn data(&self) -> &Vec<(TIME, u8)> {
        &self.data
    }
    #[inline]
    fn add_electron_hit(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef) {
        let ele_time = correct_or_not_etime(packet.electron_time(), line_tdc);
        self.collect_mask_values(packet.x(), packet.y());

        let temp_data = &mut self.data;
        self.on_mask
            .iter()
            .enumerate()
            .filter(|(_channel_index, val)| **val == true)
            .for_each(|(channel_index, _val)| {
                temp_data.push((ele_time - line_tdc.begin_frame - VIDEO_TIME, channel_index as u8)); //This added the overflow.
            });

    }
    fn add_tdc_hit<T: TdcControl>(&mut self, packet: &PacketEELS, line_tdc: &PeriodicTdcRef, ref_tdc: &mut T) {
        /*
        let tdc_time = packet.tdc_time_norm();
        ref_tdc.upt(tdc_time, packet.tdc_counter());
        if tdc_time > line_tdc.begin_frame + VIDEO_TIME {
            self.data.push((SPIM_PIXELS-1, tdc_time - line_tdc.begin_frame - VIDEO_TIME))
        }
        */
    }
    fn upt_line(&self, packet: &PacketEELS, _settings: &Settings, line_tdc: &mut PeriodicTdcRef) {
        line_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
    }
    fn check(&self) -> bool {
        self.data.get(0).is_some()
    }
    #[inline]
    fn build_output(&mut self, set: &Settings, spim_tdc: &PeriodicTdcRef) -> &[u8] {

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
        
        let number_of_masks = self.number_of_masks();
        let my_vec = self.data.iter()
            //.filter_map(|&(x, y, dt, channel)| {
            .filter_map(|&(dt, channel)| {
                get_4d_complete_positional_index(number_of_masks, channel, dt, spim_tdc, set.xspim_size, set.yspim_size)})
            .collect::<Vec<u32>>();

        as_bytes(&self.data_out)
    }
    fn clear(&mut self) {
        self.data.clear();
    }
    fn copy_empty(&mut self) -> Self {
        LiveFrame4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), channels: std::mem::take(&mut self.channels), on_mask: [false; MAX_CHANNELS]}
    }
    fn new() -> Self {
        LiveFrame4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), channels: vec![Live4DChannel::new_circle((128, 128), 8, 1, 0), Live4DChannel::new_circle((128, 128), 8, 0, 1)], on_mask: [false; MAX_CHANNELS]}
        //Live4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), channels: vec![Live4DChannel::new_standard()]}
    }
}

///Reads timepix3 socket and writes in the output socket a list of frequency followed by a list of unique indexes. First TDC must be a periodic reference, while the second can be nothing, periodic tdc or a non periodic tdc.
pub fn build_spim<V, T, W, U>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut spim_tdc: PeriodicTdcRef, mut ref_tdc: T, mut meas_type: W) -> Result<(), Tp3ErrorKind>
    where V: 'static + Send + TimepixRead,
          T: 'static + Send + TdcControl,
          W: 'static + Send + SpimKind,
          U: 'static + Send + Write,
{
    let (tx, rx) = mpsc::channel();
    let mut last_ci = 0;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let mut list = meas_type.copy_empty();
    
    //let mut current_read = 0;
    //let minimal_read = 512_000;

    thread::spawn(move || {
        while let Ok(size) = pack_sock.read_timepix(&mut buffer_pack_data) {
            build_spim_data(&mut list, &buffer_pack_data[0..size], &mut last_ci, &my_settings, &mut spim_tdc, &mut ref_tdc);
            //current_read += size;
            //if current_read > minimal_read {
            let list2 = list.copy_empty();
            if tx.send(list).is_err() {println!("Cannot send data over the thread channel."); break;}
            list = list2;
            //current_read = 0;
            //}
        }
    });
 
    let start = Instant::now();
    for mut tl in rx {
        let result = tl.build_output(&my_settings, &spim_tdc);
        if ns_sock.write(result).is_err() {println!("Client disconnected on data."); break;}
        //if ns_sock.write(as_bytes(&tl.build_output(&my_settings, &spim_tdc))).is_err() {println!("Client disconnected on data."); break;}
    }

    let elapsed = start.elapsed(); 
    println!("Total elapsed time is: {:?}.", elapsed);
    Ok(())
}

pub fn build_spim_isi<V, T, W, U>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut spim_tdc: PeriodicTdcRef, mut ref_tdc: T, mut meas_type: W, mut handler: IsiBoxType<Vec<u32>>) -> Result<(), Tp3ErrorKind>
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
    for mut tl in rx {
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
                let packet = PacketEELS { chip_index: *last_ci, data: packet_change(x)[0]};
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

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
//use rayon::prelude::*;

pub const VIDEO_TIME: TIME = 3200;
pub const SPIM_PIXELS: POSITION = 1025 + 200;
const BUFFER_SIZE: usize = 16384 * 2;

type MaskValues = i16;


///This is little endian
fn as_bytes<T>(v: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u8,
            v.len() * std::mem::size_of::<T>())
    }
}

///This is little endian
fn as_bytes_mut<T>(v: &mut [T]) -> &mut [u8] {
    unsafe {
        std::slice::from_raw_parts_mut(
            v.as_ptr() as *mut u8,
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
    fn build_output(&mut self, set: &Settings, spim_tdc: &PeriodicTdcRef) -> &[u8];
    fn copy_empty(&mut self) -> Self;
    fn is_ready(&mut self, line_tdc: &PeriodicTdcRef) -> bool;
    fn new(settings: &Settings) -> Self;
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
pub fn get_positional_index(channel: u8, dt: TIME, spim_tdc: &PeriodicTdcRef, xspim: POSITION, yspim: POSITION) -> Option<(POSITION, u8)> {
    let val = dt % spim_tdc.period;
    if val < spim_tdc.low_time {
        let mut r = (dt / spim_tdc.period) as POSITION; //how many periods -> which line to put.
        let rin = ((xspim as TIME * val) / spim_tdc.low_time) as POSITION; //Column correction. Maybe not even needed.
            
            if r > (yspim-1) {
                if r > 4096 {return None;} //This removes overflow electrons. See add_electron_hit
                r %= yspim;
            }
            
            let index = r * xspim + rin;
        
            Some((index, channel))
        } else {
            None
        }
}

//This recovers the position of the probe given the TDC and the electron ToA
#[inline]
pub fn get_4d_complete_positional_index(number_of_masks: u8, mask: u8, dt: TIME, spim_tdc: &PeriodicTdcRef, xspim: POSITION, yspim: POSITION) -> Option<u32> {
    
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

fn grab_mask<R: std::io::Read>(mut array: R) -> Result<Live4DChannelMask<MaskValues>, Tp3ErrorKind> {
    let mut mask = [0_i16; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize];
    let mut total_size = 0;
    //while let Ok(size) = array.read(&mut mask) {
    while let Ok(size) = array.read(as_bytes_mut(&mut mask)) {
        total_size += size;
    }
    if total_size != (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize {
        return Err(Tp3ErrorKind::STEM4DCouldNotSetMask);
    }
    println!("***4D STEM***: Mask received. Number of bytes read is {}.", total_size);
    Ok(Live4DChannelMask::new(mask))
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
    channels: Vec<Live4DChannelMask<T>>,
    on_mask: [T; MAX_CHANNELS],
}

///It outputs a frame-based to be used to reconstruct a channel in 4D STEM.
pub struct LiveFrame4D<T> {
    data: Vec<(TIME, u8, T)>,
    data_out: Vec<T>,
    scan_size: (POSITION, POSITION),
    channels: Vec<Live4DChannelMask<T>>,
    on_mask: [T; MAX_CHANNELS],
    debouncer: bool,
}


//Only method of this struct. Must be called to initialize the data_out array, as it is not
//a list-based output
impl LiveFrame4D<MaskValues> {
    pub fn create_data_channels(&mut self) {
        self.data_out = vec![0; (self.scan_size.0 * self.scan_size.1) as usize * self.number_of_masks() as usize];
    }
}

macro_rules! implement_mask_control {
    ($l: ty) => {
        impl $l {
            pub fn number_of_masks(&self) -> u8 {
                //self.channels.len() as u8
                2 as u8
            }
        
            pub fn create_mask<R: std::io::Read>(&mut self, array: R) -> Result<(), Tp3ErrorKind> {
                Ok(self.channels.push(grab_mask(array)?))
            }
        
            fn create_dummy_mask(&mut self, center: (u32, u32), radius: u32, start_value: MaskValues, value: MaskValues) -> Result<(), Tp3ErrorKind> {
                let mut array = [start_value; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize];
                for x in 0..DETECTOR_SIZE.0 {
                    for y in 0..DETECTOR_SIZE.1 {
                        if ((x + DETECTOR_LIMITS.0.0) as i32 - center.0 as i32) * ((x + DETECTOR_LIMITS.0.0) as i32 - center.0 as i32) + (y as i32 - center.1 as i32) * (y as i32 - center.1 as i32) < (radius * radius) as i32 {
                            let index = (y * DETECTOR_SIZE.1 + (x - DETECTOR_LIMITS.0.0)) as usize;
                            array[index] = value;
                        }
                    }
                }
                Ok(self.channels.push(Live4DChannelMask::new(array)))
            }

            /*
            fn replace_mask<R: std::io::Read>(&mut self, channel: usize, array: R) -> Result<(), Tp3ErrorKind> {
                Ok(self.channels[channel] = grab_mask(array)?)
            }
            */

            fn collect_mask_values(&mut self, x: POSITION, y: POSITION) {
                if !((x > DETECTOR_LIMITS.0.0) && (x < DETECTOR_LIMITS.0.1) && (y > DETECTOR_LIMITS.1.0) && (y < DETECTOR_LIMITS.1.1)) {
                    return;
                }
            
                let index = |x: u32, y: u32| -> usize
                {
                    (y * DETECTOR_SIZE.0 + (x - DETECTOR_LIMITS.0.0)) as usize
                };

                self.on_mask
                    .iter_mut()
                    .zip(self.channels.iter())
                    .for_each(|(mask, channel)| *mask = channel.mask[index(x, y)]);

            }
        }
    }
}

implement_mask_control!(Live4D<MaskValues>);
implement_mask_control!(LiveFrame4D<MaskValues>);

//T is mask data type
pub struct Live4DChannelMask<T> {
    mask: [T; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize],
}

impl<T> Live4DChannelMask<T> {
    fn new(array: [T; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize]) -> Self {
        Self {mask: array}
    }
}

impl SpimKind for Live {
    type InputData = (POSITION, TIME);

    fn data(&self) -> &Vec<Self::InputData> {
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
    fn is_ready(&mut self, _line_tdc: &PeriodicTdcRef) -> bool {
        true
    }
    fn copy_empty(&mut self) -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8) , data_out: Vec::new()}
    }
    fn new(_settings: &Settings) -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new()}
    }
}

impl SpimKind for Live4D<MaskValues> {
    type InputData = (TIME, u8);

    fn data(&self) -> &Vec<Self::InputData> {
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
            .filter(|(_channel_index, val)| **val != 0)
            .for_each(|(channel_index, _val)| {
                temp_data.push((ele_time - line_tdc.begin_frame - VIDEO_TIME, channel_index as u8)); //This added the overflow.
            });

    }
    fn add_tdc_hit<T: TdcControl>(&mut self, _packet: &PacketEELS, _line_tdc: &PeriodicTdcRef, _ref_tdc: &mut T) {
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
        self.data_out = self.data.iter()
            .filter_map(|&(dt, channel)| {
                get_4d_complete_positional_index(number_of_masks, channel, dt, spim_tdc, set.xspim_size, set.yspim_size)})
            .collect::<Vec<u32>>();

        as_bytes(&self.data_out)
    }
    fn is_ready(&mut self, _line_tdc: &PeriodicTdcRef) -> bool {
        true
    }
    fn copy_empty(&mut self) -> Self {
        Live4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), channels: std::mem::take(&mut self.channels), on_mask: [0; MAX_CHANNELS]}
    }
    fn new(_settings: &Settings) -> Self {
        let mut data_structure = Live4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), channels: Vec::new(), on_mask: [0; MAX_CHANNELS]};
        data_structure.create_dummy_mask((648, 148), 44, 0, 1).unwrap();
        data_structure.create_dummy_mask((648, 148), 44, 1, 0).unwrap();
        data_structure
    }
}

impl SpimKind for LiveFrame4D<MaskValues> {
    type InputData = (TIME, u8, MaskValues);

    fn data(&self) -> &Vec<Self::InputData> {
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
            .filter(|(_channel_index, val)| **val != 0)
            .for_each(|(channel_index, &val)| {
                temp_data.push((ele_time - line_tdc.begin_frame - VIDEO_TIME, channel_index as u8, val));
            });

    }
    fn add_tdc_hit<T: TdcControl>(&mut self, _packet: &PacketEELS, _line_tdc: &PeriodicTdcRef, _ref_tdc: &mut T) {
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
        let temp = &mut self.data_out;
        
         self.data.iter()
            .filter_map(|&(dt, channel, value)| {
                match get_4d_complete_positional_index(number_of_masks, channel, dt, spim_tdc, set.xspim_size, set.yspim_size) {
                    Some(index) => Some((index, value)),
                    None => None,
                }
            })
            .for_each(|(index, value)| {
                      temp[index as usize] += value});

        as_bytes(&self.data_out)
    }
    #[inline]
    fn is_ready(&mut self, line_tdc: &PeriodicTdcRef) -> bool {
        let is_new_frame = line_tdc.new_frame;
        if is_new_frame {
            if self.debouncer { 
                self.debouncer = false;
                return true
            }
        } else {
            self.debouncer = true;
        }
        false
    }
    fn copy_empty(&mut self) -> Self {
        let frame = LiveFrame4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: vec![0; (self.scan_size.0 * self.scan_size.1) as usize * self.number_of_masks() as usize], scan_size: self.scan_size, channels: std::mem::take(&mut self.channels), on_mask: [0; MAX_CHANNELS], debouncer: false};
        frame
    }
    fn new(settings: &Settings) -> Self {
        let mut frame = LiveFrame4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), scan_size: (settings.xspim_size, settings.yspim_size), channels: Vec::new(), on_mask: [0; MAX_CHANNELS], debouncer: false};
        frame.create_dummy_mask((648, 108), 44, 0, 1).unwrap();
        frame.create_dummy_mask((648, 108), 44, 1, 0).unwrap();
        frame.create_data_channels();
        frame
    }
}

///Reads timepix3 socket and writes in the output socket a list of frequency followed by a list of unique indexes. First TDC must be a periodic reference, while the second can be nothing, periodic tdc or a non periodic tdc.
pub fn build_spim<V, T, W, U>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut line_tdc: PeriodicTdcRef, mut ref_tdc: T, mut meas_type: W) -> Result<(), Tp3ErrorKind>
    where V: 'static + Send + TimepixRead,
          T: 'static + Send + TdcControl,
          W: 'static + Send + SpimKind,
          U: 'static + Send + Write,
{
    let (tx, rx) = mpsc::channel();
    let mut last_ci = 0;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    //let mut list = meas_type.copy_empty();
    
    thread::spawn(move || {
        while let Ok(size) = pack_sock.read_timepix(&mut buffer_pack_data) {
            build_spim_data(&mut meas_type, &buffer_pack_data[0..size], &mut last_ci, &my_settings, &mut line_tdc, &mut ref_tdc);
            if meas_type.is_ready(&line_tdc) {
               let list2 = meas_type.copy_empty();
                if tx.send(meas_type).is_err() {println!("Cannot send data over the thread channel."); break;}
                meas_type = list2;
            }
        }
    });
 
    let start = Instant::now();
    for mut tl in rx {
        let result = tl.build_output(&my_settings, &line_tdc);
        if ns_sock.write(result).is_err() {println!("Client disconnected on data."); break;}
        //if ns_sock.write(as_bytes(&tl.build_output(&my_settings, &spim_tdc))).is_err() {println!("Client disconnected on data."); break;}
    }

    let elapsed = start.elapsed(); 
    println!("Total elapsed time is: {:?}.", elapsed);
    Ok(())
}

pub fn build_spim_isi<V, T, W, U>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut line_tdc: PeriodicTdcRef, mut ref_tdc: T, mut meas_type: W, mut handler: IsiBoxType<Vec<u32>>) -> Result<(), Tp3ErrorKind>
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
            build_spim_data(&mut list, &buffer_pack_data[0..size], &mut last_ci, &my_settings, &mut line_tdc, &mut ref_tdc);
            if tx.send(list).is_err() {println!("Cannot send data over the thread channel."); break;}
            list = meas_type.copy_empty();
        }
    });
 
    let start = Instant::now();
    for mut tl in rx {
        let result = tl.build_output(&my_settings, &line_tdc);
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

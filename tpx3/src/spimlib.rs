//!`spimlib` is a collection of tools to set hyperspectral EELS acquisition.

use crate::packetlib::{Packet, PacketEELSInverted as Pack};
use crate::auxiliar::{Settings, misc::{TimepixRead, packet_change}};
use crate::tdclib::{TdcRef, isi_box::{IsiBoxHand, IsiBoxType}};
use crate::errorlib::Tp3ErrorKind;
use std::time::Instant;
use std::io::Write;
use std::sync::mpsc;
use std::thread;
use crate::auxiliar::value_types::*;
use crate::constlib::*;
//use rayon::prelude::*;

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
    
    fn data(&self) -> &Vec<Self::InputData>;
    fn add_electron_hit(&mut self, packet: &Pack, line_tdc: &TdcRef, set: &Settings);
    fn add_tdc_hit(&mut self, packet: &Pack, line_tdc: &TdcRef, ref_tdc: &mut TdcRef);
    fn upt_line(&self, packet: &Pack, settings: &Settings, line_tdc: &mut TdcRef);
    fn build_output(&mut self, set: &Settings, spim_tdc: &TdcRef, list_scan: SlType) -> &[u8];
    fn copy_empty(&mut self) -> Self;
    fn is_ready(&mut self, line_tdc: &TdcRef) -> bool;
    fn new(settings: &Settings) -> Self;
}

#[inline]
pub fn get_return_spimindex(x: POSITION, dt: TIME, spim_tdc: &TdcRef, xspim: POSITION, yspim: POSITION) -> Option<INDEX_HYPERSPEC> {
    Some(spim_tdc.get_return_positional_index(dt, xspim, yspim)? * SPIM_PIXELS + x)
}

#[inline]
pub fn get_spimindex(x: POSITION, dt: TIME, spim_tdc: &TdcRef, xspim: POSITION, yspim: POSITION, list_scan: SlType) -> Option<INDEX_HYPERSPEC> {
    Some(spim_tdc.get_positional_index(dt, xspim, yspim, list_scan)? * SPIM_PIXELS + x)
}

#[inline]
pub fn get_4dindex(x: POSITION, y: POSITION, dt: TIME, spim_tdc: &TdcRef, xspim: POSITION, yspim: POSITION) -> Option<INDEX_4D> {
    Some(spim_tdc.get_positional_index(dt, xspim, yspim, None)? as INDEX_4D * (RAW4D_PIXELS_X * RAW4D_PIXELS_Y) as INDEX_4D + (y * RAW4D_PIXELS_X + x)as INDEX_4D)
}

#[inline]
pub fn get_return_4dindex(x: POSITION, y: POSITION, dt: TIME, spim_tdc: &TdcRef, xspim: POSITION, yspim: POSITION) -> Option<INDEX_4D> {
    Some(spim_tdc.get_return_positional_index(dt, xspim, yspim)? as u64 * (RAW4D_PIXELS_X * RAW4D_PIXELS_Y) as u64 + (y * RAW4D_PIXELS_X + x)as u64)
}


/*
//This recovers the position of the probe given the TDC and the electron ToA. dT is the time from
//the last frame begin
#[inline]
fn get_positional_index(dt: TIME, spim_tdc: &TdcRef, xspim: POSITION, yspim: POSITION, list_scan: SlType) -> Option<POSITION> {
    if UNIFORM_PIXEL {
        let mut index = (dt * xspim as TIME / spim_tdc.period()?) as POSITION;
        if index > xspim * yspim {
            index %= xspim * yspim
        }
        if let Some(custom_list) = list_scan {
            custom_list.get(index as usize).copied()
        } else {
            Some( index )
        }
    } else {
        let determ = |dt: TIME, dt_partial: TIME, period: TIME, xspim: POSITION, low_time: TIME, yspim: POSITION| {
            let mut r = (dt / period) as POSITION; //how many periods -> which line to put.
            let rin = ((xspim as TIME * dt_partial) / low_time) as POSITION; //Column correction. Maybe not even needed.
                
            if r > (yspim-1) {
                if r > 4096 {return None;} //This removes overflow electrons. See add_electron_hit
                r %= yspim;
            }
                
            let index = r * xspim + rin;
            Some(index)
        };

        if REMOVE_RETURN {
            let val = dt % spim_tdc.period()?;
            if val < spim_tdc.low_time()? {
                determ(dt, val, spim_tdc.period()?, xspim, spim_tdc.low_time()?, yspim)
            } else {
                None
            }
        } else {
            let val = dt % spim_tdc.period()?;
            determ(dt, val, spim_tdc.period()?, xspim, spim_tdc.low_time()?, yspim)
        }
    }
}

//This recovers the position of the probe during the return given the TDC and the electron ToA
#[inline]
fn get_return_positional_index(dt: TIME, spim_tdc: &TdcRef, xspim: POSITION, yspim: POSITION) -> Option<POSITION> {
    let val = dt % spim_tdc.period()?;
    if val >= spim_tdc.low_time()? {
        let mut r = (dt / spim_tdc.period()?) as POSITION; //how many periods -> which line to put.
        let rin = ((xspim as TIME * (val - spim_tdc.low_time()?)) / spim_tdc.high_time()?) as POSITION; //Column correction. Maybe not even needed.
            
        if r > (yspim-1) {
            if r > 4096 {return None;} //This removes overflow electrons. See add_electron_hit
            r %= yspim;
        }
        
        let index = r * xspim + rin;
    
        Some( index )
    } else {
        None
    }
}
*/


/*
#[inline]
pub fn correct_or_not_etime(mut ele_time: TIME, line_tdc: &TdcRef) -> TIME {
    if SYNC_MODE == 0 {
        if ele_time < line_tdc.begin_frame() + VIDEO_TIME {
            let factor = (line_tdc.begin_frame() + VIDEO_TIME - ele_time) / (line_tdc.period()*line_tdc.ticks_to_frame().unwrap() as TIME) + 1;
            ele_time += line_tdc.period()*line_tdc.ticks_to_frame().unwrap() as TIME * factor;
        }
        ele_time - line_tdc.begin_frame() - VIDEO_TIME
    } else {
        if ele_time < line_tdc.time() + VIDEO_TIME {
            let factor = (line_tdc.time() + VIDEO_TIME - ele_time) / (line_tdc.period()) + 1;
            ele_time += line_tdc.period() * factor * line_tdc.current_line().unwrap() as u64;
        }
        ele_time - line_tdc.begin_frame() - VIDEO_TIME
    }
}
*/

///It outputs list of indices (max `u32`) that
///must be incremented. This is Hyperspectral Imaging
pub struct Live {
    data: Vec<(POSITION, TIME)>,
    data_out: Vec<INDEX_HYPERSPEC>,
    _timer: Instant,
}

///It outputs list of indices (max `u32`) that
///must be incremented. This is Hyperspectral Imaging
pub struct LiveScanList {
    data: Vec<(POSITION, TIME)>,
    data_out: Vec<INDEX_HYPERSPEC>,
    _timer: Instant,
}

impl LiveScanList {
    pub fn create_list<R: std::io::Read>(mut array: R, settings: &Settings) -> Result<Vec<POSITION>, Tp3ErrorKind> {
        let mut list_scan: Vec<POSITION> = vec![0; (settings.xspim_size * settings.yspim_size) as usize];
        array.read_exact(as_bytes_mut(&mut list_scan))?;
        Ok(list_scan)
    }
}

///It outputs a list of indices that must
///be increment. This is Hyperspectral imaging with
///coincident photons around
pub struct LiveCoincidence {
    data: Vec<(POSITION, TIME)>,
    aux_data: [TIME; LIST_SIZE_AUX_EVENTS],
    data_out: Vec<INDEX_HYPERSPEC>,
    _timer: Instant,
}

///It outputs a list of indices that must
///be increment. This is Hyperspectral imaging with
///coincident photons around
pub struct Live4D {
    data: Vec<(POSITION, TIME)>,
    data_out: Vec<INDEX_4D>,
    _timer: Instant,
}

///It outputs a frame-based to be used to reconstruct a channel in 4D STEM.
pub struct LiveFrame4D<T> {
    data: Vec<(POSITION, TIME)>,
    data_out: Vec<T>,
    com: (T, T),
    scan_size: (POSITION, POSITION),
    channels: Vec<Live4DChannelMask<T>>,
    debouncer: bool,
    timer: Instant,
}

//Only method of this struct. Must be called to initialize the data_out array, as it is not
//a list-based output
impl LiveFrame4D<MaskValues> {
    fn create_data_channels(&mut self) {
        self.data_out = vec![0; (self.scan_size.0 * self.scan_size.1) as usize * self.number_of_masks() as usize];
    }
    fn number_of_masks(&self) -> u8 {
        //self.channels.len() as u8
        4_u8
    }
    fn create_mask<R: std::io::Read>(&mut self, array: R) -> Result<(), Tp3ErrorKind> {
        //Ok(self.channels.push(grab_mask(array)?))
        self.channels = Live4DChannelMask::grab_mask(array)?;
        Ok(())
    }
}

//T is mask data type
struct Live4DChannelMask<T> {
    mask: [T; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize],
}

impl Live4DChannelMask<MaskValues> {
    fn new(array: [MaskValues; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize]) -> Self {
        Self {mask: array}
    }
    fn grab_mask<R: std::io::Read>(mut array: R) -> Result<Vec<Self>, Tp3ErrorKind> {
        let mut vec_of_channels: Vec<Self> = Vec::new();
        let mut mask = [0; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize];
        while array.read_exact(as_bytes_mut(&mut mask)).is_ok() {
            vec_of_channels.push(Live4DChannelMask::new(mask));
        }
        Ok(vec_of_channels)
    }
}

impl SpimKind for Live {
    type InputData = (POSITION, TIME);

    fn data(&self) -> &Vec<Self::InputData> {
        &self.data
    }
    #[inline]
    fn add_electron_hit(&mut self, packet: &Pack, line_tdc: &TdcRef, _set: &Settings) {
        let ele_time = line_tdc.correct_or_not_etime(packet.electron_time()).unwrap();
        self.data.push((packet.x(), ele_time)); //This added the overflow.
    }
    fn add_tdc_hit(&mut self, packet: &Pack, line_tdc: &TdcRef, ref_tdc: &mut TdcRef) {
        let tdc_time = packet.tdc_time_norm();
        ref_tdc.upt(tdc_time, packet.tdc_counter());
        if tdc_time > line_tdc.begin_frame() + VIDEO_TIME {
            self.data.push((SPIM_PIXELS-1, tdc_time - line_tdc.begin_frame() - VIDEO_TIME))
        }
    }
    fn upt_line(&self, packet: &Pack, _settings: &Settings, line_tdc: &mut TdcRef) {
        line_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
    }
    #[inline]
    fn build_output(&mut self, set: &Settings, spim_tdc: &TdcRef, _list_scan: SlType) -> &[u8] {

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
                get_spimindex(x, dt, spim_tdc, set.xspim_size, set.yspim_size, None)
            }).collect::<Vec<POSITION>>();

        /*
        let temp = &mut self.data_out;
        let my_vec = self.data.iter()
            .filter_map(|&(x, dt)| {
                get_spimindex(x, dt, spim_tdc, set.xspim_size, set.yspim_size)
            }).for_each(|x| temp.push(x));
        */
        
        as_bytes(&self.data_out)
    }
    fn is_ready(&mut self, _line_tdc: &TdcRef) -> bool {
        true
    }
    fn copy_empty(&mut self) -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8) , data_out: Vec::new(), _timer: Instant::now()}
    }
    fn new(_settings: &Settings) -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), _timer: Instant::now()}
    }
}

impl SpimKind for LiveScanList {
    type InputData = (POSITION, TIME);

    fn data(&self) -> &Vec<Self::InputData> {
        &self.data
    }
    #[inline]
    fn add_electron_hit(&mut self, packet: &Pack, line_tdc: &TdcRef, _set: &Settings) {
        let ele_time = line_tdc.correct_or_not_etime(packet.electron_time()).unwrap();
        self.data.push((packet.x(), ele_time)); //This added the overflow.
    }
    fn add_tdc_hit(&mut self, packet: &Pack, line_tdc: &TdcRef, ref_tdc: &mut TdcRef) {
        let tdc_time = packet.tdc_time_norm();
        ref_tdc.upt(tdc_time, packet.tdc_counter());
        if tdc_time > line_tdc.begin_frame() + VIDEO_TIME {
            self.data.push((SPIM_PIXELS-1, tdc_time - line_tdc.begin_frame() - VIDEO_TIME))
        }
    }
    fn upt_line(&self, packet: &Pack, _settings: &Settings, line_tdc: &mut TdcRef) {
        line_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
    }
    #[inline]
    fn build_output(&mut self, set: &Settings, spim_tdc: &TdcRef, list_scan: SlType) -> &[u8] {

        self.data_out = self.data.iter()
            .filter_map(|&(x, dt)| {
                get_spimindex(x, dt, spim_tdc, set.xspim_size, set.yspim_size, list_scan)
            }).collect::<Vec<POSITION>>();

        as_bytes(&self.data_out)
    }
    fn is_ready(&mut self, _line_tdc: &TdcRef) -> bool {
        true
    }
    fn copy_empty(&mut self) -> Self {
        LiveScanList{ data: Vec::with_capacity(BUFFER_SIZE / 8) , data_out: Vec::new(), _timer: Instant::now()}
    }
    fn new(_settings: &Settings) -> Self {
        LiveScanList{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), _timer: Instant::now()}
    }
}

impl SpimKind for LiveCoincidence {
    type InputData = (POSITION, TIME);

    fn data(&self) -> &Vec<Self::InputData> {
        &self.data
    }
    #[inline]
    fn add_electron_hit(&mut self, packet: &Pack, line_tdc: &TdcRef, set: &Settings) {
        let ele_time = line_tdc.correct_or_not_etime(packet.electron_time()).unwrap();
        for phtime in self.aux_data.iter() {
            if (*phtime < ele_time + set.time_delay + set.time_width) && (ele_time + set.time_delay < *phtime + set.time_width) {
                self.data.push((packet.x(), ele_time)); //This added the overflow.
            }
        }
    }
    fn add_tdc_hit(&mut self, packet: &Pack, _line_tdc: &TdcRef, ref_tdc: &mut TdcRef) {
        let tdc_time = packet.tdc_time_norm();
        ref_tdc.upt(tdc_time, packet.tdc_counter());
        for index in 0..LIST_SIZE_AUX_EVENTS-1 {
            self.aux_data[index+1] = self.aux_data[index];
        }
        self.aux_data[0] = packet.tdc_time_norm();
    }
    fn upt_line(&self, packet: &Pack, _settings: &Settings, line_tdc: &mut TdcRef) {
        line_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
    }
    #[inline]
    fn build_output(&mut self, set: &Settings, spim_tdc: &TdcRef, _list_scan: SlType) -> &[u8] {

        self.data_out = self.data.iter()
            .filter_map(|&(x, dt)| {
                get_spimindex(x, dt, spim_tdc, set.xspim_size, set.yspim_size, None)
            }).collect::<Vec<POSITION>>();
        
        as_bytes(&self.data_out)
    }
    fn is_ready(&mut self, _line_tdc: &TdcRef) -> bool {
        true
    }
    fn copy_empty(&mut self) -> Self {
        LiveCoincidence{ data: Vec::with_capacity(BUFFER_SIZE / 8) , aux_data: [0; LIST_SIZE_AUX_EVENTS], data_out: Vec::new(), _timer: Instant::now()}
    }
    fn new(_settings: &Settings) -> Self {
        LiveCoincidence{ data: Vec::with_capacity(BUFFER_SIZE / 8), aux_data: [0; LIST_SIZE_AUX_EVENTS], data_out: Vec::new(), _timer: Instant::now()}
    }
}

impl SpimKind for Live4D {
    type InputData = (POSITION, TIME);

    fn data(&self) -> &Vec<Self::InputData> {
        &self.data
    }
    #[inline]
    fn add_electron_hit(&mut self, packet: &Pack, line_tdc: &TdcRef, _set: &Settings) {
        let ele_time = line_tdc.correct_or_not_etime(packet.electron_time());
        self.data.push(((packet.x() << 16) + (packet.y() & 65535), ele_time.unwrap())); //This added the overflow.
    }
    fn add_tdc_hit(&mut self, packet: &Pack, line_tdc: &TdcRef, ref_tdc: &mut TdcRef) {
        let tdc_time = packet.tdc_time_norm();
        ref_tdc.upt(tdc_time, packet.tdc_counter());
        if tdc_time > line_tdc.begin_frame() + VIDEO_TIME {
            self.data.push((SPIM_PIXELS-1, tdc_time - line_tdc.begin_frame() - VIDEO_TIME))
        }
    }
    fn upt_line(&self, packet: &Pack, _settings: &Settings, line_tdc: &mut TdcRef) {
        line_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
    }
    #[inline]
    fn build_output(&mut self, set: &Settings, spim_tdc: &TdcRef, _list_scan: SlType) -> &[u8] {

        self.data_out = self.data.iter()
            .filter_map(|&(x_y, dt)| {
                get_4dindex(x_y >> 16, x_y & 65535, dt, spim_tdc, set.xspim_size, set.yspim_size)
            }).collect::<Vec<u64>>();
        
        as_bytes(&self.data_out)
    }
    fn is_ready(&mut self, _line_tdc: &TdcRef) -> bool {
        true
    }
    fn copy_empty(&mut self) -> Self {
        Live4D{ data: Vec::with_capacity(BUFFER_SIZE / 8) , data_out: Vec::new(), _timer: Instant::now()}
    }
    fn new(_settings: &Settings) -> Self {
        Live4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), _timer: Instant::now()}
    }
}

impl SpimKind for LiveFrame4D<MaskValues> {
    type InputData = (POSITION, TIME);

    fn data(&self) -> &Vec<Self::InputData> {
        &self.data
    }
    #[inline]
    fn add_electron_hit(&mut self, packet: &Pack, line_tdc: &TdcRef, _set: &Settings) {
        let ele_time = line_tdc.correct_or_not_etime(packet.electron_time());
        self.data.push(((packet.x() << 16) + (packet.y() & 65535), ele_time.unwrap())); //This added the overflow.

    }
    fn add_tdc_hit(&mut self, _packet: &Pack, _line_tdc: &TdcRef, _ref_tdc: &mut TdcRef) {
    }
    fn upt_line(&self, packet: &Pack, _settings: &Settings, line_tdc: &mut TdcRef) {
        line_tdc.upt(packet.tdc_time_norm(), packet.tdc_counter());
    }
    #[inline]
    fn build_output(&mut self, set: &Settings, spim_tdc: &TdcRef, _list_scan: SlType) -> &[u8] {

        let channel_array_index = |x: POSITION, y: POSITION| -> usize
        {
            (y * DETECTOR_SIZE.0 + (x - DETECTOR_LIMITS.0.0)) as usize
        };
        let is_inside = |x: POSITION, y: POSITION| -> bool {
            (x > DETECTOR_LIMITS.0.0) && (x < DETECTOR_LIMITS.0.1) && (y > DETECTOR_LIMITS.1.0) && (y < DETECTOR_LIMITS.1.1)
        };

        let mut frequency = vec![0i16; (set.xspim_size * set.yspim_size) as usize];
        let number_of_masks = self.number_of_masks() as POSITION;
        let com = self.com;
        let temp = &mut self.data_out;
        let temp2 = &self.channels;


        self.data
            .iter()
            .filter_map(|&(x_y, dt)| {
                match spim_tdc.get_positional_index(dt, set.xspim_size, set.yspim_size, None) {
                    Some(index) => Some((x_y >> 16, x_y & 65535, index)),
                    None => None,
                }
            })
            .filter(|&(x, y, _index)| is_inside(x, y))
            .for_each(|(x, y, index)| {
                frequency[index as usize] += 1;
                temp[(index * number_of_masks) as usize + 0] += x as i16 - com.0;
                temp[(index * number_of_masks) as usize + 1] += y as i16 - com.1;
                for (channel_number, channel) in temp2.iter().enumerate() {
                    let value = channel.mask[channel_array_index(x, y)];
                    temp[(index * number_of_masks) as usize + channel_number+2] += value;
                }
                  
            });
        
        self.data_out
            .chunks_exact_mut(number_of_masks as usize)
            .zip(frequency.iter())
            .for_each(|(chunk, frequency)| {
                if *frequency > 0 {
                    chunk[0] /= frequency;
                    chunk[1] /= frequency;
                }
            });

        as_bytes(&self.data_out)
    }
    #[inline]
    fn is_ready(&mut self, line_tdc: &TdcRef) -> bool {
        let is_new_frame = line_tdc.new_frame();
        if is_new_frame {
            if self.debouncer { 
                self.debouncer = false;
                if self.timer.elapsed().as_millis() < TIME_INTERVAL_4DFRAMES {
                    self.data.clear();
                    return false
                }
                return true
            }
        } else {
            self.debouncer = true;
        }
        false
    }
    fn copy_empty(&mut self) -> Self {
        let mut frame = LiveFrame4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: vec![0; (self.scan_size.0 * self.scan_size.1) as usize * self.number_of_masks() as usize], com: self.com, scan_size: self.scan_size, channels: Vec::new(), debouncer: false, timer: Instant::now()};
        let file = std::fs::File::open(MASK_FILE).unwrap();
        frame.create_mask(file).unwrap();
        frame.create_data_channels();
        frame
    }
    fn new(settings: &Settings) -> Self {
        let mut frame = LiveFrame4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), com: (0, 0), scan_size: (settings.xspim_size, settings.yspim_size), channels: Vec::new(), debouncer: false, timer: Instant::now()};
        let file = std::fs::File::open(MASK_FILE).unwrap();
        frame.create_mask(file).unwrap();
        frame.create_data_channels();
        frame
    }
}

///Reads timepix3 socket and writes in the output socket a list of frequency followed by a list of unique indexes. First TDC must be a periodic reference, while the second can be nothing, periodic tdc or a non periodic tdc.
pub fn build_spim<V, W, U>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut line_tdc: TdcRef, mut ref_tdc: TdcRef, mut meas_type: W, scan_list: SlType) -> Result<(), Tp3ErrorKind>
    where V: 'static + Send + TimepixRead,
          W: 'static + Send + SpimKind,
          U: 'static + Send + Write,
{
    let (tx, rx) = mpsc::channel();
    let mut last_ci = 0;
    let mut buffer_pack_data = [0; BUFFER_SIZE];

    let mut file_to_write = my_settings.create_file();
    thread::spawn(move || {
        while let Ok(size) = pack_sock.read_timepix(&mut buffer_pack_data) {
            if let Some(file) = &mut file_to_write {
                file.write(&buffer_pack_data[0..size]).unwrap();
            }
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
        let result = tl.build_output(&my_settings, &line_tdc, scan_list);
        if ns_sock.write(result).is_err() {println!("Client disconnected on data."); break;}
    }

    let elapsed = start.elapsed(); 
    println!("Total elapsed time is: {:?}.", elapsed);
    Ok(())
}

pub fn build_spim_isi<V, W, U>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut line_tdc: TdcRef, mut ref_tdc: TdcRef, mut meas_type: W, mut handler: IsiBoxType<Vec<u32>>) -> Result<(), Tp3ErrorKind>
    where V: 'static + Send + TimepixRead,
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
        let result = tl.build_output(&my_settings, &line_tdc, None);
        let x = handler.get_data();
        if ns_sock.write(as_bytes(result)).is_err() {println!("Client disconnected on data."); break;}
        if x.len() > 0 {
            if ns_sock.write(as_bytes(&x)).is_err() {println!("Client disconnected on data."); break;}
        }
    }

    handler.stop_threads();
    let elapsed = start.elapsed(); 
    println!("Total elapsed time is: {:?}.", elapsed);
    Ok(())
}

fn build_spim_data<W: SpimKind>(list: &mut W, data: &[u8], last_ci: &mut u8, settings: &Settings, line_tdc: &mut TdcRef, ref_tdc: &mut TdcRef) {
    data.chunks_exact(8).for_each(|x| {
        match *x {
            [84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
            _ => {
                let packet = Pack { chip_index: *last_ci, data: packet_change(x)[0]};
                let id = packet.id();
                match id {
                    11 => {
                        list.add_electron_hit(&packet, line_tdc, settings);
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

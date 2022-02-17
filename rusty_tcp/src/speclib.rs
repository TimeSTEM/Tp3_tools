///`modes` is a module containing tools to live acquire frames and spectral images.
use crate::packetlib::{Packet, PacketEELS as Pack};
use crate::auxiliar::{Settings, misc::TimepixRead};
use crate::tdclib::{TdcControl, PeriodicTdcRef};
use crate::errorlib::Tp3ErrorKind;
use std::time::Instant;
use std::io::Write;
use std::convert::TryInto;
//use rayon::prelude::*;
use core::ops::Add;


fn as_bytes<T>(v: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u8,
            v.len() * std::mem::size_of::<T>())
    }
}


pub trait BitDepth {}

impl BitDepth for u32 {}

pub trait Sup: BitDepth + Clone + std::ops::Add<Output = Self> + Copy {
    fn zero() -> Self;
    fn one() -> Self;
    fn ten() -> Self;
}

impl Sup for u32 {
    fn zero() -> u32 {
        0
    }
    fn one() -> u32 {
        1
    }
    fn ten() -> u32 {
        10
    }
}


const CAM_DESIGN: (usize, usize) = Pack::chip_array();
const BUFFER_SIZE: usize = 16384 * 2;
const SR_TIME: usize = 10_000; //Time window (10_000 -> 10 us);
const SR_INDEX: usize = 64; //Maximum x index value to account in the average calculation;
const SR_MIN: usize = 0; //Minimum array size to perform the average in super resolution;
const TILT_FRACTION: usize = 16; //Values with y = 256 will be tilted by 256 / 16;

pub trait SpecKind {

    fn is_ready(&self) -> bool;
    fn build_output(&self) -> &[u8];
    fn new(settings: &Settings) -> Self;
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, frame_tdc: &PeriodicTdcRef, ref_tdc: &T);
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, ref_tdc: &mut T);
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, _settings: &Settings);
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings);
}

pub struct Live2D;
pub struct Live1D;
pub struct LiveTR2D;
pub struct LiveTR1D;
pub struct LiveTilted2D;
pub struct FastChrono;
pub struct Chrono;
pub struct SuperResolution;

pub struct SpecMeasurement<T, K: Sup> {
    data: Vec<K>,
    aux_data: Vec<usize>,
    is_ready: bool,
    global_stop: bool,
    last_time: usize,
    last_mean: Option<usize>,
    _kind: T,
}

impl<L: Sup> SpecKind for SpecMeasurement<Live2D, L> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len: usize = CAM_DESIGN.1*settings.bytedepth*CAM_DESIGN.0;
        let mut temp_vec: Vec<L> = vec![L::zero(); len + 1];
        temp_vec[len] = L::ten();
        SpecMeasurement{ data: temp_vec, aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: Live2D }
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &T) {
        let index = pack.x() + CAM_DESIGN.0 * pack.y();
        self.data[index] = self.data[index] + L::one();
    }
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        self.data[1024] = self.data[1024] + L::one();
        //append_to_array(&mut self.data, CAM_DESIGN.0-1, settings.bytedepth);
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, _settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = true;
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings) {
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = L::zero());
            *self.data.iter_mut().last().expect("SpecKind: Last value is none.") = L::ten();
        }
    }
}

impl SpecKind for SpecMeasurement<Live1D, u32> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len: usize = settings.bytedepth*CAM_DESIGN.0;
        let mut temp_vec = vec![0; len + 1];
        temp_vec[len] = 10;
        SpecMeasurement{ data: temp_vec, aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: Live1D}
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &T) {
        let index = pack.x();
        //append_to_array(&mut self.data, index, settings.bytedepth);
    }
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        //append_to_array(&mut self.data, CAM_DESIGN.0-1, settings.bytedepth);
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, _settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = true;
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings) {
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = 0);
            *self.data.iter_mut().last().expect("SpecKind: Last value is none.") = 10;
        }
    }
}

impl SpecKind for SpecMeasurement<LiveTR2D, u32> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len: usize = CAM_DESIGN.1*settings.bytedepth*CAM_DESIGN.0;
        let mut temp_vec = vec![0; len + 1];
        temp_vec[len] = 10;
        SpecMeasurement{ data: temp_vec, aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: LiveTR2D}
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, _frame_tdc: &PeriodicTdcRef, ref_tdc: &T) {
        if LiveTR1D::tr_check_if_in(pack.electron_time(), ref_tdc, settings) {
            let index = pack.x() + CAM_DESIGN.0 * pack.y();
            //append_to_array(&mut self.data, index, settings.bytedepth);
        }
    }
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, _settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = true;
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings) {
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = 0);
            *self.data.iter_mut().last().expect("SpecKind: Last value is none.") = 10;
        }
    }
}

impl SpecKind for SpecMeasurement<LiveTR1D, u32> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len: usize = settings.bytedepth*CAM_DESIGN.0;
        let mut temp_vec = vec![0; len + 1];
        temp_vec[len] = 10;
        SpecMeasurement{ data: temp_vec, aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: LiveTR1D}
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, _frame_tdc: &PeriodicTdcRef, ref_tdc: &T) {
        if LiveTR1D::tr_check_if_in(pack.electron_time(), ref_tdc, settings) {
            let index = pack.x();
            //append_to_array(&mut self.data, index, settings.bytedepth);
        }
    }
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, _settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = true;
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings) {
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = 0);
            *self.data.iter_mut().last().expect("SpecKind: Last value is none.") = 10;
        }
    }
}

impl SpecKind for SpecMeasurement<LiveTilted2D, u32> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len: usize = CAM_DESIGN.1*settings.bytedepth*CAM_DESIGN.0;
        let mut temp_vec = vec![0; len + 1];
        temp_vec[len] = 10;
        SpecMeasurement{ data: temp_vec, aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: LiveTilted2D }
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &T) {
        let x = pack.x();
        let y = pack.y();
        let index = x + CAM_DESIGN.0 * y;
        //append_to_array(&mut self.data, index, settings.bytedepth);
    }
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        //append_to_array(&mut self.data, CAM_DESIGN.0-1, settings.bytedepth);
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, _settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = true;
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings) {
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = 0);
            *self.data.iter_mut().last().expect("SpecKind: Last value is none.") = 10;
        }
    }
}

impl SpecKind for SpecMeasurement<FastChrono, u32> {
    fn is_ready(&self) -> bool {
        self.is_ready && !self.global_stop
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len: usize = settings.xspim_size*settings.bytedepth*CAM_DESIGN.0;
        let mut temp_vec = vec![0; len + 1];
        temp_vec[len] = 10;
        SpecMeasurement{ data: temp_vec, aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: FastChrono}
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, frame_tdc: &PeriodicTdcRef, _ref_tdc: &T) {
        let line = frame_tdc.counter()/2;
        let index = pack.x() + line * CAM_DESIGN.0;
        if line < settings.xspim_size {
            //append_to_array(&mut self.data, index, settings.bytedepth);
        }
    }
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        //append_to_array(&mut self.data, CAM_DESIGN.0-1, settings.bytedepth);
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = (frame_tdc.counter()/2) > settings.xspim_size;
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, _settings: &Settings) {
        self.global_stop = true;
    }
}

impl SpecKind for SpecMeasurement<Chrono, u32> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len: usize = settings.xspim_size*settings.bytedepth*CAM_DESIGN.0;
        let mut temp_vec = vec![0; len + 1];
        temp_vec[len] = 10;
        SpecMeasurement{ data: temp_vec, aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: Chrono}
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, frame_tdc: &PeriodicTdcRef, _ref_tdc: &T) {
        let line = (frame_tdc.counter()/2) % settings.xspim_size;
        let index = pack.x() + line * CAM_DESIGN.0;
        //append_to_array(&mut self.data, index, settings.bytedepth);
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        let line = frame_tdc.counter() / 2;
        self.is_ready = line % 20 == 0; //Every 20 lines send chrono;
        if line % settings.xspim_size == 0 {
            self.aux_data.push(0); //This indicates the frame must be refreshed;
        }
    }
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        //append_to_array(&mut self.data, CAM_DESIGN.0-1, settings.bytedepth);
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, _settings: &Settings) {
        self.is_ready = false;
        if self.aux_data.len() > 0 { //Refresh frame if true;
            self.aux_data.pop(); //Remove for the next cycle;
            self.data.iter_mut().for_each(|x| *x = 0);
            *self.data.iter_mut().last().expect("SpecKind: Last value is none.") = 10;
        }
    }
}

impl SpecKind for SpecMeasurement<SuperResolution, u32> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
       as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len: usize = settings.bytedepth*CAM_DESIGN.0;
        let mut temp_vec = vec![0; len + 1];
        temp_vec[len] = 10;
        SpecMeasurement{ data: temp_vec, aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: SuperResolution}
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &T) {
        let index = pack.x();
        self.aux_data.push(index);
        
        let new_time = pack.fast_electron_time();
        if new_time > self.last_time + SR_TIME {
            let len = self.aux_data.iter().filter(|&&val| val <= SR_INDEX).count();
            let sum: usize = self.aux_data.iter().filter(|&&val| val <= SR_INDEX).sum();

            let offset: isize = match self.last_mean {
                None if len>SR_MIN => {
                    self.last_mean = Some(sum / len);
                    0
                },
                Some(val) if len>SR_MIN => {
                    self.last_mean = Some( sum / len);
                    val as isize - (sum / len) as isize
                }
                _ => {
                    self.last_mean = None;
                    0
                },
            };

            for val in &self.aux_data {
                //append_to_array_roll(&mut self.data, *val, settings.bytedepth, offset/1);
            }
            
            /*
            if len > 0 {
                println!("{} and {} and {}", len, sum / len, offset/2);
            }
            else {
                println!("{}", len);
            }
            */
            
            self.last_time = new_time;
            self.aux_data = Vec::new();
        }
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, _settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = true;
    }
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        //append_to_array(&mut self.data, CAM_DESIGN.0-1, settings.bytedepth);
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings) {
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = 0);
            *self.data.iter_mut().last().expect("SpecKind: Last value is none.") = 10;
        }
    }
}

impl LiveTR1D {
    fn tr_check_if_in<T: TdcControl>(ele_time: usize, ref_tdc: &T, settings: &Settings) -> bool {
        let period = ref_tdc.period().expect("Period must exist in LiveTR1D.");
        let last_time = ref_tdc.time();

        let eff_tdc = if last_time > ele_time {
            let xper = (last_time - ele_time) / period + 1;
            last_time - xper * period
        } else {
            let xper = (ele_time - last_time) / period;
            last_time + xper * period
        };

        ele_time > eff_tdc + settings.time_delay && ele_time < eff_tdc + settings.time_delay + settings.time_width

    }
}


///Reads timepix3 socket and writes in the output socket a header and a full frame (binned or not). A periodic tdc is mandatory in order to define frame time.
pub fn build_spectrum<T, V, U, W>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut frame_tdc: PeriodicTdcRef, mut ref_tdc: T, mut meas_type: W) -> Result<(), Tp3ErrorKind> 
    where T: TdcControl,
          V: TimepixRead,
          U: Write,
          W: SpecKind
{
    
    let mut last_ci = 0usize;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    
    //let mut list = Live::new(&my_settings);
    let start = Instant::now();

    while let Ok(size) = pack_sock.read_timepix(&mut buffer_pack_data) {
        if build_data(&buffer_pack_data[0..size], &mut meas_type, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
            let msg = create_header(&my_settings, &frame_tdc);
            if ns_sock.write(&msg).is_err() {println!("Client disconnected on header."); break;}
            if ns_sock.write(meas_type.build_output()).is_err() {println!("Client disconnected on data."); break;}
            meas_type.reset_or_else(&frame_tdc, &my_settings);
            if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());};
        }
    }
    println!("Total elapsed time is: {:?}.", start.elapsed());
    Ok(())

}

fn build_data<T: TdcControl, W: SpecKind>(data: &[u8], final_data: &mut W, last_ci: &mut usize, settings: &Settings, frame_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) -> bool {

    data.chunks_exact(8).for_each( |x| {
        match *x {
            [84, 80, 88, 51, nci, _, _, _] => *last_ci = nci as usize,
            _ => {
                let packet = Pack { chip_index: *last_ci, data: x.try_into().unwrap()};
                
                match packet.id() {
                    11 => {
                        final_data.add_electron_hit(&packet, settings, frame_tdc, ref_tdc);
                    },
                    6 if packet.tdc_type() == frame_tdc.id() => {
                        final_data.upt_frame(&packet, frame_tdc, settings);
                    },
                    6 if packet.tdc_type() == ref_tdc.id() => {
                        final_data.add_tdc_hit(&packet, settings, ref_tdc);
                    },
                    _ => {},
                };
            },
        };
    });
    final_data.is_ready()
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
    
}

fn append_to_array_roll(data: &mut [u8], index:usize, bytedepth: usize, roll: isize) {
    let index = index as isize + roll;
    if index >= CAM_DESIGN.0 as isize - 1 || index < 0 {
        return
    }
    let index = index as usize;

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
}

fn create_header<T: TdcControl>(set: &Settings, tdc: &T) -> Vec<u8> {
    let mut msg: String = String::from("{\"timeAtFrame\":");
    msg.push_str(&(tdc.time().to_string()));
    msg.push_str(",\"frameNumber\":");
    msg.push_str(&((tdc.counter()/2).to_string()));
    msg.push_str(",\"measurementID:\"Null\",\"dataSize\":");
    if set.mode == 6 || set.mode == 7 { //ChronoMode
        msg.push_str(&((set.xspim_size*set.bytedepth*CAM_DESIGN.0).to_string()));
    } else {
        match set.bin {
            true => { msg.push_str(&((set.bytedepth*CAM_DESIGN.0).to_string()))},
            false => { msg.push_str(&((set.bytedepth*CAM_DESIGN.0*CAM_DESIGN.1).to_string()))},
        }
    }
    msg.push_str(",\"bitDepth\":");
    msg.push_str(&((set.bytedepth<<3).to_string()));
    msg.push_str(",\"width\":");
    msg.push_str(&(CAM_DESIGN.0.to_string()));
    msg.push_str(",\"height\":");
    if set.mode == 6 || set.mode == 7 { //ChronoMode
        msg.push_str(&(set.xspim_size.to_string()));
    } else {
        match set.bin {
            true=>{msg.push_str(&(1.to_string()))},
            false=>{msg.push_str(&(CAM_DESIGN.1.to_string()))},
        }
    }
    msg.push_str("}\n");

    let s: Vec<u8> = msg.into_bytes();
    s
}

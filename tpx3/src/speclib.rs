//!`speclib` is a collection of tools to set EELS/4D acquisition.

use crate::packetlib::{Packet, PacketEELS as Pack};
use crate::auxiliar::{Settings, misc::TimepixRead};
//use crate::tdclib::{TdcControl, PeriodicTdcRef};
use crate::tdclib::{TdcControl, PeriodicTdcRef, isi_box, isi_box::{IsiBoxTools, IsiBoxHand}};
use crate::isi_box_new;
use crate::errorlib::Tp3ErrorKind;
use std::time::Instant;
use std::io::Write;
use std::convert::TryInto;
//use rayon::prelude::*;
use core::ops::{Add, AddAssign};

const CAM_DESIGN: (usize, usize) = Pack::chip_array();
const BUFFER_SIZE: usize = 16384 * 2;
const SR_TIME: usize = 10_000; //Time window (10_000 -> 10 us);
const SR_INDEX: usize = 64; //Maximum x index value to account in the average calculation;
const SR_MIN: usize = 0; //Minimum array size to perform the average in super resolution;
const TILT_FRACTION: usize = 16; //Values with y = 256 will be tilted by 256 / 16;

fn as_bytes<T>(v: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u8,
            v.len() * std::mem::size_of::<T>())
    }
}

fn as_mut_bytes<T>(v: &[T]) -> &mut [u8] {
    unsafe {
        std::slice::from_raw_parts_mut(
            v.as_ptr() as *mut u8,
            v.len() * std::mem::size_of::<T>())
    }
}

macro_rules! genbitdepth {
    ($($x: ty),*) => {
        $(
        impl BitDepth for $x {
            fn zero() -> $x {
                0 as $x
            }
            fn one() -> $x {
                1 as $x
            }
            fn ten() -> $x {
                10 as $x
            }
        }
        )*
    }
}

macro_rules! genall {
    ($($x:ident),*) => {
        $(
            pub struct $x;
            impl GenerateDepth for $x{}
        )*
    }
}

macro_rules! gendepth{
    ($x: ident, $y: ty) => {
        fn $x(&self, set: &Settings) -> SpecMeasurement::<Self, $y> 
            where Self: Sized,
                  SpecMeasurement::<Self, $y>: SpecKind,
        {
            SpecMeasurement::<Self, $y>::new(set)
        }
    }
}

pub trait BitDepth: Clone + Add<Output = Self> + Copy + AddAssign {
    fn zero() -> Self;
    fn one() -> Self;
    fn ten() -> Self;
}
genbitdepth!(u8, u16, u32); //Implement BitDepth for u8, u16, u32;

pub trait GenerateDepth {
    gendepth!(gen32, u32);
    gendepth!(gen16, u16);
    gendepth!(gen8, u8);
}

genall!(Live2D, Live1D, LiveTR2D, LiveTR1D, LiveTilted2D, FastChrono, Chrono, SuperResolution); //create struct and implement GenerateDepth. GenDepth gets this struct and transforms into a SpecMeasurement struct, which is ready for acquisition;

pub struct SpecMeasurement<T, K: BitDepth> {
    data: Vec<K>,
    aux_data: Vec<usize>,
    is_ready: bool,
    global_stop: bool,
    last_time: usize,
    last_mean: Option<usize>,
    _kind: T,
}

pub trait SpecKind {
    fn is_ready(&self) -> bool;
    fn build_output(&self) -> &[u8];
    fn build_mut_output(&self) -> &mut [u8];
    fn new(settings: &Settings) -> Self;
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, frame_tdc: &PeriodicTdcRef, ref_tdc: &T);
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, ref_tdc: &mut T);
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, _settings: &Settings);
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings);
}

macro_rules! tp3_vec {
    ($x: expr) => {
        {
            let len = match $x {
                1 => CAM_DESIGN.0,
                2 => CAM_DESIGN.1*CAM_DESIGN.0,
                _ => {panic!("One or two dimensions only!")},
            };
            let mut temp_vec: Vec<L> = vec![L::zero(); len+1];
            temp_vec[len] = L::ten();
            temp_vec
        }
    }
}


impl<L: BitDepth> SpecKind for SpecMeasurement<Live2D, L> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn build_mut_output(&self) -> &mut [u8] {
        as_mut_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        SpecMeasurement{ data: tp3_vec!(2), aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: Live2D }
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, _settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &T) {
        let index = pack.x() + CAM_DESIGN.0 * pack.y();
        self.data[index] = self.data[index] + L::one();
    }
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        self.data[CAM_DESIGN.0-1] = self.data[CAM_DESIGN.0-1] + L::one();
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

impl<L: BitDepth> SpecKind for SpecMeasurement<Live1D, L> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn build_mut_output(&self) -> &mut [u8] {
        as_mut_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        SpecMeasurement{ data: tp3_vec!(1), aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: Live1D}
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, _settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &T) {
        let index = pack.x();
        self.data[index] = self.data[index] + L::one();
    }
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        self.data[CAM_DESIGN.0-1] = self.data[CAM_DESIGN.0-1] + L::one();
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

impl<L: BitDepth> SpecKind for SpecMeasurement<LiveTR2D, L> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn build_mut_output(&self) -> &mut [u8] {
        as_mut_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        SpecMeasurement{ data: tp3_vec!(2), aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: LiveTR2D}
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, _frame_tdc: &PeriodicTdcRef, ref_tdc: &T) {
        if LiveTR1D::tr_check_if_in(pack.electron_time(), ref_tdc, settings) {
            let index = pack.x() + CAM_DESIGN.0 * pack.y();
            self.data[index] = self.data[index] + L::one();
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
            self.data.iter_mut().for_each(|x| *x = L::zero());
            *self.data.iter_mut().last().expect("SpecKind: Last value is none.") = L::ten();
        }
    }
}

impl<L: BitDepth> SpecKind for SpecMeasurement<LiveTR1D, L> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn build_mut_output(&self) -> &mut [u8] {
        as_mut_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        SpecMeasurement{ data: tp3_vec!(1), aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: LiveTR1D}
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, _frame_tdc: &PeriodicTdcRef, ref_tdc: &T) {
        if LiveTR1D::tr_check_if_in(pack.electron_time(), ref_tdc, settings) {
            let index = pack.x();
            //append_to_array(&mut self.data, index, settings.bytedepth);
            self.data[index] = self.data[index] + L::one();
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
            self.data.iter_mut().for_each(|x| *x = L::zero());
            *self.data.iter_mut().last().expect("SpecKind: Last value is none.") = L::ten();
        }
    }
}

impl<L: BitDepth> SpecKind for SpecMeasurement<LiveTilted2D, L> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn build_mut_output(&self) -> &mut [u8] {
        as_mut_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        SpecMeasurement{ data: tp3_vec!(2), aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: LiveTilted2D }
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, _settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &T) {
        let x = pack.x();
        let y = pack.y();
        let index = x + CAM_DESIGN.0 * y;
        self.data[index] = self.data[index] + L::one();
    }
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        self.data[CAM_DESIGN.0-1] = self.data[CAM_DESIGN.0-1] + L::one();
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

impl<L: BitDepth> SpecKind for SpecMeasurement<FastChrono, L> {
    fn is_ready(&self) -> bool {
        self.is_ready && !self.global_stop
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn build_mut_output(&self) -> &mut [u8] {
        as_mut_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len: usize = settings.xspim_size*CAM_DESIGN.0;
        let mut temp_vec = vec![L::zero(); len + 1];
    //type MeasKind;
        temp_vec[len] = L::ten();
        SpecMeasurement{ data: temp_vec, aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: FastChrono}
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, frame_tdc: &PeriodicTdcRef, _ref_tdc: &T) {
        let line = frame_tdc.counter()/2;
        let index = pack.x() + line * CAM_DESIGN.0;
        if line < settings.xspim_size {
            self.data[index] = self.data[index] + L::one();
        }
    }
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        self.data[CAM_DESIGN.0-1] = self.data[CAM_DESIGN.0-1] + L::one();
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = (frame_tdc.counter()/2) > settings.xspim_size;
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, _settings: &Settings) {
        self.global_stop = true;
    }
}

impl<L: BitDepth> SpecKind for SpecMeasurement<Chrono, L> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn build_mut_output(&self) -> &mut [u8] {
        as_mut_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len: usize = settings.xspim_size*CAM_DESIGN.0;
        let mut temp_vec = vec![L::zero(); len + 1];
        temp_vec[len] = L::ten();
        SpecMeasurement{ data: temp_vec, aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: Chrono}
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, settings: &Settings, frame_tdc: &PeriodicTdcRef, _ref_tdc: &T) {
        let line = (frame_tdc.counter()/2) % settings.xspim_size;
        let index = pack.x() + line * CAM_DESIGN.0;
        self.data[index] = self.data[index] + L::one();
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        let line = frame_tdc.counter() / 2;
        self.is_ready = line % 20 == 0; //Every 20 lines send chrono;
        if line % settings.xspim_size == 0 {
            self.aux_data.push(0); //This indicates the frame must be refreshed;
        }
    }
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        self.data[CAM_DESIGN.0-1] = self.data[CAM_DESIGN.0-1] + L::one();
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, _settings: &Settings) {
        self.is_ready = false;
        if self.aux_data.len() > 0 { //Refresh frame if true;
            self.aux_data.pop(); //Remove for the next cycle;
            self.data.iter_mut().for_each(|x| *x = L::zero());
            *self.data.iter_mut().last().expect("SpecKind: Last value is none.") = L::zero();
        }
    }
}

impl<L: BitDepth> SpecKind for SpecMeasurement<SuperResolution, L> {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
       as_bytes(&self.data)
    }
    fn build_mut_output(&self) -> &mut [u8] {
        as_mut_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len: usize = settings.bytedepth*CAM_DESIGN.0;
        let mut temp_vec = vec![L::zero(); len + 1];
        temp_vec[len] = L::ten();
        SpecMeasurement{ data: temp_vec, aux_data: Vec::new(), is_ready: false, global_stop: false, last_time: 0, last_mean: None, _kind: SuperResolution}
    }
    #[inline]
    fn add_electron_hit<T: TdcControl>(&mut self, pack: &Pack, _settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &T) {
        let index = pack.x();
        self.aux_data.push(index);
        
        let new_time = pack.fast_electron_time();
        if new_time > self.last_time + SR_TIME {
            let len = self.aux_data.iter().filter(|&&val| val <= SR_INDEX).count();
            let sum: usize = self.aux_data.iter().filter(|&&val| val <= SR_INDEX).sum();

            let _offset: isize = match self.last_mean {
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

            for _val in &self.aux_data {
                //TODO: this must be rolled
                self.data[index] = self.data[index] + L::one();
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
    fn add_tdc_hit<T: TdcControl>(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut T) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        //append_to_array(&mut self.data, CAM_DESIGN.0-1, settings.bytedepth);
        self.data[CAM_DESIGN.0-1] = self.data[CAM_DESIGN.0-1] + L::one();
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings) {
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = L::zero());
            *self.data.iter_mut().last().expect("SpecKind: Last value is none.") = L::ten();
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
///
///# Examples
pub fn run_spectrum<T, V, U, Y>(pack: V, ns: U, my_settings: Settings, frame_tdc: PeriodicTdcRef, np_tdc: T, kind: Y) -> Result<u8, Tp3ErrorKind>
    where T: TdcControl,
          V: TimepixRead,
          U: Write,
          Y: GenerateDepth,
          SpecMeasurement<Y, u8>: SpecKind,
          SpecMeasurement<Y, u16>: SpecKind,
          SpecMeasurement<Y, u32>: SpecKind
{

    match my_settings.bytedepth {
        1 => {
            let measurement = kind.gen8(&my_settings);
            build_spectrum(pack, ns, my_settings, frame_tdc, np_tdc, measurement)?;
        },
        2 => {
            let measurement = kind.gen16(&my_settings);
            build_spectrum(pack, ns, my_settings, frame_tdc, np_tdc, measurement)?;
        },
        4 => {
            let measurement = kind.gen32(&my_settings);
            build_spectrum(pack, ns, my_settings, frame_tdc, np_tdc, measurement)?;
        },
        _ => {return Err(Tp3ErrorKind::SetByteDepth)},
    }
    Ok(my_settings.mode)
}
    
fn build_spectrum<T, V, U, W>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut frame_tdc: PeriodicTdcRef, mut ref_tdc: T, mut meas_type: W) -> Result<(), Tp3ErrorKind> 
    where T: TdcControl,
          V: TimepixRead,
          U: Write,
          W: SpecKind
{

    let mut last_ci = 0;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let start = Instant::now();

    while let Ok(size) = pack_sock.read_timepix(&mut buffer_pack_data) {
        if build_data(&buffer_pack_data[0..size], &mut meas_type, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
            let msg = create_header(&my_settings, &frame_tdc, 0);
            if ns_sock.write(&msg).is_err() {println!("Client disconnected on header."); break;}
            if ns_sock.write(meas_type.build_output()).is_err() {println!("Client disconnected on data."); break;}
            meas_type.reset_or_else(&frame_tdc, &my_settings);
            if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());};
        }
    }
    println!("Total elapsed time is: {:?}.", start.elapsed());
    Ok(())

}

pub fn build_spectrum_isi<T, V, U, W>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut frame_tdc: PeriodicTdcRef, mut ref_tdc: T, mut meas_type: W) -> Result<(), Tp3ErrorKind> 
    where T: TdcControl,
          V: TimepixRead,
          U: Write,
          W: SpecKind
{

    let mut handler = isi_box_new!(spec);
    handler.bind_and_connect();
    handler.configure_scan_parameters(32, 32, 8334);
    handler.configure_measurement_type();
    handler.start_threads();
    
    let mut last_ci = 0;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let start = Instant::now();

    while let Ok(size) = pack_sock.read_timepix(&mut buffer_pack_data) {
        if build_data(&buffer_pack_data[0..size], &mut meas_type, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
            let x = handler.get_data();
            let msg = create_header(&my_settings, &frame_tdc, 0);
            if ns_sock.write(&msg).is_err() {println!("Client disconnected on header."); break;}
            let result = meas_type.build_output();
            if ns_sock.write(result).is_err() {println!("Client disconnected on data."); break;}
            if ns_sock.write(as_bytes(&x)).is_err() {println!("Client disconnected on data."); break;}
            meas_type.reset_or_else(&frame_tdc, &my_settings);
            if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());};
        }
    }
    println!("Total elapsed time is: {:?}.", start.elapsed());
    Ok(())
}


fn build_data<T: TdcControl, W: SpecKind>(data: &[u8], final_data: &mut W, last_ci: &mut u8, settings: &Settings, frame_tdc: &mut PeriodicTdcRef, ref_tdc: &mut T) -> bool {

    data.chunks_exact(8).for_each( |x| {
        match *x {
            [84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
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

fn add_isibox_pixels(data: &mut [u8], isi_box_data: [u32; 17]) {
    data[CAM_DESIGN.0..].iter_mut().zip(as_bytes(&isi_box_data).iter()).for_each(|(a, b)| *a+=b);
}

fn create_header<T: TdcControl>(set: &Settings, tdc: &T, extra_pixels: usize) -> Vec<u8> {
    let mut msg: String = String::from("{\"timeAtFrame\":");
    msg.push_str(&(tdc.time().to_string()));
    msg.push_str(",\"frameNumber\":");
    msg.push_str(&((tdc.counter()/2).to_string()));
    msg.push_str(",\"measurementID:\"Null\",\"dataSize\":");
    if set.mode == 6 || set.mode == 7 { //ChronoMode
        msg.push_str(&((set.xspim_size*set.bytedepth*(CAM_DESIGN.0+extra_pixels)).to_string()));
    } else {
        match set.bin {
            true => { msg.push_str(&((set.bytedepth*(CAM_DESIGN.0+extra_pixels)).to_string()))},
            false => { msg.push_str(&((set.bytedepth*(CAM_DESIGN.0+extra_pixels)*CAM_DESIGN.1).to_string()))},
        }
    }
    msg.push_str(",\"bitDepth\":");
    msg.push_str(&((set.bytedepth<<3).to_string()));
    msg.push_str(",\"width\":");
    msg.push_str(&((CAM_DESIGN.0+extra_pixels).to_string()));
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


/*
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
*/

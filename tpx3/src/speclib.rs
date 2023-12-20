//!`speclib` is a collection of tools to set EELS/4D acquisition.

use crate::packetlib::{Packet, PacketEELS as Pack};
use crate::auxiliar::{Settings, misc::{TimepixRead, packet_change}};
use crate::tdclib::{TdcType, TdcControl, PeriodicTdcRef, NonPeriodicTdcRef, SingleTriggerPeriodicTdcRef, isi_box, isi_box::{IsiBoxTools, IsiBoxHand}};
use crate::isi_box_new;
use crate::errorlib::Tp3ErrorKind;
use std::time::Instant;
use std::io::Write;
use core::ops::{Add, AddAssign};
use crate::auxiliar::value_types::*;
use crate::constlib::*;

const CAM_DESIGN: (POSITION, POSITION) = Pack::chip_array();

fn as_bytes<T>(v: &[T]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u8,
            v.len() * std::mem::size_of::<T>())
    }
}

//Generating BitDepth for the standard types
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
            fn from_u16(value: u16) -> $x {
                value as $x
            }
        }
        )*
    }
}
genbitdepth!(u8, u16, u32); //Implement BitDepth for u8, u16, u32;

//Creates the unit-like acquisition modes structs and impl GenerateDepth
macro_rules! genall {
    ($($x:ident),*) => {
        $(
            pub struct $x;
            impl GenerateDepth for $x{}
        )*
    }
}

//functions inside GenerateDepth. From the acquisition modes to a SpecMeasurement
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
    fn from_u16(value: u16) -> Self;
}

pub trait GenerateDepth {
    gendepth!(gen32, u32);
    gendepth!(gen16, u16);
    gendepth!(gen8, u8);
}

genall!(Live2D, Live1D, LiveTR2D, LiveTR1D, LiveTilted2D, FastChrono, Chrono, SuperResolution, Live1DFrame, Live2DFrame, Live1DFrameHyperspec, Coincidence2D); //create struct and implement GenerateDepth. GenDepth gets this struct and transforms into a SpecMeasurement struct, which is ready for acquisition;

pub struct SpecMeasurement<T, K: BitDepth> {
    data: Vec<K>,
    aux_data: Vec<TIME>,
    is_ready: bool,
    global_stop: bool,
    timer: Instant,
    shutter: Option<ShutterControl>,
    _kind: T,
}

pub struct ShutterControl {
    time: [TIME; 4],
    counter: [COUNTER; 4],
    hyperspectral: bool,
    hyperspectral_complete: bool,
    hyperspec_pixels_to_send: (POSITION, POSITION), //Start and end pixel that will be sent
    shutter_closed_status: [bool; 4],
}

impl ShutterControl {
    fn try_set_time(&mut self, timestamp: TIME, ci: u8, shutter_closed: bool) -> bool {
        //When shutter is closed, data transfer initiates. Shutter_closed false means a new frame just
        //started, but we must wait <ACQUISITION_TIME> in order to close it and receive our data
        let ci = ci as usize;
        self.shutter_closed_status[ci] = shutter_closed;
        //if self.time[ci] == 0 {
        //    self.time[ci] = timestamp;
        //}
        println!("{:?} and {} and {} and {}", self.shutter_closed_status, timestamp, shutter_closed, ci);
        if shutter_closed && self.time[ci] != timestamp {
            println!("ready");
            self.time[ci] = timestamp;
            self.counter[ci] += 1;
            return self.time.iter().all(|val| *val == timestamp);
        }
        false
    }
    fn set_as_hyperspectral(&mut self) {
        self.hyperspectral = true;
    }
    fn set_hyperspectral_as_complete(&mut self) {
        self.hyperspectral_complete = true;
    }
    fn is_hyperspectral_complete(&self) -> bool {
        self.hyperspectral_complete
    }
    fn set_pixel_to_send(&mut self, start_pixel: POSITION, end_pixel: POSITION) {
        self.hyperspec_pixels_to_send = (start_pixel, end_pixel);
    }
    fn get_start_pixel(&self) -> POSITION {
        self.hyperspec_pixels_to_send.0
    }
    fn get_index_range_to_send(&self) -> std::ops::Range<usize> {
        let (start_pixel, end_pixel) = self.hyperspec_pixels_to_send;
        (start_pixel * CAM_DESIGN.0) as usize..(end_pixel * CAM_DESIGN.0) as usize
    }
    fn get_pixel_to_send_size(&self) -> POSITION {
        let (start_pixel, end_pixel) = self.hyperspec_pixels_to_send;
        end_pixel - start_pixel
    }
    fn get_counter(&self) -> [COUNTER; 4] {
        self.counter
    }
}

impl Default for ShutterControl {
    fn default() -> Self {
        Self {
            time: [0, 0, 0, 0],
            counter: [0, 0, 0, 0],
            hyperspectral: false,
            hyperspectral_complete: false,
            hyperspec_pixels_to_send: (0, 0),
            shutter_closed_status: [false; 4],
        }
    }
}

pub trait SpecKind {
    type SupplementaryTdc: TdcControl;
    fn is_ready(&self) -> bool;
    fn build_output(&self) -> &[u8];
    //fn build_mut_output(&self) -> &mut [u8];
    fn new(settings: &Settings) -> Self;
    fn build_main_tdc<V: TimepixRead>(&mut self, pack: &mut V) -> Result<PeriodicTdcRef, Tp3ErrorKind> {
        PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, pack, None)
    }
    fn build_aux_tdc<V: TimepixRead>(&self, pack: &mut V) -> Result<Self::SupplementaryTdc, Tp3ErrorKind> {
        Self::SupplementaryTdc::new(TdcType::TdcTwoRisingEdge, pack, None)
    }
    fn add_electron_hit(&mut self, pack: &Pack, settings: &Settings, frame_tdc: &PeriodicTdcRef, ref_tdc: &Self::SupplementaryTdc);
    fn add_tdc_hit(&mut self, pack: &Pack, settings: &Settings, ref_tdc: &mut Self::SupplementaryTdc);
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, _settings: &Settings);
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings);
    fn shutter_control(&self) -> Option<&ShutterControl> {None}
}

pub trait IsiBoxKind: SpecKind {
    fn isi_new(settings: &Settings) -> Self;
    fn append_from_isi(&mut self, ext_data: &[u32]);
}


macro_rules! tp3_vec {
    ($x: expr) => {
        {
            let len = match $x {
                1 => CAM_DESIGN.0,
                2 => CAM_DESIGN.1*CAM_DESIGN.0,
                _ => {panic!("One or two dimensions only!")},
            } as usize;
            let temp_vec: Vec<L> = vec![L::zero(); len];
            temp_vec
        }
    }
}

macro_rules! add_index {
    ($x: ident, $y: expr) => {
        {
            $x.data[$y as usize] += L::one();
            //*$x.data.iter_mut().nth($y as usize).unwrap() += L::one();
        }
    }
}


impl<L: BitDepth> SpecKind for SpecMeasurement<Live2D, L> {
    type SupplementaryTdc = NonPeriodicTdcRef;
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        SpecMeasurement{ data: tp3_vec!(2), aux_data: Vec::new(), is_ready: false, global_stop: false, timer: Instant::now(), shutter: None, _kind: Live2D }
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Pack, _settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &Self::SupplementaryTdc) {
        let index = pack.x() + CAM_DESIGN.0 * pack.y();
        add_index!(self, index);
    }
    fn add_tdc_hit(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut Self::SupplementaryTdc) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, settings: &Settings) {
        if pack.id() != 6 {println!("{}", pack.id())};
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        if self.timer.elapsed().as_millis() < TIME_INTERVAL_FRAMES {
            self.reset_or_else(frame_tdc, settings);
        } else {
            self.is_ready = true;
            self.timer = Instant::now();
        }
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings) {
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = L::zero());
        }
    }
}

impl<L: BitDepth> SpecKind for SpecMeasurement<Live1D, L> {
    type SupplementaryTdc = NonPeriodicTdcRef;
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        SpecMeasurement{ data: tp3_vec!(1), aux_data: Vec::new(), is_ready: false, global_stop: false, timer: Instant::now(), shutter: None, _kind: Live1D}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Pack, _settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &Self::SupplementaryTdc) {
        let index = pack.x();
        add_index!(self, index);
    }
    fn add_tdc_hit(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut Self::SupplementaryTdc) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, _settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = true;
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings) {
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = L::zero());
        }
    }
}

impl<L: BitDepth> SpecKind for SpecMeasurement<LiveTR2D, L> {
    type SupplementaryTdc = SingleTriggerPeriodicTdcRef;
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        SpecMeasurement{ data: tp3_vec!(2), aux_data: Vec::new(), is_ready: false, global_stop: false, timer: Instant::now(), shutter: None, _kind: LiveTR2D}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Pack, settings: &Settings, _frame_tdc: &PeriodicTdcRef, ref_tdc: &Self::SupplementaryTdc) {
        if LiveTR1D::tr_check_if_in(pack.electron_time(), ref_tdc, settings) {
            let index = pack.x() + CAM_DESIGN.0 * pack.y();
            add_index!(self, index);
        }
    }
    fn add_tdc_hit(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut Self::SupplementaryTdc) {
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
        }
    }
}

impl<L: BitDepth> SpecKind for SpecMeasurement<LiveTR1D, L> {
    type SupplementaryTdc = SingleTriggerPeriodicTdcRef;
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        SpecMeasurement{ data: tp3_vec!(1), aux_data: Vec::new(), is_ready: false, global_stop: false, timer: Instant::now(), shutter: None, _kind: LiveTR1D}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Pack, settings: &Settings, _frame_tdc: &PeriodicTdcRef, ref_tdc: &Self::SupplementaryTdc) {
        if LiveTR1D::tr_check_if_in(pack.electron_time(), ref_tdc, settings) {
            let index = pack.x();
            add_index!(self, index);
        }
    }
    fn add_tdc_hit(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut Self::SupplementaryTdc) {
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
        }
    }
}


impl<L: BitDepth> SpecKind for SpecMeasurement<LiveTilted2D, L> {
    type SupplementaryTdc = NonPeriodicTdcRef;
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        SpecMeasurement{ data: tp3_vec!(2), aux_data: Vec::new(), is_ready: false, global_stop: false, timer: Instant::now(), shutter: None, _kind: LiveTilted2D }
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Pack, _settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &Self::SupplementaryTdc) {
        let x = pack.x();
        let y = pack.y();
        let index = x + CAM_DESIGN.0 * y;
        add_index!(self, index);
    }
    fn add_tdc_hit(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut Self::SupplementaryTdc) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, _settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = true;
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings) {
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = L::zero());
        }
    }
}

impl<L: BitDepth> SpecKind for SpecMeasurement<Coincidence2D, L> {
    type SupplementaryTdc = NonPeriodicTdcRef;
    fn is_ready(&self) -> bool {
        self.is_ready && !self.global_stop
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len = 2*settings.time_width as usize * CAM_DESIGN.0 as usize;
        let temp_vec = vec![L::zero(); len];
        SpecMeasurement{ data: temp_vec, aux_data: vec![0; LIST_SIZE_AUX_EVENTS], is_ready: false, global_stop: false, timer: Instant::now(), shutter: None, _kind: Coincidence2D}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Pack, settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &Self::SupplementaryTdc) {
        let etime = pack.electron_time();
        for phtime in self.aux_data.iter() {
            if (*phtime < etime + settings.time_delay + settings.time_width) && (etime + settings.time_delay < *phtime + settings.time_width) {
                let delay = (phtime - settings.time_delay + settings.time_width - etime) as POSITION;
                let index = pack.x() + delay * CAM_DESIGN.0;
                add_index!(self, index);
            }
        }
    }
    fn add_tdc_hit(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut Self::SupplementaryTdc) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        for index in 0..LIST_SIZE_AUX_EVENTS-1 {
            self.aux_data[index+1] = self.aux_data[index];
        }
        self.aux_data[0] = pack.tdc_time_norm();
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        if self.timer.elapsed().as_millis() < TIME_INTERVAL_COINCIDENCE_HISTOGRAM {
            self.reset_or_else(frame_tdc, settings);
        } else {
            self.is_ready = true;
            self.timer = Instant::now();
        }
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings) {
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = L::zero());
        }
    }
}



impl<L: BitDepth> SpecKind for SpecMeasurement<FastChrono, L> {
    type SupplementaryTdc = NonPeriodicTdcRef;
    fn is_ready(&self) -> bool {
        self.is_ready && !self.global_stop
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len = (settings.xspim_size*CAM_DESIGN.0) as usize;
        let mut temp_vec = vec![L::zero(); len];
        temp_vec[len] = L::ten();
        SpecMeasurement{ data: temp_vec, aux_data: Vec::new(), is_ready: false, global_stop: false, timer: Instant::now(), shutter: None, _kind: FastChrono}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Pack, settings: &Settings, frame_tdc: &PeriodicTdcRef, _ref_tdc: &Self::SupplementaryTdc) {
        let line = (frame_tdc.counter()/2) as POSITION;
        let index = pack.x() + line * CAM_DESIGN.0;
        if line < settings.xspim_size {
            add_index!(self, index);
        }
    }
    fn add_tdc_hit(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut Self::SupplementaryTdc) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = (frame_tdc.counter()/2) as POSITION > settings.xspim_size;
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, _settings: &Settings) {
        self.global_stop = true;
    }
}

impl<L: BitDepth> SpecKind for SpecMeasurement<Chrono, L> {
    type SupplementaryTdc = NonPeriodicTdcRef;
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len = (settings.xspim_size*CAM_DESIGN.0) as usize;
        let mut temp_vec = vec![L::zero(); len];
        temp_vec[len] = L::ten();
        SpecMeasurement{ data: temp_vec, aux_data: Vec::new(), is_ready: false, global_stop: false, timer: Instant::now(), shutter: None, _kind: Chrono}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Pack, settings: &Settings, frame_tdc: &PeriodicTdcRef, _ref_tdc: &Self::SupplementaryTdc) {
        let line = (frame_tdc.counter()/2) as POSITION % settings.xspim_size;
        let index = pack.x() + line * CAM_DESIGN.0;
        add_index!(self, index);
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        let line = (frame_tdc.counter() / 2) as POSITION;
        self.is_ready = line % 20 == 0; //Every 20 lines send chrono;
        if line % settings.xspim_size == 0 {
            self.aux_data.push(0); //This indicates the frame must be refreshed;
        }
    }
    fn add_tdc_hit(&mut self, pack: &Pack, _settings: &Settings, ref_tdc: &mut Self::SupplementaryTdc) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, _settings: &Settings) {
        self.is_ready = false;
        if self.aux_data.len() > 0 { //Refresh frame if true;
            self.aux_data.pop(); //Remove for the next cycle;
            self.data.iter_mut().for_each(|x| *x = L::zero());
        }
    }
}


/*
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
*/

impl<L: BitDepth> SpecKind for SpecMeasurement<Live2DFrame, L> {
    type SupplementaryTdc = NonPeriodicTdcRef;
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        SpecMeasurement{ data: tp3_vec!(2), aux_data: Vec::new(), is_ready: false, global_stop: false, timer: Instant::now(), shutter: Some(ShutterControl::default()), _kind: Live2DFrame }
    }

    #[inline]
    fn add_electron_hit(&mut self, pack: &Pack, settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &Self::SupplementaryTdc) {
        //You only add electrons to the pair frame number. You are going to send this one
        //when the next one is over, so you are sure the pair one is complete.
        
        let index = pack.x() + CAM_DESIGN.0 * pack.y();
        if settings.cumul {
            self.data[index as usize] += L::from_u16(pack.tot());
        }
    }
    fn add_tdc_hit(&mut self, _pack: &Pack, _settings: &Settings, _ref_tdc: &mut Self::SupplementaryTdc) {}
    fn build_main_tdc<V: TimepixRead>(&mut self, _pack: &mut V) -> Result<PeriodicTdcRef, Tp3ErrorKind> {
        PeriodicTdcRef::new_no_read(TdcType::TdcOneRisingEdge, None)
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, settings: &Settings) {
        if pack.id() == 5 {
            let temp_ready = self.shutter.as_mut().unwrap().try_set_time(pack.frame_time(), pack.ci(), pack.tdc_type() == 10);
            if !self.is_ready {
                self.is_ready = temp_ready;
                if self.is_ready {
                    if self.timer.elapsed().as_millis() < TIME_INTERVAL_FRAMES {
                        self.is_ready = false;
                        self.reset_or_else(frame_tdc, settings);
                    } else {
                        self.is_ready = true;
                        self.timer = Instant::now();
                    }
                }
            }
            //if self.is_ready {
            //    println!(" OK {} and {}", self.data.iter().map(|val| *val as u32).sum::<u32>(), self.timer.elapsed().as_millis());
            //}
        }
        else if pack.id() == 6 {
            frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        }
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings) {
        self.is_ready = false;
        if !settings.cumul { //No cumulation
            self.data.iter_mut().for_each(|x| *x = L::zero());
        }
    }
    fn shutter_control(&self) -> Option<&ShutterControl> {
        self.shutter.as_ref()
    }
}

impl<L: BitDepth> SpecKind for SpecMeasurement<Live1DFrame, L> {
    type SupplementaryTdc = NonPeriodicTdcRef;
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        SpecMeasurement{ data: tp3_vec!(1), aux_data: Vec::new(), is_ready: false, global_stop: false, timer: Instant::now(), shutter: Some(ShutterControl::default()), _kind: Live1DFrame }
    }

    #[inline]
    fn add_electron_hit(&mut self, pack: &Pack, settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &Self::SupplementaryTdc) {
        let index = pack.x();
        if settings.cumul {
            self.data[index as usize] += L::from_u16(pack.tot());
        }
    }
    fn add_tdc_hit(&mut self, _pack: &Pack, _settings: &Settings, _ref_tdc: &mut Self::SupplementaryTdc) {}
    fn build_main_tdc<V: TimepixRead>(&mut self, _pack: &mut V) -> Result<PeriodicTdcRef, Tp3ErrorKind> {
        PeriodicTdcRef::new_no_read(TdcType::TdcOneRisingEdge, None)
    }
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, _settings: &Settings) {
        if pack.id() == 5 {
            let temp_ready = self.shutter.as_mut().unwrap().try_set_time(pack.frame_time(), pack.ci(), pack.tdc_type() == 10);
            if !self.is_ready {
                self.is_ready = temp_ready;
            }
        }
        else if pack.id() == 6 {
            frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        }
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, settings: &Settings) {
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = L::zero());
        }
    }
    fn shutter_control(&self) -> Option<&ShutterControl> {
        self.shutter.as_ref()
    }
}

impl<L: BitDepth> SpecKind for SpecMeasurement<Live1DFrameHyperspec, L> {
    type SupplementaryTdc = NonPeriodicTdcRef;
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&self) -> &[u8] {
        //The number of pixels sent is updated on the reset or else function
        let range = self.shutter.as_ref().expect("This mode must have the Shutter Control").get_index_range_to_send();
        as_bytes(&self.data[range])
    }
    fn new(settings: &Settings) -> Self {
        let len = (CAM_DESIGN.0 * settings.xscan_size * settings.yscan_size) as usize;
        let mut shutter = ShutterControl::default();
        shutter.set_as_hyperspectral();
        SpecMeasurement{ data: vec![L::zero(); len], aux_data: Vec::new(), is_ready: false, global_stop: false, timer: Instant::now(), shutter: Some(shutter), _kind: Live1DFrameHyperspec }
    }

    #[inline]
    fn add_electron_hit(&mut self, pack: &Pack, _settings: &Settings, _frame_tdc: &PeriodicTdcRef, _ref_tdc: &Self::SupplementaryTdc) {
        let shut = self.shutter.as_ref().unwrap();
        if shut.is_hyperspectral_complete() { return }
        let frame_number = shut.get_counter()[pack.ci() as usize] as POSITION;
        //We cannot depass frame_number otherwise the indexation will be bad
        let index = frame_number * CAM_DESIGN.0 + pack.x();
        self.data[index as usize] += L::from_u16(pack.tot());
    }
    fn build_main_tdc<V: TimepixRead>(&mut self, _pack: &mut V) -> Result<PeriodicTdcRef, Tp3ErrorKind> {
        PeriodicTdcRef::new_no_read(TdcType::TdcOneRisingEdge, None)
    }
    fn add_tdc_hit(&mut self, _pack: &Pack, _settings: &Settings, _ref_tdc: &mut Self::SupplementaryTdc) {}
    fn upt_frame(&mut self, pack: &Pack, frame_tdc: &mut PeriodicTdcRef, settings: &Settings) {
        if pack.id() == 5 {
            let _temp_ready = self.shutter.as_mut().unwrap().try_set_time(pack.frame_time(), pack.ci(), pack.tdc_type() == 10);
            let shutter_counter = self.shutter.as_ref().unwrap().get_counter()[pack.ci() as usize] as POSITION;
            if shutter_counter >= settings.xscan_size * settings.yscan_size {
                self.shutter.as_mut().unwrap().set_hyperspectral_as_complete();
            }
            let pixels_sent = self.shutter.as_ref().expect("Shutter must be present in Frame-based mode.").hyperspec_pixels_to_send.1;
            if shutter_counter > pixels_sent + HYPERSPECTRAL_PIXEL_CHUNK {
                self.is_ready = true;
                let begin_pixel = pixels_sent;
                let end_pixel = std::cmp::min(pixels_sent + HYPERSPECTRAL_PIXEL_CHUNK, settings.xscan_size * settings.yscan_size);
                self.shutter.as_mut().unwrap().set_pixel_to_send(begin_pixel, end_pixel);
                println!("***FB Hyperspec***: Sending_data with counter {}. Begin and end pixels are {} and {}", shutter_counter, begin_pixel, end_pixel);

            }
            /*
            if !self.is_ready {
                if self.timer.elapsed().as_millis() < TIME_INTERVAL_HYPERSPECTRAL_FRAME {
                    self.is_ready = false;
                } else {
                    println!("sending_data with counter {}", shutter_counter);
                    self.is_ready = true;
                }
            }
            */
        }
        else if pack.id() == 6 {
            frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        }
    }
    fn reset_or_else(&mut self, _frame_tdc: &PeriodicTdcRef, _settings: &Settings) {
        self.is_ready = false;
        self.timer = Instant::now();
    }
    fn shutter_control(&self) -> Option<&ShutterControl> {
        self.shutter.as_ref()
    }
}

impl LiveTR1D {
    fn tr_check_if_in<T: TdcControl>(ele_time: TIME, ref_tdc: &T, settings: &Settings) -> bool {
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


impl IsiBoxKind for SpecMeasurement<Live1D, u32> {
    fn isi_new(_settings: &Settings) -> Self {
        let len = (CAM_DESIGN.0 + CHANNELS as POSITION) as usize;
        SpecMeasurement{ data: vec![0; len], aux_data: Vec::new(), is_ready: false, global_stop: false, timer: Instant::now(), shutter: None, _kind: Live1D }
    }
    fn append_from_isi(&mut self, ext_data: &[u32]) {
        self.data[CAM_DESIGN.0 as usize..].iter_mut().zip(ext_data.iter()).for_each(|(a, b)| *a+=b);
    }
}

///Reads timepix3 socket and writes in the output socket a header and a full frame (binned or not). A periodic tdc is mandatory in order to define frame time.
///
///# Examples
pub fn run_spectrum<V, U, Y>(mut pack: V, ns: U, my_settings: Settings, kind: Y) -> Result<u8, Tp3ErrorKind>
    where V: TimepixRead,
          U: Write,
          Y: GenerateDepth,
          SpecMeasurement<Y, u8>: SpecKind,
          SpecMeasurement<Y, u16>: SpecKind,
          SpecMeasurement<Y, u32>: SpecKind
{

    
    match my_settings.bytedepth {
        1 => {
            let mut measurement = kind.gen8(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack)?;
            let aux_tdc = measurement.build_aux_tdc(&mut pack)?;
            build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement)?;
        },
        2 => {
            let mut measurement = kind.gen16(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack)?;
            let aux_tdc = measurement.build_aux_tdc(&mut pack)?;
            build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement)?;
        },
        4 => {
            let mut measurement = kind.gen32(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack)?;
            let aux_tdc = measurement.build_aux_tdc(&mut pack)?;
            build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement)?;
        },
        _ => {return Err(Tp3ErrorKind::SetByteDepth)},
    }
    
    Ok(my_settings.mode)
}

fn build_spectrum<V, U, W>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut frame_tdc: PeriodicTdcRef, mut ref_tdc: W::SupplementaryTdc, mut meas_type: W) -> Result<(), Tp3ErrorKind> 
    where V: TimepixRead,
          U: Write,
          W: SpecKind
{

    let mut last_ci = 0;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let start = Instant::now();
    
    let mut file_to_write = my_settings.create_file();
    while let Ok(size) = pack_sock.read_timepix(&mut buffer_pack_data) {
        if let Some(file) = &mut file_to_write {
            file.write(&buffer_pack_data[0..size]).unwrap();
        }
        if build_data(&buffer_pack_data[0..size], &mut meas_type, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
            let msg = create_header(&my_settings, &frame_tdc, 0, meas_type.shutter_control());
            if ns_sock.write(&msg).is_err() {println!("Client disconnected on header."); break;}
            if ns_sock.write(meas_type.build_output()).is_err() {println!("Client disconnected on data."); break;}
            meas_type.reset_or_else(&frame_tdc, &my_settings);
            if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());};
        }
    }
    println!("Total elapsed time is: {:?}.", start.elapsed());
    Ok(())

}

pub fn build_spectrum_isi<V, U, W>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut frame_tdc: PeriodicTdcRef, mut ref_tdc: W::SupplementaryTdc, mut meas_type: W) -> Result<(), Tp3ErrorKind> 
    where V: TimepixRead,
          U: Write,
          W: IsiBoxKind
{

    let mut handler = isi_box_new!(spec);
    handler.bind_and_connect()?;
    handler.configure_scan_parameters(32, 32, 8334)?;
    handler.configure_measurement_type(false)?;
    handler.start_threads();
    
    let mut last_ci = 0;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let start = Instant::now();

    while let Ok(size) = pack_sock.read_timepix(&mut buffer_pack_data) {
        if build_data(&buffer_pack_data[0..size], &mut meas_type, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
            let x = handler.get_data();
            let msg = create_header(&my_settings, &frame_tdc, CHANNELS as POSITION, meas_type.shutter_control());
            if ns_sock.write(&msg).is_err() {println!("Client disconnected on header."); break;}
            meas_type.append_from_isi(&x);
            let result = meas_type.build_output();
            if ns_sock.write(result).is_err() {println!("Client disconnected on data."); break;}
            meas_type.reset_or_else(&frame_tdc, &my_settings);
            if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());};
        }
    }
    handler.stop_threads();
    println!("Total elapsed time is: {:?}.", start.elapsed());
    Ok(())
}


fn build_data<W: SpecKind>(data: &[u8], final_data: &mut W, last_ci: &mut u8, settings: &Settings, frame_tdc: &mut PeriodicTdcRef, ref_tdc: &mut W::SupplementaryTdc) -> bool {

    data.chunks_exact(8).for_each( |x| {
        match *x {
            [84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
            _ => {
                let packet = Pack { chip_index: *last_ci, data: packet_change(x)[0]};
                
                match packet.id() {
                    11 | 10 => { //Event or frame based
                        final_data.add_electron_hit(&packet, settings, frame_tdc, ref_tdc);
                    },
                    6 if packet.tdc_type() == frame_tdc.id() => { //Tdc value 1
                        final_data.upt_frame(&packet, frame_tdc, settings);
                    },
                    6 if packet.tdc_type() == ref_tdc.id() => { //Tdc value 2
                        final_data.add_tdc_hit(&packet, settings, ref_tdc);
                    },
                    5 if packet.tdc_type() == 10 || packet.tdc_type() == 15  => { //Shutter value.
                        final_data.upt_frame(&packet, frame_tdc, settings);
                    },
                    _ => {},
                };
            },
        };
    });
    final_data.is_ready()
}

//fn add_isibox_pixels(data: &mut [u8], isi_box_data: [u32; 17]) {
//    data[CAM_DESIGN.0..].iter_mut().zip(as_bytes(&isi_box_data).iter()).for_each(|(a, b)| *a+=b);
//}

fn create_header<T: TdcControl>(set: &Settings, tdc: &T, extra_pixels: POSITION, shutter_control: Option<&ShutterControl>) -> Vec<u8> {
    let mut msg: String = String::from("{\"timeAtFrame\":");
    msg.push_str(&(tdc.time().to_string()));
    msg.push_str(",\"frameNumber\":");
    if let Some(shutter) = shutter_control {
        msg.push_str(&((shutter.get_start_pixel()).to_string()));
    } else {
        msg.push_str(&((tdc.counter()/2).to_string()));
    }
    msg.push_str(",\"measurementID:\"Null\",\"dataSize\":");
    if set.mode == 6 { //ChronoMode
        msg.push_str(&((set.xspim_size*set.bytedepth*(CAM_DESIGN.0+extra_pixels)).to_string()));
    } else if set.mode == 7 { //Coincidence2D
        msg.push_str(&((set.time_width as POSITION*2*set.bytedepth*(CAM_DESIGN.0+extra_pixels)).to_string()));
    } else if set.mode == 11 { //Frame-based hyperspectral image
        let data_size = shutter_control.unwrap().get_pixel_to_send_size();
        msg.push_str(&((data_size*set.bytedepth*(CAM_DESIGN.0+extra_pixels)).to_string()));
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
    if set.mode == 6 { //ChronoMode
        msg.push_str(&(set.xspim_size.to_string()));
    } else if set.mode == 7 { //Coincidence2D Mode
        msg.push_str(&((set.time_width*2).to_string()));
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

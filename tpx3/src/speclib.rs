//!`speclib` is a collection of tools to set EELS/4D acquisition.

use crate::packetlib::Packet;
use crate::auxiliar::{aux_func, Settings, misc::{TimepixRead, packet_change}};
use crate::tdclib::{TdcType, TdcRef, isi_box, isi_box::{IsiBoxTools, IsiBoxHand}};
use crate::isi_box_new;
//use crate::clusterlib::cluster::{SingleElectron, CollectionElectron, SinglePhoton, CollectionPhoton};
use crate::errorlib::Tp3ErrorKind;
use std::time::Instant;
use std::io::Write;
use crate::auxiliar::{value_types::*, FileManager};
use crate::constlib::*;

const CAM_DESIGN: (POSITION, POSITION) = Packet::chip_array();


fn tr_check_if_in(ele_time: TIME, ref_tdc: &TdcRef, settings: &Settings) -> bool {
    let period = ref_tdc.period().expect("Period must exist in LiveTR1D.");
    let last_tdc_time = ref_tdc.time();
 
    //This case photon time is always greater than electron time
    let eff_tdc = if last_tdc_time > ele_time {
        let xper = (last_tdc_time - ele_time) / period;
        last_tdc_time - xper * period
    } else {
        let xper = (ele_time - last_tdc_time) / period + 1;
        last_tdc_time + xper * period
    };
    ele_time + settings.time_delay + settings.time_width > eff_tdc && ele_time + settings.time_delay < eff_tdc + settings.time_width
}


#[derive(Default)]
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
        //When shutter_closed is true, we receive electrons as packets. Shutter_closed false means a new frame just
        //started, but we must wait <ACQUISITION_TIME> in order to close it and receive our data
        let ci = ci as usize;
        self.shutter_closed_status[ci] = shutter_closed;
        if !shutter_closed && self.time[ci] != timestamp {
            //first false in which all timestemps differ is the frame_condition. The shutter just
            //opened in one chip so electrons are not arriving as packets anymore.
            let temp2 = if HIGH_DYNAMIC_FRAME_BASED {
                self.time.iter().all(|val| *val != timestamp) && self.counter[ci] % HIGH_DYNAMIC_FRAME_BASED_VALUE  == 0
            } else {
                self.time.iter().all(|val| *val != timestamp)
            };
            self.time[ci] = timestamp;
            self.counter[ci] += 1;
            return temp2;
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

pub trait SpecKind {
    fn is_ready(&self) -> bool;
    fn build_output(&mut self, settings: &Settings) -> &[u8];
    fn new(settings: &Settings) -> Self;
    fn build_main_tdc<V: TimepixRead>(&mut self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_periodic(TdcType::TdcOneRisingEdge, pack, &my_settings, file_to_write)
    }
    fn build_aux_tdc<V: TimepixRead>(&self, _pack: &mut V, _my_settings: &Settings, _file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_no_read(TdcType::TdcTwoRisingEdge)
    }
    fn add_electron_hit(&mut self, pack: &Packet, settings: &Settings, frame_tdc: &TdcRef, ref_tdc: &TdcRef);
    fn add_tdc_hit1(&mut self, pack: &Packet, frame_tdc: &mut TdcRef, _settings: &Settings);
    fn add_tdc_hit2(&mut self, pack: &Packet, settings: &Settings, ref_tdc: &mut TdcRef);
    fn add_shutter_hit(&mut self, _pack: &Packet, _frame_tdc: &mut TdcRef, _settings: &Settings) {}
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, settings: &Settings);
    fn shutter_control(&self) -> Option<&ShutterControl> {None}
}

pub trait IsiBoxKind: SpecKind {
    fn isi_new(settings: &Settings) -> Self;
    fn append_from_isi(&mut self, ext_data: &[u32]);
}

macro_rules! add_index {
    ($x: ident, $y: expr) => {
        {
            $x.data[$y as usize] += 1;
        }
    }
}

macro_rules! tp3_vec {
    ($x: expr) => {
        {
            let len = match $x {
                1 => CAM_DESIGN.0,
                2 => CAM_DESIGN.0 * CAM_DESIGN.1,
                _ => {panic!("One or two dimensions only!")},
            } as usize;
            let temp_vec: Vec<u32> = vec![0; len];
            temp_vec
        }
    }
}

pub struct Live2D {
    data: Vec<u32>,
    is_ready: bool,
    timer: Instant,
}


impl SpecKind for Live2D {
    //If INTERNAL_TIMER_FRAME, the ready is given by the internal elapsed time.
    fn is_ready(&self) -> bool {
        if !INTERNAL_TIMER_FRAME {
            self.is_ready
        } else {
            self.timer.elapsed().as_millis() > TIME_INTERVAL_FRAMES
        }
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        aux_func::as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        Self{ data: tp3_vec!(2), is_ready: false, timer: Instant::now() }
    }
    fn build_main_tdc<V: TimepixRead>(&mut self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        if INTERNAL_TIMER_FRAME {
            TdcRef::new_no_read(TdcType::TdcOneRisingEdge)
        } else {
            TdcRef::new_periodic(TdcType::TdcOneRisingEdge, pack, &my_settings, file_to_write)
        }
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Packet, _settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        let index = pack.x() + CAM_DESIGN.0 * pack.y();
        add_index!(self, index);
    }
    fn add_tdc_hit2(&mut self, pack: &Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        add_index!(self, CAM_DESIGN.0-1);
    }
    //If INTERNAL_TIMER_FRAME, update frame actually adds one to the before last pixel of the
    //system
    fn add_tdc_hit1(&mut self, pack: &Packet, frame_tdc: &mut TdcRef, settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        if !INTERNAL_TIMER_FRAME {
            if self.timer.elapsed().as_millis() < TIME_INTERVAL_FRAMES {
                self.is_ready = false;
                if !settings.cumul {
                    self.data.iter_mut().for_each(|x| *x = 0);
                }
            } else {
                self.is_ready = true;
            }
        } else {
            add_index!(self, CAM_DESIGN.0-2);
        }
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, settings: &Settings) {
        self.timer = Instant::now();
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = 0);
        }
    }
}

pub struct Live1D {
    data: Vec<u32>,
    is_ready: bool,
    timer: Instant,
}


impl SpecKind for Live1D {
    fn is_ready(&self) -> bool {
        if !INTERNAL_TIMER_FRAME {
            self.is_ready
        } else {
            self.timer.elapsed().as_millis() > TIME_INTERVAL_FRAMES
        }
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        aux_func::as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        Self{ data: tp3_vec!(1), is_ready: false, timer: Instant::now()}
    }
    fn build_main_tdc<V: TimepixRead>(&mut self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        if INTERNAL_TIMER_FRAME {
            TdcRef::new_no_read(TdcType::TdcOneRisingEdge)
        } else {
            TdcRef::new_periodic(TdcType::TdcOneRisingEdge, pack, &my_settings, file_to_write)
        }
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Packet, _settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        let index = pack.x();
        add_index!(self, index);
    }
    fn add_tdc_hit2(&mut self, pack: &Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn add_tdc_hit1(&mut self, pack: &Packet, frame_tdc: &mut TdcRef, settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        if !INTERNAL_TIMER_FRAME {
            if self.timer.elapsed().as_millis() < TIME_INTERVAL_FRAMES {
                self.is_ready = false;
                if !settings.cumul {
                    self.data.iter_mut().for_each(|x| *x = 0);
                }
            } else {
                self.is_ready = true;
            }
        } else {
            add_index!(self, CAM_DESIGN.0-2);
        }
    }
        fn reset_or_else(&mut self, _frame_tdc: &TdcRef, settings: &Settings) {
        self.timer = Instant::now();
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = 0);
        }
    }
}

pub struct LiveTR2D {
    data: Vec<u32>,
    is_ready: bool,
    timer: Instant,
}


impl  SpecKind for LiveTR2D {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        aux_func::as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        Self{ data: tp3_vec!(2), is_ready: false, timer: Instant::now()}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Packet, settings: &Settings, _frame_tdc: &TdcRef, ref_tdc: &TdcRef) {
        if tr_check_if_in(pack.electron_time(), ref_tdc, settings) {
            let index = pack.x() + CAM_DESIGN.0 * pack.y();
            add_index!(self, index);
        }
    }
    fn build_aux_tdc<V: TimepixRead>(&self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_periodic(TdcType::TdcTwoRisingEdge, pack, &my_settings, file_to_write)
    }
    fn add_tdc_hit2(&mut self, pack: &Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
    }
    fn add_tdc_hit1(&mut self, pack: &Packet, frame_tdc: &mut TdcRef, settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        if self.timer.elapsed().as_millis() < TIME_INTERVAL_FRAMES {
            self.is_ready = false;
            if !settings.cumul {
                self.data.iter_mut().for_each(|x| *x = 0);
            }
        } else {
            self.is_ready = true;
        }
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, settings: &Settings) {
        self.is_ready = false;
        self.timer = Instant::now();
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = 0);
        }
    }
}

pub struct LiveTR1D {
    data: Vec<u32>,
    is_ready: bool,
    timer: Instant,
}

impl SpecKind for LiveTR1D {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        aux_func::as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        Self{ data: tp3_vec!(1), is_ready: false, timer: Instant::now()}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Packet, settings: &Settings, _frame_tdc: &TdcRef, ref_tdc: &TdcRef) {
        if tr_check_if_in(pack.electron_time(), ref_tdc, settings) {
            let index = pack.x();
            add_index!(self, index);
        }
    }
    fn build_aux_tdc<V: TimepixRead>(&self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_periodic(TdcType::TdcTwoRisingEdge, pack, &my_settings, file_to_write)
    }
    fn add_tdc_hit2(&mut self, pack: &Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
    }
    fn add_tdc_hit1(&mut self, pack: &Packet, frame_tdc: &mut TdcRef, settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        if self.timer.elapsed().as_millis() < TIME_INTERVAL_FRAMES {
            self.is_ready = false;
            if !settings.cumul {
                self.data.iter_mut().for_each(|x| *x = 0);
            }
        } else {
            self.is_ready = true;
        }
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, settings: &Settings) {
        self.is_ready = false;
        self.timer = Instant::now();
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = 0);
        }
    }
}

pub struct Coincidence2D {
    data: Vec<u32>,
    aux_data: Vec<TIME>,
    //electrons: CollectionElectron,
    //photons: CollectionPhoton,
    //indexes: Vec<usize>,
    //min_index: usize,
    timer: Instant,
}

impl SpecKind for Coincidence2D {
    fn is_ready(&self) -> bool {
        self.timer.elapsed().as_millis() > TIME_INTERVAL_COINCIDENCE_HISTOGRAM
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        /*
        self.electrons.sort();
        self.photons.sort();
        let (coinc_electron, coinc_photon) = self.electrons.search_coincidence(&self.photons, &mut self.indexes, &mut self.min_index, settings.time_delay, settings.time_width);
        //println!("electrons and photons in coincidence: {:?} and {:?}", coinc_electron.len(), coinc_photon.len());
        coinc_electron.iter().zip(coinc_photon.iter()).for_each(|(ele, pho)| {
            let delay = (pho.time() / 6 - settings.time_delay + settings.time_width - ele.time()) as POSITION;
            let index = ele.x() + delay * CAM_DESIGN.0;
            add_index!(self, index);
        });
        println!("{} and {}", self.electrons.len(), coinc_electron.len());
        self.min_index = 0;
        self.electrons.clear();
        self.photons.clear();
        */
        aux_func::as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len = 2*settings.time_width as usize * CAM_DESIGN.0 as usize;
        let temp_vec = vec![0; len];
        Self { data: temp_vec, aux_data: vec![0; LIST_SIZE_AUX_EVENTS], timer: Instant::now()}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Packet, settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        let etime = pack.electron_time();
        for phtime in self.aux_data.iter() {
            if (*phtime < etime + settings.time_delay + settings.time_width) && (etime + settings.time_delay < *phtime + settings.time_width) {
                let delay = (phtime - settings.time_delay + settings.time_width - etime) as POSITION;
                let index = pack.x() + delay * CAM_DESIGN.0;
                add_index!(self, index);
                return
            }
        }
        //let electron = SingleElectron::new(pack, None, 0);
        //self.electrons.add_electron(electron);
    }
    fn add_tdc_hit2(&mut self, pack: &Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        self.aux_data.push(pack.tdc_time_norm());
        self.aux_data.remove(0);
        //let photon = SinglePhoton::new(pack, 0, None, 0);
        //self.photons.add_photon(photon);
        
    }
    fn add_tdc_hit1(&mut self, pack: &Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, settings: &Settings) {
        self.timer = Instant::now();
        //self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = 0);
        }
    }
}

pub struct FastChrono {
    data: Vec<u32>,
    is_ready: bool,
    global_stop: bool,
}

impl SpecKind for FastChrono {
    fn is_ready(&self) -> bool {
        self.is_ready && !self.global_stop
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        aux_func::as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len = (settings.xspim_size*CAM_DESIGN.0) as usize;
        let data = vec![0; len];
        Self{ data, is_ready: false, global_stop: false}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Packet, settings: &Settings, frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        let line = (frame_tdc.counter()/2) as POSITION;
        let index = pack.x() + line * CAM_DESIGN.0;
        if line < settings.xspim_size {
            add_index!(self, index);
        }
    }
    fn add_tdc_hit2(&mut self, pack: &Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn add_tdc_hit1(&mut self, pack: &Packet, frame_tdc: &mut TdcRef, settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        self.is_ready = (frame_tdc.counter()/2) as POSITION > settings.xspim_size;
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, _settings: &Settings) {
        self.global_stop = true;
    }
}

pub struct Chrono {
    data: Vec<u32>,
    aux_data: Vec<TIME>,
    is_ready: bool,
}

impl SpecKind for Chrono {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        aux_func::as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len = (settings.xspim_size*CAM_DESIGN.0) as usize;
        let data = vec![0; len];
        Self{ data, aux_data: Vec::new(), is_ready: false}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: &Packet, settings: &Settings, frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        let line = (frame_tdc.counter()/2) as POSITION % settings.xspim_size;
        let index = pack.x() + line * CAM_DESIGN.0;
        add_index!(self, index);
    }
    fn add_tdc_hit1(&mut self, pack: &Packet, frame_tdc: &mut TdcRef, settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
        let line = (frame_tdc.counter() / 2) as POSITION;
        self.is_ready = line % 20 == 0; //Every 20 lines send chrono;
        if line % settings.xspim_size == 0 {
            self.aux_data.push(0); //This indicates the frame must be refreshed;
        }
    }
    fn add_tdc_hit2(&mut self, pack: &Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, _settings: &Settings) {
        self.is_ready = false;
        if self.aux_data.len() > 0 { //Refresh frame if true;
            self.aux_data.pop(); //Remove for the next cycle;
            self.data.iter_mut().for_each(|x| *x = 0);
        }
    }
}
pub struct Live2DFrame {
    data: Vec<u32>,
    is_ready: bool,
    timer: Instant,
    shutter: Option<ShutterControl>,
}

impl SpecKind for Live2DFrame {
    fn is_ready(&self) -> bool {
        self.is_ready && (self.timer.elapsed().as_millis() > TIME_INTERVAL_FRAMES)
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        aux_func::as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        Self{ data: tp3_vec!(2), is_ready: false, timer: Instant::now(), shutter: Some(ShutterControl::default())}
    }

    #[inline]
    fn add_electron_hit(&mut self, pack: &Packet, settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        if !self.is_ready || settings.cumul {
            let index = pack.x() + CAM_DESIGN.0 * pack.y();
            self.data[index as usize] += pack.tot() as u32;
        }
    }
    fn add_tdc_hit2(&mut self, pack: &Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn build_main_tdc<V: TimepixRead>(&mut self, _pack: &mut V, _my_settings: &Settings, _file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_no_read(TdcType::TdcOneRisingEdge)
    }
    fn add_tdc_hit1(&mut self, pack: &Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
    }
    fn add_shutter_hit(&mut self, pack: &Packet, _frame_tdc: &mut TdcRef, settings: &Settings) {
        let temp_ready = self.shutter.as_mut().unwrap().try_set_time(pack.frame_time(), pack.ci(), pack.tdc_type() == 10);
        if !self.is_ready {
            self.is_ready = temp_ready;
            if self.is_ready {
                if self.timer.elapsed().as_millis() < TIME_INTERVAL_FRAMES {
                    self.is_ready = false;
                    if !settings.cumul { //No cumulation
                        self.data.iter_mut().for_each(|x| *x = 0);
                    }
                } else {
                    self.is_ready = true;
                }
            }
        } else {
            if temp_ready {
                if !settings.cumul { //No cumulation
                    self.data.iter_mut().for_each(|x| *x = 0);
                }
                self.is_ready = false;
            }
        }
    }

    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, _settings: &Settings) {
        self.timer = Instant::now();
    }
    fn shutter_control(&self) -> Option<&ShutterControl> {
        self.shutter.as_ref()
    }
}

pub struct Live1DFrame {
    data: Vec<u32>,
    is_ready: bool,
    timer: Instant,
    shutter: Option<ShutterControl>,
}

impl SpecKind for Live1DFrame {
    fn is_ready(&self) -> bool {
        self.is_ready && (self.timer.elapsed().as_millis() > TIME_INTERVAL_FRAMES)
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        aux_func::as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        Self{ data: tp3_vec!(1), is_ready: false, timer: Instant::now(), shutter: Some(ShutterControl::default())}
    }

    #[inline]
    fn add_electron_hit(&mut self, pack: &Packet, settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        //If you are in cumulation mode, save all the electrons. If not, only save those that the
        //frame has not yet been sent
        if !self.is_ready || settings.cumul{
            let index = pack.x();
            self.data[index as usize] += pack.tot() as u32;
        }
    }
    fn add_tdc_hit2(&mut self, pack: &Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn build_main_tdc<V: TimepixRead>(&mut self, _pack: &mut V, _my_settings: &Settings, _file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_no_read(TdcType::TdcOneRisingEdge)
    }
    fn add_tdc_hit1(&mut self, pack: &Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
    }
    fn add_shutter_hit(&mut self, pack: &Packet, _frame_tdc: &mut TdcRef, settings: &Settings) {
        let temp_ready = self.shutter.as_mut().unwrap().try_set_time(pack.frame_time(), pack.ci(), pack.tdc_type() == 10);
        //If is_ready is false, set with temp_ready. If is_ready is true and another temp_ready
        //arrives, then we reset the array and do not send the frame. In this mode,
        //reset_or_else does not set is_ready to false.
        if !self.is_ready {
            self.is_ready = temp_ready;
        } else {
            if temp_ready {
                if !settings.cumul {
                    self.data.iter_mut().for_each(|x| *x = 0);
                }
                self.is_ready = false;
            }
        }
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, _settings: &Settings) {
        //Empty. The shutter control defines the behaviour and not the TCP stack, but we reset the
        //timer. We also reset the counter in case we use it for high dynamic measurements
        self.timer = Instant::now();
    }
    fn shutter_control(&self) -> Option<&ShutterControl> {
        self.shutter.as_ref()
    }
}

pub struct Live1DFrameHyperspec {
    data: Vec<u32>,
    is_ready: bool,
    timer: Instant,
    shutter: Option<ShutterControl>,
}

impl SpecKind for Live1DFrameHyperspec {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        //The number of pixels sent is updated on the reset or else function
        let range = self.shutter.as_ref().expect("This mode must have the Shutter Control").get_index_range_to_send();
        aux_func::as_bytes(&self.data[range])
    }
    fn new(settings: &Settings) -> Self {
        let len = (CAM_DESIGN.0 * settings.xscan_size * settings.yscan_size) as usize;
        let mut shutter = ShutterControl::default();
        shutter.set_as_hyperspectral();
        Self{ data: vec![0; len], is_ready: false, timer: Instant::now(), shutter: Some(shutter)}
    }

    #[inline]
    fn add_electron_hit(&mut self, pack: &Packet, _settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        let shut = self.shutter.as_ref().unwrap();
        if shut.is_hyperspectral_complete() { return }
        let frame_number = shut.get_counter()[pack.ci() as usize] as POSITION;
        //We cannot depass frame_number otherwise the indexation will be bad
        let index = frame_number * CAM_DESIGN.0 + pack.x();
        self.data[index as usize] += pack.tot() as u32;
    }
    fn build_main_tdc<V: TimepixRead>(&mut self, _pack: &mut V, _my_settings: &Settings, _file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_no_read(TdcType::TdcOneRisingEdge)
    }
    fn add_tdc_hit2(&mut self, _pack: &Packet, _settings: &Settings, _ref_tdc: &mut TdcRef) {}
    fn add_tdc_hit1(&mut self, pack: &Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(pack.tdc_time(), pack.tdc_counter());
    } 
    fn add_shutter_hit(&mut self, pack: &Packet, _frame_tdc: &mut TdcRef, settings: &Settings) {
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
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, _settings: &Settings) {
        self.is_ready = false;
        self.timer = Instant::now();
    }
    fn shutter_control(&self) -> Option<&ShutterControl> {
        self.shutter.as_ref()
    }
}



impl IsiBoxKind for Live1D {
    fn isi_new(_settings: &Settings) -> Self {
        let len = (CAM_DESIGN.0 + CHANNELS as POSITION) as usize;
        Self{ data: vec![0; len], is_ready: false,timer: Instant::now()}
    }
    fn append_from_isi(&mut self, ext_data: &[u32]) {
        self.data[CAM_DESIGN.0 as usize..].iter_mut().zip(ext_data.iter()).for_each(|(a, b)| *a+=b);
    }
}

/*
///Reads timepix3 socket and writes in the output socket a header and a full frame (binned or not). A periodic tdc is mandatory in order to define frame time.
///
///# Examples
pub fn run_spectrum<V, U, Y>(mut pack: V, ns: U, my_settings: Settings, kind: Y, mut file_to_write: FileManager) -> Result<u8, Tp3ErrorKind>
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
            let frame_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let aux_tdc = measurement.build_aux_tdc()?;
            build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement)?;
        },
        2 => {
            let mut measurement = kind.gen16(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let aux_tdc = measurement.build_aux_tdc()?;
            build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement)?;
        },
        4 => {
            let mut measurement = kind.gen32(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let aux_tdc = measurement.build_aux_tdc()?;
            build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement)?;
        },
        _ => {return Err(Tp3ErrorKind::SetByteDepth)},
    }
    
    Ok(my_settings.mode)
}
*/

pub fn build_spectrum<V, U, W>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut frame_tdc: TdcRef, mut ref_tdc: TdcRef, mut meas_type: W, mut file_to_write: FileManager) -> Result<(), Tp3ErrorKind> 
    where V: TimepixRead,
          U: Write,
          W: SpecKind
{

    let mut last_ci = 0;
    let mut buffer_pack_data: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let start = Instant::now();
    
    while let Ok(size) = pack_sock.read_timepix(&mut buffer_pack_data) {
        file_to_write.write_all(&buffer_pack_data[0..size])?;
        if build_data(&buffer_pack_data[0..size], &mut meas_type, &mut last_ci, &my_settings, &mut frame_tdc, &mut ref_tdc) {
            let msg = create_header(&my_settings, &frame_tdc, 0, meas_type.shutter_control());
            if ns_sock.write(&msg).is_err() {println!("Client disconnected on header."); break;}
            if ns_sock.write(meas_type.build_output(&my_settings)).is_err() {println!("Client disconnected on data."); break;}
            meas_type.reset_or_else(&frame_tdc, &my_settings);
            if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());};
        }
    }
    println!("Total elapsed time is: {:?}.", start.elapsed());
    Ok(())

}

pub fn build_spectrum_isi<V, U, W>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut frame_tdc: TdcRef, mut ref_tdc: TdcRef, mut meas_type: W) -> Result<(), Tp3ErrorKind> 
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
            let result = meas_type.build_output(&my_settings);
            if ns_sock.write(result).is_err() {println!("Client disconnected on data."); break;}
            meas_type.reset_or_else(&frame_tdc, &my_settings);
            if frame_tdc.counter() % 1000 == 0 { let elapsed = start.elapsed(); println!("Total elapsed time is: {:?}. Counter is {}.", elapsed, frame_tdc.counter());};
        }
    }
    handler.stop_threads();
    println!("Total elapsed time is: {:?}.", start.elapsed());
    Ok(())
}


fn build_data<W: SpecKind>(data: &[u8], final_data: &mut W, last_ci: &mut u8, settings: &Settings, frame_tdc: &mut TdcRef, ref_tdc: &mut TdcRef) -> bool {

    let iterator = data.chunks_exact(8);
    
    for x in iterator {
        match *x {
            [84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
            _ => {
                let packet = Packet { chip_index: *last_ci, data: packet_change(x)[0]};
                
                match packet.id() {
                    11 | 10 => { //Event or frame based
                        final_data.add_electron_hit(&packet, settings, frame_tdc, ref_tdc);
                    },
                    6 if packet.tdc_type() == frame_tdc.id() => { //Tdc value 1
                        final_data.add_tdc_hit1(&packet, frame_tdc, settings);
                    },
                    6 if packet.tdc_type() == ref_tdc.id() => { //Tdc value 2
                        final_data.add_tdc_hit2(&packet, settings, ref_tdc);
                    },
                    5 if packet.tdc_type() == 10 || packet.tdc_type() == 15  => { //Shutter value.
                        final_data.add_shutter_hit(&packet, frame_tdc, settings);
                    },
                    _ => {},
                };
            },
        };
    };
    final_data.is_ready()
}

//fn add_isibox_pixels(data: &mut [u8], isi_box_data: [u32; 17]) {
//    data[CAM_DESIGN.0..].iter_mut().zip(as_bytes(&isi_box_data).iter()).for_each(|(a, b)| *a+=b);
//}

fn create_header(set: &Settings, tdc: &TdcRef, extra_pixels: POSITION, shutter_control: Option<&ShutterControl>) -> Vec<u8> {
    let mut msg: String = String::from("{\"timeAtFrame\":");
    msg.push_str(&(tdc.time().to_string()));
    msg.push_str(",\"frameNumber\":");
    if let Some(shutter) = shutter_control {
        if set.mode == 11 {//Frame-based hyperspectral image
            msg.push_str(&((shutter.get_start_pixel()).to_string()));
        } else {
            msg.push_str(&((shutter.get_counter()[0]).to_string()));
        }
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

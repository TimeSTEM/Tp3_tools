//!`speclib` is a collection of tools to set EELS/4D acquisition.

use crate::packetlib::Packet;
use crate::auxiliar::{Settings, misc::{TimepixRead, as_bytes, packet_change, check_if_in}};
use crate::tdclib::{TdcRef, isi_box, isi_box::{IsiBoxTools, IsiBoxHand}};
use crate::isi_box_new;
use crate::errorlib::Tp3ErrorKind;
use crate::clusterlib::cluster::{SinglePhoton, CollectionPhoton, SingleElectron, CollectionElectron};
use std::time::Instant;
use std::io::Write;
use crate::auxiliar::{value_types::*, FileManager, misc};
use crate::constlib::*;
//use rayon::prelude::*;

const CAM_DESIGN: (POSITION, POSITION) = Packet::chip_array();

#[derive(Default)]
pub struct ShutterControl {
    time: [TIME; 4],
    counter: [COUNTER; 4],
    is_2d: bool,
    hyperspectral: bool,
    hyperspectral_complete: bool,
    hyperspec_pixels_to_send: (POSITION, POSITION), //Start and end pixel that will be sent
    shutter_closed_status: [bool; 4],
}

impl ShutterControl {
    fn try_set_time(&mut self, timestamp: TIME, ci: u8, shutter_closed: bool) -> bool {
        //When shutter_closed is true, we receive electrons as packets. Shutter_closed false means a new frame just
        //started, and we must wait <ACQUISITION_TIME> in order to shutter to close and receive our data.
        let ci = ci as usize;
        self.shutter_closed_status[ci] = shutter_closed;
        if !shutter_closed && self.time[ci] != timestamp {
            //first false (new frame) in which all timestemps differ is the frame_condition. The shutter just
            //opened in one chip so electrons are not arriving as packets anymore. Frame is ready.
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
    fn set_as_hyperspectral(&mut self, is_2d: bool) {
        self.hyperspectral = true;
        self.is_2d = is_2d;
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
        if !self.is_2d {
            (start_pixel * CAM_DESIGN.0) as usize..(end_pixel * CAM_DESIGN.0) as usize
        } else {
            (start_pixel * CAM_DESIGN.0 * CAM_DESIGN.1) as usize..(end_pixel * CAM_DESIGN.0 * CAM_DESIGN.1) as usize
        }
    }
    fn get_data_size_to_send(&self) -> usize {
        let (start_pixel, end_pixel) = self.hyperspec_pixels_to_send;
        if !self.is_2d {
            ((end_pixel - start_pixel) * CAM_DESIGN.0) as usize
        } else {
            ((end_pixel - start_pixel) * CAM_DESIGN.0 * CAM_DESIGN.1) as usize
        }
    }
    fn get_counter(&self) -> [COUNTER; 4] {
        self.counter
    }
}

pub trait SpecKind {
    fn is_ready(&self) -> bool;
    fn build_output(&mut self, settings: &Settings) -> &[u8];
    fn new(settings: &Settings) -> Self;
    fn build_main_tdc<V: TimepixRead>(&self, _pack: &mut V, _my_settings: &Settings, _file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_no_read(MAIN_TDC)
    }
    fn build_aux_tdc<V: TimepixRead>(&self, _pack: &mut V, _my_settings: &Settings, _file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_no_read(SECONDARY_TDC)
    }
    fn add_electron_hit(&mut self, pack: Packet, settings: &Settings, frame_tdc: &TdcRef, ref_tdc: &TdcRef);
    fn add_tdc_hit1(&mut self, pack: Packet, frame_tdc: &mut TdcRef, _settings: &Settings);
    fn add_tdc_hit2(&mut self, pack: Packet, settings: &Settings, ref_tdc: &mut TdcRef);
    fn add_shutter_hit(&mut self, _pack: Packet, _frame_tdc: &mut TdcRef, _settings: &Settings) {}
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, settings: &Settings);
    fn shutter_control(&self) -> Option<&ShutterControl> {None}
    fn get_frame_counter(&self, tdc_value: &TdcRef) -> COUNTER {
        tdc_value.counter() / 2
    }
    fn data_size_in_bytes(&self) -> usize;
    fn data_height(&self) -> COUNTER;
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
    frame_counter: COUNTER,
    last_time: TIME,
    timer: Instant,
}

impl SpecKind for Live2D {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        Self{ data: tp3_vec!(2), is_ready: false, frame_counter: 0, last_time: 0, timer: Instant::now() }
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: Packet, settings: &Settings, _frame_tdc: &TdcRef, ref_tdc: &TdcRef) {
        let index = pack.x() + CAM_DESIGN.0 * pack.y();
        if settings.time_resolved {
            if ref_tdc.tr_electron_check_if_in(&pack, settings).is_some() {
                add_index!(self, index);
            } 
        } else {
            add_index!(self, index);
        }
        
        let ele_time = pack.fast_electron_time();
        //This is an overflow. We correct it.
        if self.last_time > ele_time + (ELECTRON_OVERFLOW >> 2) {
            self.last_time = ele_time;
        }
        //We check if the frame must be ready or not.
        if ele_time > self.last_time + settings.acquisition_us * 640 {
            self.last_time = ele_time;
            self.frame_counter += 1;
            if self.timer.elapsed().as_millis() < TIME_INTERVAL_FRAMES {
                self.is_ready = false;
                if !settings.cumul {
                    self.data.iter_mut().for_each(|x| *x = 0);
                }
            } else {
                self.is_ready = true;
            }
        }
    }
    fn build_aux_tdc<V: TimepixRead>(&self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        if my_settings.time_resolved {
            TdcRef::new_periodic(SECONDARY_TDC, pack, my_settings, file_to_write)
        } else {
            TdcRef::new_no_read(SECONDARY_TDC)
        }
    }
    fn add_tdc_hit2(&mut self, pack: Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(&pack);
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn add_tdc_hit1(&mut self, pack: Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(&pack);
        add_index!(self, CAM_DESIGN.0-2);
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, settings: &Settings) {
        self.timer = Instant::now();
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = 0);
        }
    }
    fn get_frame_counter(&self, _tdc_value: &TdcRef) -> COUNTER {
        self.frame_counter
    }
    fn data_size_in_bytes(&self) -> usize {
        misc::vector_len_in_bytes(&self.data)
    }
    fn data_height(&self) -> COUNTER {
        CAM_DESIGN.1
    }
}

pub struct Live1D {
    data: Vec<u32>,
    is_ready: bool,
    frame_counter: COUNTER,
    last_time: TIME,
    timer: Instant,
}

impl SpecKind for Live1D {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        Self{ data: tp3_vec!(1), is_ready: false, frame_counter: 0, last_time: 0, timer: Instant::now()}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: Packet, settings: &Settings, _frame_tdc: &TdcRef, ref_tdc: &TdcRef) {
        let index = pack.x();
        if settings.time_resolved {
            if ref_tdc.tr_electron_check_if_in(&pack, settings).is_some() {
                add_index!(self, index);
            } 
        } else {
            add_index!(self, index);
        }
        
        let ele_time = pack.fast_electron_time();
        //This is an overflow. We correct it.
        if self.last_time > ele_time + (ELECTRON_OVERFLOW >> 2) {
            self.last_time = ele_time;
        }
        //We check if the frame must be ready or not.
        if ele_time > self.last_time + settings.acquisition_us * 640 {
            self.last_time = ele_time;
            self.frame_counter += 1;
            if self.timer.elapsed().as_millis() < TIME_INTERVAL_FRAMES {
                self.is_ready = false;
                if !settings.cumul {
                    self.data.iter_mut().for_each(|x| *x = 0);
                }
            } else {
                self.is_ready = true;
            }
        }
    }
    fn build_aux_tdc<V: TimepixRead>(&self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        if my_settings.time_resolved {
            TdcRef::new_periodic(SECONDARY_TDC, pack, my_settings, file_to_write)
        } else {
            TdcRef::new_no_read(SECONDARY_TDC)
        }
    }
    fn add_tdc_hit2(&mut self, pack: Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(&pack);
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn add_tdc_hit1(&mut self, pack: Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(&pack);
        add_index!(self, CAM_DESIGN.0-2);
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, settings: &Settings) {
        self.timer = Instant::now();
        self.is_ready = false;
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = 0);
        }
    }
    fn get_frame_counter(&self, _tdc_value: &TdcRef) -> COUNTER {
        self.frame_counter
    }
    fn data_size_in_bytes(&self) -> usize {
        misc::vector_len_in_bytes(&self.data)
    }
    fn data_height(&self) -> COUNTER {
        1
    }
}

//Same implementation as the post-processing data. Currently not used.
pub struct Coincidence2DV3 {
    data: Vec<u32>,
    electron_buffer: CollectionElectron,
    photon_buffer: CollectionPhoton,
    timer: Instant,
}

impl SpecKind for Coincidence2DV3 {
    fn is_ready(&self) -> bool {
        self.timer.elapsed().as_millis() > TIME_INTERVAL_COINCIDENCE_HISTOGRAM
    }
    fn build_output(&mut self, settings: &Settings) -> &[u8] {
        let mut rpi = Vec::new();
        let mut index = 0;
        self.electron_buffer.sort();
        self.photon_buffer.sort();
        let (coinc_electron, coinc_photon) = self.electron_buffer.search_coincidence(&mut self.photon_buffer, &mut rpi, &mut index, settings.time_delay, settings.time_width);
        coinc_electron.iter().zip(coinc_photon.iter()).for_each(|(ele, photon)| {
            let delay = (photon.time() / 6 - settings.time_delay + settings.time_width - ele.time()) as POSITION;
            let index = ele.x() + delay * CAM_DESIGN.0;
            self.data[index as usize] += 1;
        });
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len = 4*settings.time_width as usize * CAM_DESIGN.0 as usize;
        let temp_vec = vec![0; len];
        Self { data: temp_vec, electron_buffer: CollectionElectron::new(), photon_buffer: CollectionPhoton::new(), timer: Instant::now()}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: Packet, _settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        let se = SingleElectron::new(pack, None, 0);
        self.electron_buffer.add_electron(se);
    }
    fn add_tdc_hit2(&mut self, pack: Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(&pack);
        let sp = SinglePhoton::new(pack, 1, None, 0);
        self.photon_buffer.add_photon(sp);
    }
    fn add_tdc_hit1(&mut self, pack: Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(&pack);
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, _settings: &Settings) {
        self.timer = Instant::now();
        self.electron_buffer.clear();
        self.photon_buffer.clear();
    }
    fn data_size_in_bytes(&self) -> usize {
        misc::vector_len_in_bytes(&self.data)
    }
    fn data_height(&self) -> COUNTER {
        self.data.len() as COUNTER / CAM_DESIGN.0
    }
}

//Circular buffer for the elctrons. Currently not used. This is similar to what is done in the
//FPGA.
pub struct Coincidence2DV2 {
    data: Vec<u32>,
    electron_buffer: Vec<(TIME, POSITION)>,
    photon_buffer: Vec<TIME>,
    index: usize,
    timer: Instant,
}

impl SpecKind for Coincidence2DV2 {
    fn is_ready(&self) -> bool {
        self.timer.elapsed().as_millis() > TIME_INTERVAL_COINCIDENCE_HISTOGRAM
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len = 4*settings.time_width as usize * CAM_DESIGN.0 as usize;
        let temp_vec = vec![0; len];
        Self { data: temp_vec, electron_buffer: vec![(0, 0); CIRCULAR_BUFFER], photon_buffer: vec![0; LIST_SIZE_AUX_EVENTS], timer: Instant::now(), index: 0}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: Packet, _settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        self.electron_buffer[self.index] = (pack.electron_time_in_tdc_units(), pack.x());
        self.index += 1;
        self.index %= CIRCULAR_BUFFER;
    }
    fn add_tdc_hit2(&mut self, pack: Packet, settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(&pack);
        self.photon_buffer.push(pack.tdc_time_abs_norm());
        self.photon_buffer.remove(0);

        for phtime in &self.photon_buffer {
            for ele in &mut self.electron_buffer {
                if (*phtime < ele.0 + settings.time_delay + settings.time_width) &&
                    (ele.0 + settings.time_delay < phtime + settings.time_width) {
                        let delay = (phtime - settings.time_delay + settings.time_width - ele.0) as POSITION;
                        let index = ele.1 + delay * CAM_DESIGN.0;
                        *ele = (0, 0); //this electron should not appear again. It is already send.
                        self.data[index as usize] += 1;
                }
            }
        }

        /*
        //This is a rayon implementation

        let all_indices_to_add: Vec<POSITION> = self.photon_buffer.par_iter()
            .flat_map(|phtime| {
                self.electron_buffer.iter()
                    .filter_map(|ele| {
                        if (*phtime < ele.0 + settings.time_delay + settings.time_width)
                            && (ele.0 + settings.time_delay < phtime + settings.time_width)
                        {
                            let delay = (phtime - settings.time_delay + settings.time_width - ele.0) as POSITION;
                            let index = ele.1 + delay * CAM_DESIGN.0;
                            *ele = (0, 0); // This part needs to be handled carefully
                            Some(index)
                        } else {
                            None
                        }
                    })
                .collect::<Vec<_>>() // Collect the results for this photon
            })
        .collect(); // Collect all indices across all photons
        
        for index in all_indices_to_add {
            self.data[index as usize] += 1;
        }
        */
        

    }
    fn add_tdc_hit1(&mut self, pack: Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(&pack);
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, _settings: &Settings) {
        self.timer = Instant::now();
    }
    fn data_size_in_bytes(&self) -> usize {
        misc::vector_len_in_bytes(&self.data)
    }
    fn data_height(&self) -> COUNTER {
        self.data.len() as COUNTER / CAM_DESIGN.0
    }
}


pub struct Coincidence2D {
    data: Vec<u32>,
    aux_data: Vec<TIME>,
    aux_data2: Vec<TIME>,
    timer: Instant,
}

impl SpecKind for Coincidence2D {
    fn is_ready(&self) -> bool {
        //Be careful with the timer. This is quite slow.
        self.timer.elapsed().as_millis() > TIME_INTERVAL_COINCIDENCE_HISTOGRAM
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len = 4*settings.time_width as usize * CAM_DESIGN.0 as usize;
        let temp_vec = vec![0; len];
        Self { data: temp_vec, aux_data: vec![0; LIST_SIZE_AUX_EVENTS], aux_data2: vec![0; LIST_SIZE_AUX_EVENTS], timer: Instant::now()}
    }
    //This func gets electrons in which the TDC has already arrived by TCP. So it could be
    //electrons second tcp, first time, or electrons second tdc, second time.
    #[inline]
    fn add_electron_hit(&mut self, pack: Packet, settings: &Settings, frame_tdc: &TdcRef, ref_tdc: &TdcRef) {
        let etime = pack.electron_time_in_tdc_units();
        if settings.time_resolved {
            if let Some(phtime) = frame_tdc.tr_electron_check_if_in(&pack, settings) {
                let delay = (phtime - settings.time_delay + settings.time_width - etime) as POSITION;
                let index = pack.x() + delay * CAM_DESIGN.0 + 2*settings.time_width as u32 * CAM_DESIGN.0;
                add_index!(self, index);
            }
            if let Some(phtime) = ref_tdc.tr_electron_check_if_in(&pack, settings) {
                if let Some(etime) = ref_tdc.tr_electron_correct_by_blanking(&pack) {
                    let delay = (phtime - settings.time_delay + settings.time_width - etime) as POSITION;
                    let index = pack.x() + delay * CAM_DESIGN.0;
                    add_index!(self, index);
                }
            }
        } else {
            for phtime in self.aux_data.iter() {
                if check_if_in(&etime, phtime, settings) {
                    let delay = (phtime - settings.time_delay + settings.time_width - etime) as POSITION;
                    let index = pack.x() + delay * CAM_DESIGN.0;
                    add_index!(self, index);
                }
            }
            for phtime in self.aux_data2.iter() {
                if check_if_in(&etime, phtime, settings) {
                    let delay = (phtime - settings.time_delay + settings.time_width - etime) as POSITION;
                    let index = pack.x() + delay * CAM_DESIGN.0 + 2*settings.time_width as u32 * CAM_DESIGN.0;
                    add_index!(self, index);
                }
            }
        }
    }
    fn build_aux_tdc<V: TimepixRead>(&self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        if my_settings.time_resolved {
            TdcRef::new_periodic(SECONDARY_TDC, pack, my_settings, file_to_write)
        } else {
            TdcRef::new_no_read(SECONDARY_TDC)
        }
    }
    fn build_main_tdc<V: TimepixRead>(&self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        if my_settings.time_resolved {
            TdcRef::new_periodic(MAIN_TDC, pack, my_settings, file_to_write)
        } else {
            TdcRef::new_no_read(MAIN_TDC)
        }
    }
    fn add_tdc_hit2(&mut self, pack: Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(&pack);
        self.aux_data.push(pack.tdc_time_abs_norm());
        self.aux_data.remove(0);
    }
    fn add_tdc_hit1(&mut self, pack: Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(&pack);
        self.aux_data2.push(pack.tdc_time_abs_norm());
        self.aux_data2.remove(0);
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, settings: &Settings) {
        self.timer = Instant::now();
        if !settings.cumul {
            self.data.iter_mut().for_each(|x| *x = 0);
        }
    }
    fn data_size_in_bytes(&self) -> usize {
        misc::vector_len_in_bytes(&self.data)
    }
    fn data_height(&self) -> COUNTER {
        self.data.len() as COUNTER / CAM_DESIGN.0
    }
}

pub struct Chrono {
    data: Vec<u32>,
    last_time: TIME,
    frame_counter: COUNTER,
    current_line: COUNTER,
    timer: Instant,
}

impl SpecKind for Chrono {
    fn is_ready(&self) -> bool {
        self.timer.elapsed().as_millis() > TIME_INTERVAL_FRAMES
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len = (settings.xspim_size*CAM_DESIGN.0) as usize;
        let data = vec![0; len];
        Self{ data, last_time: 0, frame_counter: 0, current_line: 0, timer: Instant::now()}
    }
    #[inline]
    fn add_electron_hit(&mut self, pack: Packet, settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        
        let ele_time = pack.fast_electron_time();
        //This is an overflow. We correct it.
        if self.last_time > ele_time + (ELECTRON_OVERFLOW >> 2) {
            self.last_time = ele_time;
        }
        
        //We check for a new line and if true we erase it.
        if ele_time > self.last_time + settings.acquisition_us * 640 {
            self.last_time = ele_time;
            self.frame_counter += 1;
            self.current_line = self.frame_counter % settings.xspim_size;

            let start = ((self.frame_counter % settings.xspim_size) * CAM_DESIGN.0) as usize;
            let end = start + PIXELS_X as usize;
            self.data[start..end].iter_mut().for_each(|x| *x = 0);
        }

        //We determine the current line
        let index = pack.x() + self.current_line * CAM_DESIGN.0;
        add_index!(self, index);

    }
    fn add_tdc_hit2(&mut self, pack: Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(&pack);
    }
    fn add_tdc_hit1(&mut self, pack: Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(&pack);
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, _settings: &Settings) {
        self.timer = Instant::now();
    }
    fn get_frame_counter(&self, _tdc_value: &TdcRef) -> COUNTER {
        self.frame_counter
    }
    fn data_size_in_bytes(&self) -> usize {
        misc::vector_len_in_bytes(&self.data)
    }
    fn data_height(&self) -> COUNTER {
        self.data.len() as COUNTER / CAM_DESIGN.0
    }
}

pub struct ChronoFrame {
    data: Vec<u32>,
    frame_counter: COUNTER,
    current_line: COUNTER,
    timer: Instant,
    shutter: Option<ShutterControl>,
}

impl SpecKind for ChronoFrame {
    fn is_ready(&self) -> bool {
        self.timer.elapsed().as_millis() > TIME_INTERVAL_FRAMES
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(settings: &Settings) -> Self {
        let len = (settings.xspim_size*CAM_DESIGN.0) as usize;
        let shutter = ShutterControl::default();
        Self{ data: vec![0; len], frame_counter: 0, current_line: 0, timer: Instant::now(), shutter: Some(shutter)}
    }

    #[inline]
    fn add_electron_hit(&mut self, pack: Packet, settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        let shut = self.shutter.as_ref().unwrap();
        let frame_number = shut.get_counter()[pack.ci() as usize] as POSITION;
        
        //If a new frame, we update and delete the current line in Chrono.
        if frame_number > self.frame_counter {
            self.frame_counter = frame_number;
            self.current_line = self.frame_counter % settings.xspim_size;
            
            let start = ((self.frame_counter % settings.xspim_size) * CAM_DESIGN.0) as usize;
            let end = start + PIXELS_X as usize;
            self.data[start..end].iter_mut().for_each(|x| *x = 0);
        }
        //We determine the current line
        let index = pack.x() + self.current_line * CAM_DESIGN.0;
        self.data[index as usize] += pack.tot() as u32;
    }
    fn add_tdc_hit2(&mut self, pack: Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(&pack);
    }
    fn add_tdc_hit1(&mut self, pack: Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(&pack);
    }
    fn add_shutter_hit(&mut self, pack: Packet, _frame_tdc: &mut TdcRef, _settings: &Settings) {
        self.shutter.as_mut().unwrap().try_set_time(pack.frame_time(), pack.ci(), pack.tdc_type() == 10);
    }
    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, _settings: &Settings) {
        self.timer = Instant::now();
    }
    fn shutter_control(&self) -> Option<&ShutterControl> {
        self.shutter.as_ref()
    }
    fn get_frame_counter(&self, _tdc_value: &TdcRef) -> COUNTER {
        self.frame_counter
    }
    fn data_size_in_bytes(&self) -> usize {
        misc::vector_len_in_bytes(&self.data)
    }
    fn data_height(&self) -> COUNTER {
        self.data.len() as COUNTER / CAM_DESIGN.0
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
        self.is_ready && self.timer.elapsed().as_millis() > TIME_INTERVAL_FRAMES
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        Self{ data: tp3_vec!(2), is_ready: false, timer: Instant::now(), shutter: Some(ShutterControl::default())}
    }

    #[inline]
    fn add_electron_hit(&mut self, pack: Packet, settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        if !self.is_ready || settings.cumul {
            let index = pack.x() + CAM_DESIGN.0 * pack.y();
            self.data[index as usize] += pack.tot() as u32;
        }
    }
    fn add_tdc_hit2(&mut self, pack: Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(&pack);
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn add_tdc_hit1(&mut self, pack: Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(&pack);
        add_index!(self, CAM_DESIGN.0-2);
    }
    fn add_shutter_hit(&mut self, pack: Packet, _frame_tdc: &mut TdcRef, settings: &Settings) {
        let temp_ready = self.shutter.as_mut().unwrap().try_set_time(pack.frame_time(), pack.ci(), pack.tdc_type() == 10);
        
        if !self.is_ready {
            self.is_ready = temp_ready;
        } else if temp_ready {
            if !settings.cumul {
                self.data.iter_mut().for_each(|x| *x = 0);
            }
            self.is_ready = false;
        }
    }

    fn reset_or_else(&mut self, _frame_tdc: &TdcRef, _settings: &Settings) {
        self.timer = Instant::now();
    }
    fn shutter_control(&self) -> Option<&ShutterControl> {
        self.shutter.as_ref()
    }
    fn get_frame_counter(&self, _tdc_value: &TdcRef) -> COUNTER {
        self.shutter.as_ref().unwrap().get_counter()[0]
    }
    fn data_size_in_bytes(&self) -> usize {
        misc::vector_len_in_bytes(&self.data)
    }
    fn data_height(&self) -> COUNTER {
        CAM_DESIGN.1
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
        self.is_ready && self.timer.elapsed().as_millis() > TIME_INTERVAL_FRAMES
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        as_bytes(&self.data)
    }
    fn new(_settings: &Settings) -> Self {
        Self{ data: tp3_vec!(1), is_ready: false, timer: Instant::now(), shutter: Some(ShutterControl::default())}
    }

    #[inline]
    fn add_electron_hit(&mut self, pack: Packet, settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        //If you are in cumulation mode, save all the electrons. If not, only save those that the
        //frame has not yet been sent
        if !self.is_ready || settings.cumul{
            let index = pack.x();
            self.data[index as usize] += pack.tot() as u32;
        }
    }
    fn add_tdc_hit2(&mut self, pack: Packet, _settings: &Settings, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(&pack);
        add_index!(self, CAM_DESIGN.0-1);
    }
    fn add_tdc_hit1(&mut self, pack: Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(&pack);
        add_index!(self, CAM_DESIGN.0-2);
    }
    fn add_shutter_hit(&mut self, pack: Packet, _frame_tdc: &mut TdcRef, settings: &Settings) {
        let temp_ready = self.shutter.as_mut().unwrap().try_set_time(pack.frame_time(), pack.ci(), pack.tdc_type() == 10);
        //If is_ready is false, set with temp_ready. If is_ready is true and another temp_ready
        //arrives, then we reset the array and do not send the frame. In this mode,
        //reset_or_else does not set is_ready to false.
        if !self.is_ready {
            self.is_ready = temp_ready;
        } else if temp_ready {
            if !settings.cumul {
                self.data.iter_mut().for_each(|x| *x = 0);
            }
            self.is_ready = false;
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
    fn get_frame_counter(&self, _tdc_value: &TdcRef) -> COUNTER {
        self.shutter.as_ref().unwrap().get_counter()[0]
    }
    fn data_size_in_bytes(&self) -> usize {
        misc::vector_len_in_bytes(&self.data)
    }
    fn data_height(&self) -> COUNTER {
        1
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
        as_bytes(&self.data[range])
    }
    fn new(settings: &Settings) -> Self {
        let len = (CAM_DESIGN.0 * settings.xscan_size * settings.yscan_size) as usize;
        let mut shutter = ShutterControl::default();
        shutter.set_as_hyperspectral(false);
        Self{ data: vec![0; len], is_ready: false, timer: Instant::now(), shutter: Some(shutter)}
    }

    #[inline]
    fn add_electron_hit(&mut self, pack: Packet, _settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        let shut = self.shutter.as_ref().unwrap();
        if shut.is_hyperspectral_complete() { return }
        let pixel_number = shut.get_counter()[pack.ci() as usize] as POSITION;
        //We cannot depass frame_number otherwise the indexation will be bad
        let index = pixel_number * CAM_DESIGN.0 + pack.x();
        self.data[index as usize] += pack.tot() as u32;
    }
    fn add_tdc_hit2(&mut self, _pack: Packet, _settings: &Settings, _ref_tdc: &mut TdcRef) {}
    fn add_tdc_hit1(&mut self, pack: Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(&pack);
    } 
    fn add_shutter_hit(&mut self, pack: Packet, _frame_tdc: &mut TdcRef, settings: &Settings) {
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
    fn get_frame_counter(&self, _tdc_value: &TdcRef) -> COUNTER {
        self.shutter.as_ref().unwrap().get_start_pixel()
    }
    fn data_size_in_bytes(&self) -> usize {
        self.shutter.as_ref().unwrap().get_data_size_to_send() * std::mem::size_of_val(&self.data[0])
    }
    fn data_height(&self) -> COUNTER {
        1
    }
}

pub struct Live2DFrameHyperspec {
    data: Vec<u32>,
    is_ready: bool,
    timer: Instant,
    shutter: Option<ShutterControl>,
}

impl SpecKind for Live2DFrameHyperspec {
    fn is_ready(&self) -> bool {
        self.is_ready
    }
    fn build_output(&mut self, _settings: &Settings) -> &[u8] {
        //The number of pixels sent is updated on the reset or else function
        let range = self.shutter.as_ref().expect("This mode must have the Shutter Control").get_index_range_to_send();
        as_bytes(&self.data[range])
    }
    fn new(settings: &Settings) -> Self {
        let len = (CAM_DESIGN.0 * CAM_DESIGN.1 * settings.xscan_size * settings.yscan_size) as usize;
        let mut shutter = ShutterControl::default();
        shutter.set_as_hyperspectral(true);
        Self{ data: vec![0; len], is_ready: false, timer: Instant::now(), shutter: Some(shutter)}
    }

    #[inline]
    fn add_electron_hit(&mut self, pack: Packet, _settings: &Settings, _frame_tdc: &TdcRef, _ref_tdc: &TdcRef) {
        let shut = self.shutter.as_ref().unwrap();
        if shut.is_hyperspectral_complete() { return }
        let pixel_number = shut.get_counter()[pack.ci() as usize] as POSITION;
        //We cannot depass frame_number otherwise the indexation will be bad
        let index = pixel_number * CAM_DESIGN.0 * CAM_DESIGN.1 + (pack.y() * CAM_DESIGN.0 + pack.x());
        self.data[index as usize] += pack.tot() as u32;
    }
    fn add_tdc_hit2(&mut self, _pack: Packet, _settings: &Settings, _ref_tdc: &mut TdcRef) {}
    fn add_tdc_hit1(&mut self, pack: Packet, frame_tdc: &mut TdcRef, _settings: &Settings) {
        frame_tdc.upt(&pack);
    } 
    fn add_shutter_hit(&mut self, pack: Packet, _frame_tdc: &mut TdcRef, settings: &Settings) {
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
    fn get_frame_counter(&self, _tdc_value: &TdcRef) -> COUNTER {
        self.shutter.as_ref().unwrap().get_start_pixel()
    }
    fn data_size_in_bytes(&self) -> usize {
        self.shutter.as_ref().unwrap().get_data_size_to_send() * std::mem::size_of_val(&self.data[0])
    }
    fn data_height(&self) -> COUNTER {
        CAM_DESIGN.1
    }
}




impl IsiBoxKind for Live1D {
    fn isi_new(_settings: &Settings) -> Self {
        let len = (CAM_DESIGN.0 + CHANNELS as POSITION) as usize;
        Self{ data: vec![0; len], is_ready: false, frame_counter: 0, last_time: 0, timer: Instant::now()}
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
            let msg = create_header(&meas_type, &my_settings, &frame_tdc, 0, meas_type.shutter_control());
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
            let msg = create_header(&meas_type, &my_settings, &frame_tdc, CHANNELS as POSITION, meas_type.shutter_control());
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
                let packet = Packet::new(*last_ci, packet_change(x)[0]);
                match packet.id() {
                    11 | 10 => { //Event or frame based
                        final_data.add_electron_hit(packet, settings, frame_tdc, ref_tdc);
                    },
                    6 if packet.tdc_type() == frame_tdc.id() => { //Tdc value 1
                        final_data.add_tdc_hit1(packet, frame_tdc, settings);
                    },
                    6 if packet.tdc_type() == ref_tdc.id() => { //Tdc value 2
                        final_data.add_tdc_hit2(packet, settings, ref_tdc);
                    },
                    5 if packet.tdc_type() == 10 || packet.tdc_type() == 15  => { //Shutter value.
                        final_data.add_shutter_hit(packet, frame_tdc, settings);
                    },
                    _ => {
                        if packet.id() == 6 {
                            //println!("{} and {}", packet.tdc_type(), packet.tdc_time_abs());
                        }
                    },
                };
            },
        };
    };
    final_data.is_ready()
}

fn create_header<W: SpecKind>(measurement: &W, set: &Settings, tdc: &TdcRef, extra_pixels: POSITION, shutter_control: Option<&ShutterControl>) -> Vec<u8> {
    let mut msg: String = String::from("{\"timeAtFrame\":");
    msg.push_str(&(tdc.time().to_string()));
    msg.push_str(",\"frameNumber\":");
    msg.push_str(&((measurement.get_frame_counter(tdc)).to_string()));
    msg.push_str(",\"measurementID:\"Null\",\"dataSize\":");
    msg.push_str(&((measurement.data_size_in_bytes().to_string())));
    /*
    if set.mode == 6 || set.mode == 8 { //ChronoMode
        msg.push_str(&((set.xspim_size*set.bytedepth*(CAM_DESIGN.0+extra_pixels)).to_string()));
    } else if set.mode == 7 { //Coincidence2D
        msg.push_str(&((set.time_width as POSITION*4*set.bytedepth*(CAM_DESIGN.0+extra_pixels)).to_string()));
    } else if set.mode == 11 { //Frame-based hyperspectral image
        let data_size = shutter_control.unwrap().get_pixel_to_send_size();
        msg.push_str(&((data_size*set.bytedepth*(CAM_DESIGN.0+extra_pixels)).to_string()));
    } else if set.mode == 15 { //Frame-based 2D hyperspectral image
        let data_size = shutter_control.unwrap().get_pixel_to_send_size();
        msg.push_str(&((data_size*set.bytedepth*(CAM_DESIGN.0+extra_pixels)*CAM_DESIGN.1).to_string()));
    } else {
        match set.bin {
            true => { msg.push_str(&((set.bytedepth*(CAM_DESIGN.0+extra_pixels)).to_string()))},
            false => { msg.push_str(&((set.bytedepth*(CAM_DESIGN.0+extra_pixels)*CAM_DESIGN.1).to_string()))},
        }
    }
    */
    msg.push_str(",\"bitDepth\":");
    msg.push_str(&((set.bytedepth<<3).to_string()));
    msg.push_str(",\"width\":");
    msg.push_str(&((CAM_DESIGN.0+extra_pixels).to_string()));
    msg.push_str(",\"height\":");
    msg.push_str(&(measurement.data_height().to_string()));
    /*
    if set.mode == 6 || set.mode == 8 { //ChronoMode
        msg.push_str(&(set.xspim_size.to_string()));
    } else if set.mode == 7 { //Coincidence2D Mode
        msg.push_str(&((set.time_width*4).to_string()));
    } else {
        match set.bin {
            true=>{msg.push_str(&(1.to_string()))},
            false=>{msg.push_str(&(CAM_DESIGN.1.to_string()))},
        }
    }
    */
    msg.push_str("}\n");
    println!("{:?}", msg);

    let s: Vec<u8> = msg.into_bytes();
    s
}

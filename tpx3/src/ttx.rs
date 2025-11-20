use std::slice;
use crate::auxiliar::{Settings, value_types::*};
use crate::tdclib;
use crate::speclib::SpecKind;
use std::time::Instant;

// Opaque type for FFI
#[repr(C)]
pub struct MyTimeTagger;

// FFI bindings
#[link(name = "TTX")] // TTX.dll
#[allow(improper_ctypes)]
extern "C" {
    fn mytt_create() -> *mut MyTimeTagger;
    fn mytt_destroy(t: *mut MyTimeTagger);
    fn mytt_add_channel(t: *mut MyTimeTagger, channel: i32, is_test_signal: bool);
    fn mytt_start_stream(t: *mut MyTimeTagger);
    fn mytt_get_data(t: *mut MyTimeTagger);
    fn mytt_get_timestamps(t: *mut MyTimeTagger, out_len: *mut usize) -> *const u64;
    fn mytt_get_channels(t: *mut MyTimeTagger, out_len: *mut usize) -> *const u32;
    fn mytt_set_stream_block_size(t: *mut MyTimeTagger, max_events: i32, max_latency: i32);
}

// Safe Rust wrapper
pub struct TimeTagger {
    inner: *mut MyTimeTagger,
    //timestamp_ptr: *const u64,
    //channel_ptr: *const u32,
}

impl TimeTagger {
    pub fn new() -> Self {
        let ptr = unsafe { mytt_create() };
        TimeTagger { inner: ptr }
    }

    pub fn add_channel(&self, channel: i32, is_test: bool) {
        unsafe { mytt_add_channel(self.inner, channel, is_test) }
    }

    pub fn start_stream(&self) {
        unsafe { mytt_start_stream(self.inner) }
    }

    pub fn get_data(&self) {
        unsafe { mytt_get_data(self.inner) }
    }

    pub fn get_timestamps(&self) -> Vec<u64> {
        let mut len = 0;
        let ptr = unsafe { mytt_get_timestamps(self.inner, &mut len) };
        unsafe { slice::from_raw_parts(ptr, len).to_vec() }
    }

    pub fn get_channels(&self) -> Vec<u32> {
        let mut len = 0;
        let ptr = unsafe { mytt_get_channels(self.inner, &mut len) };
        unsafe { slice::from_raw_parts(ptr, len).to_vec() }
    }
    pub fn set_stream_block_size(&self, max_events: i32, max_latency: i32) {
        unsafe { mytt_set_stream_block_size(self.inner, max_events, max_latency) }
    }
}

impl Drop for TimeTagger {
    fn drop(&mut self) {
        unsafe { mytt_destroy(self.inner) }
    }
}

//#[derive(Debug, Clone)]
pub struct TTXRef {
    ttx: TimeTagger,
    counter: [COUNTER; 16],
    begin_time: [TIME; 16],
    time: [TIME; 16],
    period: [Option<TIME>; 16],
    ticks_to_frame: Option<COUNTER>, //Here it means it is a scan reference.
    subsample: POSITION,
    //video_delay: TIME,
    begin_frame: TIME,
    new_frame: bool,
    //oscillator_size: Option<(POSITION, POSITION)>,
    scan_ref_time: Option<TIME>,
    scan_ref_counter: Option<COUNTER>,
    scan_ref_period: Option<TIME>,
}

impl TTXRef {
    //pub fn new(is_scanning: bool, my_settings: &Settings) -> Self {
    pub fn new(is_scanning: bool, my_settings: &Settings) -> Option<Self> {
        let ttx = TimeTagger::new();
        if ttx.inner.is_null() {
            return None;
        }
        ttx.set_stream_block_size(4096, 20);
        ttx.add_channel(1, true);
        Some(TTXRef {
            ttx,
            counter: [0; 16],
            begin_time: [0; 16],
            time: [0; 16],
            period: [None; 16],
            ticks_to_frame: if is_scanning { None } else { Some(my_settings.yspim_size)},
            subsample: 1,
            //video_delay: 0, //my_settings.video_time,
            begin_frame: 0,
            new_frame: false,
            //oscillator_size: None,
            scan_ref_time: None,
            scan_ref_counter: None,
            scan_ref_period: None,
        })
    }

    pub fn prepare_periodic(&mut self, periodic_channels: Vec<u32>) {
        self.ttx.start_stream();
        
        self.ttx.get_data();
        let mut timestamps = self.ttx.get_timestamps();
        while timestamps.len() == 0 { //Guarantee we have data, specially at the beginning
            self.ttx.get_data();
            timestamps = self.ttx.get_timestamps();
        }
        let channels = self.ttx.get_channels();

        let mut check_next_scan = [false; 16];
        timestamps.iter().zip(channels.iter())
            .for_each(|(&ts, &ch)| {
                if periodic_channels.contains(&ch) { //If its periodic we save the period
                    if check_next_scan[ch as usize - 1] {
                        check_next_scan[ch as usize - 1] = false;
                        self.period[ch as usize - 1] = Some(ts - self.begin_time[ch as usize - 1]);
                    }
                }
                if self.begin_time[ch as usize - 1] == 0 { //We save the initial time. If its periodic, we will get the period in the next interaction
                    self.begin_time[ch as usize - 1] = ts;
                    if periodic_channels.contains(&ch) {check_next_scan[ch as usize - 1] = true};
                }
                self.counter[ch as usize - 1] += 1;
                self.time[ch as usize - 1] = ts;
            });
        //println!("{:?} and {:?} and {:?}", self.counter, self.begin_time, self.period);
    }

    pub fn build_data<K: SpecKind>(&mut self, speckind: &mut K) {
        let start = Instant::now();
        self.ttx.get_data();
        let mut timestamps = self.ttx.get_timestamps();
        while timestamps.len() == 0 {
            self.ttx.get_data();
            timestamps = self.ttx.get_timestamps();
        }
        let channels = self.ttx.get_channels();
        //println!("elapsed time is {:?}. Length is {}", start.elapsed(), timestamps.len());
        
        timestamps.iter().zip(channels.iter())
            .for_each(|(&ts, &ch)| {
                self.time[ch as usize - 1] = ts;
                self.counter[ch as usize - 1] += 1;
                if let (Some(spimy), Some(_period)) = (self.ticks_to_frame, self.period[ch as usize -1]) {
                    if ch == 1 { //SCAN SIGNAL
                        if self.counter[0] % (self.subsample * spimy) == 0 {
                            self.begin_frame = ts;
                            self.new_frame = true;
                        }
                    }
                }
                speckind.ttx_index(self.into_tdc_time(ts), ch);
            });
        //println!("Counter is: {:?} and {:?}", self.time, timestamps.len());
    }

    pub fn inform_scan_tdc(&mut self, scan_tdc: &mut tdclib::TdcRef) {
        self.scan_ref_counter = Some(scan_tdc.counter());
        self.scan_ref_time = Some(scan_tdc.time());
        self.scan_ref_period = scan_tdc.period();
    }

    pub fn into_tdc_time(&self, ts: u64) -> u64 {
        let counter_time_difference = (self.counter[0] as i64 - self.scan_ref_counter.unwrap() as i64) * self.period[0].unwrap() as i64; //in ps
        let offset = self.scan_ref_time.unwrap() as i64 - (self.time[0] / 260) as i64; //in ps
        let correction = offset - counter_time_difference; //in ps

        let ts_into = (ts as i64 / 260) + correction / 260;
        ts_into as u64

    }
}

pub fn determine_spread_period(timestamp: &[u64]) {
    let result: Vec<u64> = timestamp.iter().zip(timestamp.iter().skip(1))
        .map(|(&x, &y)| y - x).collect();

    let events = result.len() as u64;
    let average = result.iter().sum::<u64>() / events;
    let std = result.iter().map(|&x| {
        let diff = x as i64 - average as i64;
        (diff * diff) as u64
    })
    .sum::<u64>() / events;

    //println!("Number of events: {}. Average value is: {}. The standard deviation is: {}.", events, average, std);
}

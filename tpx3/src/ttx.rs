use std::slice;
use crate::auxiliar::{Settings, value_types::*};
use crate::tdclib;
use crate::speclib::SpecKind;
use crate::spimlib::SpimKind;
use crate::constlib::*;
use std::time::{Duration, Instant};
use std::thread;

// Opaque type for FFI
#[repr(C)]
pub struct MyTimeTagger;

// FFI bindings
#[cfg(target_os = "windows")]
#[link(name = "TTX")] // TTX.dll
#[allow(improper_ctypes)]
extern "C" {
    fn mytt_create() -> *mut MyTimeTagger;
    fn mytt_destroy(t: *mut MyTimeTagger);
    fn mytt_reset(t: *mut MyTimeTagger);
    fn mytt_add_channel(t: *mut MyTimeTagger, channel: i32, is_test_signal: bool);
    fn mytt_start_stream(t: *mut MyTimeTagger);
    fn mytt_stop_stream(t: *mut MyTimeTagger);
    fn mytt_get_data(t: *mut MyTimeTagger);
    fn mytt_get_timestamps(t: *mut MyTimeTagger, out_len: *mut usize) -> *const u64;
    fn mytt_get_channels(t: *mut MyTimeTagger, out_len: *mut usize) -> *const i32;
    fn mytt_set_stream_block_size(t: *mut MyTimeTagger, max_events: i32, max_latency: i32);
}

// FFI bindings
#[cfg(target_os = "linux")]
#[link(name = "TTTX")] // TTX.so
#[allow(improper_ctypes)]
extern "C" {
    fn mytt_create() -> *mut MyTimeTagger;
    fn mytt_destroy(t: *mut MyTimeTagger);
    fn mytt_reset(t: *mut MyTimeTagger);
    fn mytt_add_channel(t: *mut MyTimeTagger, channel: i32, is_test_signal: bool);
    fn mytt_start_stream(t: *mut MyTimeTagger);
    fn mytt_stop_stream(t: *mut MyTimeTagger);
    fn mytt_get_data(t: *mut MyTimeTagger);
    fn mytt_get_timestamps(t: *mut MyTimeTagger, out_len: *mut usize) -> *const u64;
    fn mytt_get_channels(t: *mut MyTimeTagger, out_len: *mut usize) -> *const i32;
    fn mytt_set_stream_block_size(t: *mut MyTimeTagger, max_events: i32, max_latency: i32);
}

// Safe Rust wrapper
pub struct TimeTagger {
    inner: *mut MyTimeTagger,
    //timestamp_ptr: *const u64,
    //channel_ptr: *const u32,
}

impl Clone for TimeTagger {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner,
        }
    }
}

unsafe impl Send for TimeTagger {}
//unsafe impl Sync for TimeTagger {}

impl TimeTagger {
    pub fn new() -> Self {
        let ptr = unsafe { mytt_create() };
        TimeTagger { inner: ptr }
    }

    pub fn reset(&self) {
        unsafe { mytt_reset(self.inner) }
    }

    pub fn add_channel(&self, channel: i32, is_test: bool) {
        unsafe { mytt_add_channel(self.inner, channel, is_test) }
    }

    pub fn start_stream(&self) {
        unsafe { mytt_start_stream(self.inner) }
    }
 
    pub fn stop_stream(&self) {
        unsafe { mytt_stop_stream(self.inner) }
    }

    pub fn get_data(&self) {
        unsafe { mytt_get_data(self.inner) }
    }

    pub fn get_timestamps(&self) -> Vec<u64> {
        let mut len = 0;
        let ptr = unsafe { mytt_get_timestamps(self.inner, &mut len) };
        unsafe { slice::from_raw_parts(ptr, len).to_vec() }
    }

    pub fn get_channels(&self) -> Vec<i32> {
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
        //unsafe { mytt_reset(self.inner) }
        //unsafe { mytt_destroy(self.inner) }
        unsafe { mytt_stop_stream(self.inner) }
    }
}

//#[derive(Debug, Clone)]
pub struct TTXRef {
    ttx: TimeTagger,
    counter: [COUNTER; 32],
    begin_time: [TIME; 32],
    time: [TIME; 32],
    periodic_channels: Vec<i32>,
    active_channels: Vec<i32>,
    period: [Option<TIME>; 16],
    high_time: [Option<TIME>; 16],
    low_time: [Option<TIME>; 16],
    ticks_to_frame: Option<COUNTER>, //Here it means it is a scan reference.
    subsample: POSITION,
    video_delay: TIME,
    begin_frame: TIME,
    new_frame: bool,
    //oscillator_size: Option<(POSITION, POSITION)>,
    scan_ref_time: Option<TIME>,
    scan_ref_counter: Option<COUNTER>,
    scan_ref_period: Option<TIME>,
    ttx_into_tpx3_correction: Option<i64>,
    is_running: bool,
}

impl Drop for TTXRef {
    fn drop(&mut self) {
    //    self.ttx.stop_stream()
    }
}

impl TTXRef {

    pub fn new_ttx() -> TimeTagger {
        return TimeTagger::new()
    }

    pub fn new_from_ttx(ttx: TimeTagger) -> Option<Self> {
        if ttx.inner.is_null() {
            return None;
        }
        ttx.set_stream_block_size(2048, 10);
        Some(TTXRef {
            ttx,
            counter: [0; 32],
            begin_time: [0; 32],
            time: [0; 32],
            periodic_channels: Vec::new(),
            active_channels: Vec::new(),
            period: [None; 16],
            high_time: [None; 16],
            low_time: [None; 16],
            ticks_to_frame: None,
            subsample: 1,
            video_delay: 0, //my_settings.video_time,
            begin_frame: 0,
            new_frame: false,
            //oscillator_size: None,
            scan_ref_time: None,
            scan_ref_counter: None,
            scan_ref_period: None,
            ttx_into_tpx3_correction: None,
            is_running: false,
        })
    }

    pub fn reset(&self) {
        self.ttx.reset();
    }

    pub fn apply_settings(&mut self, is_scanning: bool, my_settings: &Settings) {
        if is_scanning {
            self.ticks_to_frame = Some(my_settings.yspim_size);
            self.video_delay = my_settings.video_time;
        }
    }

    pub fn stop_stream(&mut self) {
        self.ttx.stop_stream();
        self.is_running = false;
    }

    pub fn add_channel(&mut self, channel: i32, is_test: bool, both_edges: bool, is_periodic: bool) {
        self.ttx.add_channel(channel, is_test);
        if both_edges {
            self.ttx.add_channel(-channel, is_test);
            self.active_channels.push(-channel);
        }
        if is_periodic {self.periodic_channels.push(channel)};
        self.active_channels.push(channel);
    }

    pub fn prepare(&mut self) {
        self.ttx.start_stream();
        self.is_running = true;

        thread::sleep(Duration::from_millis(100));

        
        self.ttx.get_data();
        let mut timestamps = self.ttx.get_timestamps();
        while timestamps.len() == 0 { //Guarantee we have data, specially at the beginning
            self.ttx.get_data();
            timestamps = self.ttx.get_timestamps();
        }
        let channels = self.ttx.get_channels();

        fn get_period(ts: &[u64], ch: &[i32], desired_channel: i32) -> Option<u64> {
            let filtered: Vec<u64> = ts.iter().zip(ch.iter())
                .filter_map(|(&ts, &ch)| (ch == desired_channel).then_some(ts))
                .collect();

            let len = filtered.len() / 2; //only in pairs so this removes the odd value;
            if len < 1 {return None}
            let sum = filtered.chunks_exact(2)
                .map(|val| val[1] - val[0])
                .sum::<u64>();

            Some(sum / len as u64)
        }

        fn get_high_time(ts: &[u64], ch: &[i32], desired_channel: i32) -> Option<u64> {
            let filtered: Vec<u64> = ts.iter().zip(ch.iter())
                .filter_map(|(&ts, &ch)| (ch.abs() == desired_channel).then_some(ts))
                .collect();

            let len = filtered.len() / 2; //only in pairs so this removes the odd value;
            if len < 1 {return None}
            let sum = filtered.chunks_exact(2)
                .map(|val| val[1] - val[0])
                .sum::<u64>();

            Some(sum / len as u64)
        }
        fn get_begin_time(ts: &[u64], ch: &[i32], desired_channel: i32) -> Option<u64> {
            let filtered: Vec<u64> = ts.iter().zip(ch.iter())
                .filter_map(|(&ts, &ch)| (ch == desired_channel).then_some(ts))
                .collect();

            let len = filtered.len(); //only in pairs so this removes the odd value;
            if len == 0 {return None}

            Some(filtered[0])
        }
        fn get_counter(ts: &[u64], ch: &[i32], desired_channel: i32) -> Option<u32> {
            let filtered: Vec<u64> = ts.iter().zip(ch.iter())
                .filter_map(|(&ts, &ch)| (ch == desired_channel).then_some(ts))
                .collect();

            let len = filtered.len(); //only in pairs so this removes the odd value;
            if len == 0 {return None}

            Some(len as u32)
        }
        fn get_last_time(ts: &[u64], ch: &[i32], desired_channel: i32) -> Option<u64> {
            let filtered: Vec<u64> = ts.iter().zip(ch.iter())
                .filter_map(|(&ts, &ch)| (ch == desired_channel).then_some(ts))
                .collect();

            let len = filtered.len(); //only in pairs so this removes the odd value;
            if len == 0 {return None}

            Some(filtered[len - 1])
        }
        for channel in &self.periodic_channels {
            let period = get_period(&timestamps, &channels, *channel).unwrap();
            let high_time = get_high_time(&timestamps, &channels, *channel).unwrap();
            let low_time = period - high_time;
            self.period[(channel-1) as usize] = Some(period);
            self.high_time[(channel-1) as usize] = Some(high_time);
            self.low_time[(channel-1) as usize] = Some(low_time);
        }
        for channel in &self.active_channels {
            //self.begin_time[(channel + 15) as usize] = get_begin_time(&timestamps, &channels, *channel);
            //self.counter[(channel + 15) as usize] = get_counter(&timestamps, &channels, *channel);
            //self.time[(channel + 15) as usize] = get_last_time(&timestamps, &channels, *channel);
        }
        //println!("The period {:?}", get_period(&timestamps, &channels, 1));
        //println!("The high time {:?}", get_high_time(&timestamps, &channels, 1));
        //println!("The begin time {:?}", get_begin_time(&timestamps, &channels, 1));
        //println!("The counter {:?}", get_counter(&timestamps, &channels, 1));
        //println!("The last time {:?}", get_last_time(&timestamps, &channels, 1));
    }

    pub fn build_spec_data<K: SpecKind>(&mut self, speckind: &mut K) {
        self.ttx.get_data();
        let timestamps = self.ttx.get_timestamps();
        let channels = self.ttx.get_channels();
        
        println!("length is {}", timestamps.len());
        
        timestamps.iter().zip(channels.iter())
            .for_each(|(&ts, &ch)| {
                let chi = (ch + 15) as usize;
                self.time[chi] = ts;
                self.counter[chi] += 1;
                if let (Some(spimy), Some(_period)) = (self.ticks_to_frame, self.period[(ch.abs() - 1) as usize]) {
                    if ch == 1 { //SCAN SIGNAL
                        if self.counter[0] % (self.subsample * spimy) == 0 {
                            self.begin_frame = ts;
                            self.new_frame = true;
                        }
                    }
                }
                speckind.ttx_index(ts, ch, self.into_tdc_time(ts));
            });
    }

    pub fn build_spim_data<K: SpimKind>(&mut self, spimkind: &mut K) {
        //let start = Instant::now();
        self.ttx.get_data();
        let timestamps = self.ttx.get_timestamps();
        let channels = self.ttx.get_channels();
        
        timestamps.iter().zip(channels.iter())
            .for_each(|(&ts, &ch)| {
                let chi = (ch + 15) as usize;
                self.time[chi] = ts;
                self.counter[chi] += 1;
                if let (Some(spimy), Some(_period)) = (self.ticks_to_frame, self.period[(ch.abs() - 1) as usize]) {
                    if ch == 1 { //SCAN SIGNAL
                        if self.counter[0] % (self.subsample * spimy) == 0 {
                            self.begin_frame = ts;
                            self.new_frame = true;
                        }
                    }
                }
                spimkind.ttx_index(ts, ch, self.sync_anytime_frame_time(ts));
            });
        //println!("Counter is: {:?} and {:?}", self.time, timestamps.len());
        //println!("elapsed time is {:?}. Length is {}", start.elapsed(), timestamps.len());
    }

    pub fn inform_scan_tdc(&mut self, scan_tdc: &mut tdclib::TdcRef) {
        //println!("***TTX***: scan tdc period is {:?}", scan_tdc.period());
        if scan_tdc.period().is_none() || self.period[0].is_none() { return }

        self.scan_ref_counter = Some(scan_tdc.counter());
        self.scan_ref_time = Some(scan_tdc.time());
        self.scan_ref_period = scan_tdc.period();
        //println!("***TTX***: Synchronizing TTX with TPX3. The period on TTX is {:?}. The period on TPX3 is {:?}.", self.period[0], self.scan_ref_period);

        //The difference in counter time between TTX and TPX3, in ps
        let counter_time_difference = (self.counter[0] as i64 - self.scan_ref_counter.unwrap() as i64) * self.period[0].unwrap() as i64; //in ps
        
        //For the given counter above, the time difference, in ps
        let offset = self.scan_ref_time.unwrap() as i64 - (self.time[0] / 260) as i64; //in ps
        
        //The correction is the time difference discounted the counter time, in ps
        let correction = offset - counter_time_difference; //in ps

        self.ttx_into_tpx3_correction = Some(correction);
    }

    pub fn into_tdc_time(&self, ts: u64) -> Option<u64> {
        let ts_into = (ts as i64 / 260) + self.ttx_into_tpx3_correction? / 260;
        Some(ts_into as u64)
    } 

    /*
    pub fn get_positional_index(&self, dt: TIME, xspim: POSITION, yspim: POSITION, _list_scan: SlType) -> Option<POSITION> {
        let determ = |dt: TIME, dt_partial: TIME, period: TIME, xspim: POSITION, low_time: TIME, yspim: POSITION| {
            let mut r = (dt / period) as POSITION / self.subsample; //how many periods -> which line to put.
            let rin = ((xspim as TIME * dt_partial) / low_time) as POSITION; //Column correction. Maybe not even needed.
                
            if r > (yspim-1) {
                if r > 4096 {return None;}
                r %= yspim;
            }
                
            let index = r * xspim + rin;
            Some(index)
        };

        let val = dt % self.period[0]?;
        if val < self.low_time[0]? {
            determ(dt, val, self.period[0]?, xspim, self.low_time[0]?, yspim)
        } else {
            None
        }
    }
    */

    #[inline]
    pub fn sync_anytime_frame_time(&self, mut time: TIME) -> Option<TIME> {
        if time < self.begin_frame + VIDEO_TIME + self.video_delay {
            let factor = (self.begin_frame + VIDEO_TIME + self.video_delay - time) / (self.period[0]?*(self.subsample*self.ticks_to_frame?) as TIME) + 1;
            time += self.period[0]?*(self.subsample*self.ticks_to_frame?) as TIME * factor;
        }
        Some(time - self.begin_frame - VIDEO_TIME - self.video_delay)
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

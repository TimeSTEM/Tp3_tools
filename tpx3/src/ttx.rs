use std::slice;
use std::io::Write;
use crate::auxiliar::{Settings, value_types::*, misc};
use crate::tdclib;
use crate::speclib::SpecKind;
use crate::spimlib::SpimKind;
use crate::constlib::*;
use crate::auxiliar::FileManager;
//use crate::errorlib::Tp3ErrorKind;

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
#[link(name = "TTX")] // TTX.so
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
    fn new() -> Option<Self> {
        let ptr = unsafe { mytt_create() };
        if ptr.is_null() { 
            println!("***TTX Lib***: Could not find the TTX device.");
            return None 
        }
        println!("***TTX Lib***: TTX device properly loaded.");
        Some(TimeTagger { inner: ptr })
    }

    fn reset(&self) {
        unsafe { mytt_reset(self.inner) }
    }

    fn add_channel(&self, channel: i32, is_test: bool) {
        unsafe { mytt_add_channel(self.inner, channel, is_test) }
    }

    fn start_stream(&self) {
        unsafe { mytt_start_stream(self.inner) }
    }
 
    fn stop_stream(&self) {
        unsafe { mytt_stop_stream(self.inner) }
    }

    fn get_data(&self) {
        unsafe { mytt_get_data(self.inner) }
    }

    fn get_timestamps(&self) -> &[u64] {
        let mut len = 0;
        let ptr = unsafe { mytt_get_timestamps(self.inner, &mut len) };
        unsafe { slice::from_raw_parts(ptr, len) }
    }

    fn get_channels(&self) -> &[i32] {
        let mut len = 0;
        let ptr = unsafe { mytt_get_channels(self.inner, &mut len) };
        unsafe { slice::from_raw_parts(ptr, len) }
    }
    fn set_stream_block_size(&self, max_events: i32, max_latency: i32) {
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
    ttx_into_tpx3_correction: Option<f64>,
    is_running: bool,
    ts_file: FileManager,
    ch_file: FileManager,
    histogram: Histogram,
}

impl Drop for TTXRef {
    fn drop(&mut self) {
    //    self.ttx.stop_stream()
    }
}

impl TTXRef {
    pub fn new_ttx() -> Option<TimeTagger> {
        return TimeTagger::new()
    }

    pub fn new_from_ttx(ttx: Option<TimeTagger>) -> Option<Self> {
        let ttx = ttx?;
        ttx.set_stream_block_size(256, 1);
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
            scan_ref_time: None,
            scan_ref_counter: None,
            scan_ref_period: None,
            ttx_into_tpx3_correction: None,
            is_running: false,
            ts_file: FileManager::new_empty(),
            ch_file: FileManager::new_empty(),
            histogram: Histogram::new(5, 100),
        })
    }

    pub fn reset(&self) {
        self.ttx.reset();
    }

    pub fn apply_settings(&mut self, is_scanning: bool, my_settings: &Settings) {
        if is_scanning {
            self.ticks_to_frame = Some(my_settings.yspim_size);
            self.video_delay = my_settings.video_time;
            self.ts_file = my_settings.create_ttx_file("_ts").expect("***TTX Lib***: Could not create TTX file.");
            self.ch_file = my_settings.create_ttx_file("_ch").expect("***TTX Lib***: Could not create TTX file.");
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

    fn poll_data(&mut self) { //-> Result<(), Tp3ErrorKind> {
        self.ttx.get_data();
        self.ts_file.write_all(misc::as_bytes(self.ttx.get_timestamps())).expect("***TTX Lib***: Could not save timestamp data.");
        self.ch_file.write_all(misc::as_bytes(self.ttx.get_channels())).expect("***TTX Lib***: Could not save channel data.");
        //Ok(())
    }

    pub fn prepare(&mut self) {
        //Closure to determine if we have gather enough information about channels. If bigger than
        //minimum the return is false (to escape from while)
        let periodic_channels = self.periodic_channels.clone();
        let get_counter_for_periodic_channels = {|ts: &[u64], ch: &[i32]| {
            for channel in periodic_channels.iter() {
                if get_counter(ts, ch, *channel) > MINIMUM_TTX_CHANNEL_COUNT { return false }
            }
            true
        }};

        //Start the stream
        self.ttx.start_stream();
        self.is_running = true;

        //Creating the timstamps and channel structs
        let mut timestamps = Vec::new();
        let mut channels = Vec::new();
        
        self.poll_data();
        timestamps.append(&mut self.ttx.get_timestamps().to_vec());
        channels.append(&mut self.ttx.get_channels().to_vec());

        while get_counter_for_periodic_channels(&timestamps, &channels) { //Guarantee we have data, specially at the beginning
            self.poll_data();
            timestamps.append(&mut self.ttx.get_timestamps().to_vec());
            channels.append(&mut self.ttx.get_channels().to_vec());
        }

        //Auxiliary functions
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
        fn get_counter(ts: &[u64], ch: &[i32], desired_channel: i32) -> u32 {
            ts.iter().zip(ch.iter())
                .filter(|(&_ts, &ch)| (ch == desired_channel))
                .count() as u32
        }
        fn get_last_time(ts: &[u64], ch: &[i32], desired_channel: i32) -> Option<u64> {
            let filtered: Vec<u64> = ts.iter().zip(ch.iter())
                .filter_map(|(&ts, &ch)| (ch == desired_channel).then_some(ts))
                .collect();

            let len = filtered.len(); //only in pairs so this removes the odd value;
            if len == 0 {return None}

            Some(filtered[len - 1])
        }

        //Determining values
        for channel in &self.periodic_channels {
            let period = get_period(&timestamps, &channels, *channel).unwrap();
            let high_time = get_high_time(&timestamps, &channels, *channel).unwrap();
            let low_time = period - high_time;
            self.period[(channel-1) as usize] = Some(period);
            self.high_time[(channel-1) as usize] = Some(high_time);
            self.low_time[(channel-1) as usize] = Some(low_time);
        }
        for channel in &self.active_channels {
            self.begin_time[(channel + 15) as usize] = get_begin_time(&timestamps, &channels, *channel).unwrap_or(0);
            self.counter[(channel + 15) as usize] = get_counter(&timestamps, &channels, *channel);
            self.time[(channel + 15) as usize] = get_last_time(&timestamps, &channels, *channel).unwrap_or(0);
        }
        //println!("***TTX Lib***: Creating a new TTX reference: {:?}.", self);
        //println!("The period {:?}", get_period(&timestamps, &channels, 1));
        //println!("The high time {:?}", get_high_time(&timestamps, &channels, 1));
        //println!("The begin time {:?}", get_begin_time(&timestamps, &channels, 1));
        //println!("The counter {:?}", get_counter(&timestamps, &channels, 1));
        //println!("The last time {:?}", get_last_time(&timestamps, &channels, 1));
    }

    pub fn build_spec_data<K: SpecKind>(&mut self, speckind: &mut K) {
        self.poll_data();
        let timestamps = self.ttx.get_timestamps();
        let channels = self.ttx.get_channels();

        //self.histogram.add_events(timestamps, channels);
        //self.histogram.try_determine_channel_jitter(self.period[0].unwrap() as i64, 1);
        //self.histogram.try_determine_cross_correlation(4, 5);
        
        // This determines the values inside the loop //
        let mut first_ttx = 0;
        let mut last_ttx = 0;
        let mut set_ttx_times = {|time: TIME| {
            if first_ttx == 0 {first_ttx = time;}
            last_ttx = time;
        }};
        
        for (&ts, &ch) in timestamps.iter().zip(channels.iter()) {
            let chi = (ch + 15) as usize;
            self.time[chi] = ts;
            self.counter[chi] += 1;
            //if ch == 1 && self.counter[16] % 1000 == 0 { //SCAN SIGNAL RISING EDGE
            //    println!("***TTX***: counter, time, time_into_tpx and correction {}, {} and {:?} and {:?}", self.counter[16], self.time[16], self.into_tdc_time(self.time[16]), self.ttx_into_tpx3_correction)
            //}
            if let (Some(spimy), Some(_period)) = (self.ticks_to_frame, self.period[(ch.abs() - 1) as usize]) {
                if ch == 1 { //SCAN SIGNAL RISING EDGE
                    if self.counter[0] % (self.subsample * spimy) == 0 {
                        self.begin_frame = ts;
                        self.new_frame = true;
                    }
                }
            }
            speckind.ttx_index(ts, ch, self.into_tpx3_tdc_time(ts));
            set_ttx_times(self.into_tpx3_tdc_time(ts).unwrap());
        };
        //println!("***TTX***: The values inside the loop is min/max/nevents: {} / {} / {}.", first_ttx, last_ttx, timestamps.len());
    }

    pub fn build_spim_data<K: SpimKind>(&mut self, spimkind: &mut K) {
        //let start = Instant::now();
        self.poll_data();
        let timestamps = self.ttx.get_timestamps();
        let channels = self.ttx.get_channels();
        
        for (&ts, &ch) in timestamps.iter().zip(channels.iter()) {
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
        };
        //println!("Counter is: {:?} and {:?}", self.time, timestamps.len());
        //println!("elapsed time is {:?}. Length is {}", start.elapsed(), timestamps.len());
    }

    pub fn inform_scan_tdc(&mut self, scan_tdc: &mut tdclib::TdcRef) {
        if scan_tdc.period().is_none() || self.period[0].is_none() { return }

        self.scan_ref_counter = Some(scan_tdc.counter());
        self.scan_ref_time = Some(scan_tdc.time());
        self.scan_ref_period = scan_tdc.period();

        //The difference in counter time between TTX and TPX3, in ps
        let counter_time_difference = (self.scan_ref_counter.unwrap() as i64 / 2 - self.counter[16] as i64) * self.period[0].unwrap() as i64; //in ps
        
        //For the given counter above, the time difference, in ps
        let offset = self.scan_ref_time.unwrap() as f64 * (1562.5 / 6.0) - self.time[16] as f64; //in ps
        
        //The correction is the time difference discounted the counter time, in ps
        let correction: f64 = offset - counter_time_difference as f64; //in ps

        self.ttx_into_tpx3_correction = Some(correction);
        //println!("***TTX***: Synchronizing TTX with TPX3. The time/counter/period on TTX is {:?}, {:?} and {:?}. The value in TPX3 domain is: {:?}.", self.time[16], self.counter[16], self.period[0], self.into_tdc_time(self.time[16]));
        //println!("***TTX***: Synchronizing TTX with TPX3. The time/counter/period on TPx3 is {:?}, {:?} and {:?}.", self.scan_ref_time.unwrap(), self.scan_ref_counter.unwrap(), self.scan_ref_period.unwrap());
    }

    pub fn into_tpx3_tdc_time(&self, ts: u64) -> Option<u64> {
        let ts_into = (ts as f64 / (1562.5 / 6.0)) + self.ttx_into_tpx3_correction? / (1562.5 / 6.0);
        Some(ts_into as u64)
    }

    fn into_tpx3_coarse_ticks(&self, ts: u64) -> Option<u64> {
        let ts_into = (ts as f64 / 3125.0) + self.ttx_into_tpx3_correction? / 3125.0;
        Some(ts_into as u64)
    }   

    fn into_tpx3_fine_ticks(&self, ts: u64) -> Option<u64> {
        //TODO: This will be zero all the time. Need to check in the scope.
        let total_ticks = (ts as f64 / (1562.5 / 6.0)) + self.ttx_into_tpx3_correction? / (1562.5 / 6.0);
        let coarse_ticks = (ts as f64 / 3125.0) + self.ttx_into_tpx3_correction? / 3125.0;
        Some((total_ticks - 12.0 * coarse_ticks) as u64)
    }  

    #[inline]
    pub fn sync_anytime_frame_time(&self, mut time: TIME) -> Option<TIME> {
        if time < self.begin_frame + VIDEO_TIME + self.video_delay {
            let factor = (self.begin_frame + VIDEO_TIME + self.video_delay - time) / (self.period[0]?*(self.subsample*self.ticks_to_frame?) as TIME) + 1;
            time += self.period[0]?*(self.subsample*self.ticks_to_frame?) as TIME * factor;
        }
        Some(time - self.begin_frame - VIDEO_TIME - self.video_delay)
    }

    pub fn into_tdc_packet(&self, file_manager: &mut FileManager) {
        let ts_slice = self.ttx.get_timestamps();
        let ch_slice = self.ttx.get_channels();

        let mut initial_counter = self.counter;
        let result: Vec<u64> = ts_slice.iter().zip(ch_slice.iter())
            .filter_map(|(ts, ch)| {
                if *ch == 1 { //Rising EDGE
                    initial_counter[16] += 1;
                    let packet_value: u64 = (6 << 60) | (15 << 56) | ((initial_counter[16] as u64) << 44) | (self.into_tpx3_coarse_ticks(*ts)? << 9) | (self.into_tpx3_fine_ticks(*ts)? << 5) ;
                    Some(packet_value)
                } else if *ch == -1 {
                    initial_counter[14] += 1;
                    let packet_value: u64 = (6 << 60) | (10 << 56) | ((initial_counter[14] as u64) << 44) | (self.into_tpx3_coarse_ticks(*ts)? << 9) | (self.into_tpx3_fine_ticks(*ts)? << 5) ;
                    Some(packet_value)
                } else {
                    None
                }
            })
        .collect::<Vec<u64>>();

        file_manager.write_all(misc::as_bytes(&result)).expect("***TTX Lib***: Could not save timestamp into Timepix3 raw data.");
    }


}

#[derive(Debug, Clone)]
pub struct Histogram {
    bin_width: i64,
    max_lag: i64,
    hist: Vec<i64>,
    timestamps: Vec<u64>,
    channels: Vec<i32>,
}

impl Histogram {
    pub fn new(bin_width: i64, max_lag: i64) -> Self {
        let nbins = (2 * max_lag / bin_width + 1) as usize;
        Histogram {
            bin_width,
            max_lag,
            hist: vec![0i64; nbins],
            timestamps: Vec::new(),
            channels: Vec::new(),
        }
    }
    pub fn set_hist_size(&mut self) {
        let nbins = (2 * self.max_lag / self.bin_width + 1) as usize;
        self.hist = vec![0i64; nbins];
    }
    pub fn set_bin_width(&mut self, value: i64) {
        self.bin_width = value;
        self.set_hist_size();
    }
    pub fn set_max_lag(&mut self, value: i64) {
        self.max_lag = value;
        self.set_hist_size();
    }
    pub fn try_determine_channel_jitter(&mut self, period: i64, ch1: i32) {
        if self.timestamps.len() > 1000 {
            self.determine_channel_jitter(period, ch1);
            self.print_histogram();
        }
    }
    pub fn try_determine_cross_correlation(&mut self, ch1: i32, ch2: i32) {
        if self.timestamps.len() > 1000 {
            self.determine_cross_correlation(ch1, ch2);
            self.print_histogram();
        }
    }
    pub fn add_event(&mut self, timestamp: u64, channel: i32) {
        self.timestamps.push(timestamp);
        self.channels.push(channel);
    }
    pub fn add_events(&mut self, timestamp: &[u64], channel: &[i32]) {
        self.timestamps.append(&mut timestamp.to_vec());
        self.channels.append(&mut channel.to_vec());
    }
    fn determine_channel_jitter(&mut self, period: i64, ch1: i32) {
        let channel_times: Vec<i64> = self.timestamps.iter().zip(self.channels.iter())
            .filter_map(|(&ts, &ch)| (ch == ch1).then_some(ts as i64)).collect();

        let diffs: Vec<i64> = channel_times.iter().zip(channel_times.iter().skip(1))
            .map(|(&x, &y)| y - x - period).collect();
            
        for dt in &diffs {
            if dt.abs() <= self.max_lag { 
                let idx = ((dt + self.max_lag) / self.bin_width) as usize;
                self.hist[idx] += 1;
            }
        }
        self.timestamps.clear();
        self.channels.clear();
    }
    fn determine_cross_correlation(&mut self, ch1: i32, ch2: i32) {
        let vec1: Vec<i64> = self.timestamps.iter().zip(self.channels.iter())
        .filter_map(|(&ts, &ch)| (ch == ch1).then_some(ts as i64)).collect();
    
        let vec2: Vec<i64> = self.timestamps.iter().zip(self.channels.iter())
        .filter_map(|(&ts, &ch)| (ch == ch2).then_some(ts as i64)).collect();

        let mut i_start = 0;
        let mut i_end = 0;

        for t1 in &vec1 {
            let start = t1 - self.max_lag;
            let end = t1 + self.max_lag;

            // This can be used for unsorted data. If sorted, see below
            //let s = vec2.partition_point(|&x| x < start);
            //let e = vec2.partition_point(|&x| x <= end);
    
            // Two-pointer algorithm
            // Sorted data, we advance the start index
            while i_start < vec2.len() && vec2[i_start] < start {
                i_start += 1;
            }
        
            // Sorted data, we advance the end index
            while i_end < vec2.len() && vec2[i_end] < end {
                i_end += 1;
            }
    
            for t2 in &vec2[i_start..i_end] {
                let tau = t2 - t1;
                let idx = ((tau + self.max_lag) / self.bin_width) as usize;
                self.hist[idx] += 1;
            }
        }
        self.timestamps.clear();
        self.channels.clear();
    }

    fn print_histogram(&self) {
        let total_counts: i64 = self.hist.iter().sum();
        if total_counts == 0 { return; }
        if total_counts % 2 != 0 { return; }

        let max_counts: i64 = *self.hist.iter().max().unwrap();
        let half_max = max_counts / 2;

        // Find FWHM interval
        let mut fwhm_start = None;
        let mut fwhm_end = None;

        for (i, &v) in self.hist.iter().enumerate() {
            if v >= half_max {
                if fwhm_start.is_none() {
                    fwhm_start = Some(i);
                }
                fwhm_end = Some(i);
            }
        }

        println!("number of counts: {}", total_counts);

        for (i, &v) in self.hist.iter().enumerate() {
            let scaled = (v * 500 / total_counts) as usize;

            // If inside FWHM → print using '#'
            let c = if let (Some(s), Some(e)) = (fwhm_start, fwhm_end) {
                if i >= s && i <= e { '#' } else { '*' }
            } else {
                '*'
            };

            println!("{:4} ps: {}", i as i64 * self.bin_width - self.max_lag, c.to_string().repeat(scaled));
        }
    }

}

//!`tdclib` is a collection of tools to facilitate manipulation and choice of tdcs. Module is built
//!in around `TdcType` enum.


mod prepare_tdc {
    use crate::errorlib::Tp3ErrorKind;
    use crate::tdclib::TdcType;
    use crate::packetlib::Packet;
    use crate::auxiliar::{misc::packet_change, value_types::*};

    ///This struct is used to search for the tdc in case of periodic signals.
    pub struct TdcSearch<'a> {
        data: Vec<(TIME, TdcType)>,
        how_many: usize,
        tdc_choosen: &'a TdcType,
        initial_counter: Option<COUNTER>,
        last_counter: u16,
    }

    impl<'a> TdcSearch<'a> {
        pub fn new(tdc_choosen: &'a TdcType, how_many: usize) -> Self {
            TdcSearch{
                data: Vec::new(),
                how_many,
                tdc_choosen,
                initial_counter: None,
                last_counter: 0,
            }
        }

        fn add_tdc(&mut self, packet: &Packet) {
            if let Some(tdc) = TdcType::associate_value_to_enum(packet.tdc_type()) {
                let time = packet.tdc_time_abs_norm();
                self.data.push( (time, tdc) );
                if packet.tdc_type() == self.tdc_choosen.associate_value() {
                    self.last_counter = packet.tdc_counter();
                    self.initial_counter = match self.initial_counter {
                        None => Some(packet.tdc_counter() as COUNTER),
                        Some(val) => Some(val),
                    };
                }
            }
        }

        pub fn check_tdc(&self) -> Result<bool, Tp3ErrorKind> {
            let mut counter = 0;
            for (_time, tdc_type) in &self.data {
                if tdc_type.associate_value() == self.tdc_choosen.associate_value() {counter+=1;}
            }
            if counter>=self.how_many {
                self.check_ascending_order()?;
                Ok(true)
            } else {Ok(false)}
        }

        fn get_timelist(&self, which: &TdcType) -> Vec<TIME> {
            let result: Vec<_> = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value() == which.associate_value())
                .map(|(time, _tdct)| *time)
                .collect();
            result
        }
        
        fn get_auto_timelist(&self) -> Vec<TIME> {
            let result: Vec<_> = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value() == self.tdc_choosen.associate_value())
                .map(|(time, _tdct)| *time)
                .collect();
            result
        }

        fn check_ascending_order(&self) -> Result<(), Tp3ErrorKind> {
            let time_list = self.get_auto_timelist();
            let result = time_list.iter().zip(time_list.iter().skip(1)).find(|(a, b)| a>b);
            if result.is_some() {Err(Tp3ErrorKind::TdcNotAscendingOrder)}
            else {Ok(())}
        }

        pub fn find_high_time(&self) -> Option<TIME> {
            let fal_tdc_type = match self.tdc_choosen {
                TdcType::TdcOneRisingEdge | TdcType::TdcOneFallingEdge => TdcType::TdcOneFallingEdge,
                TdcType::TdcTwoRisingEdge | TdcType::TdcTwoFallingEdge => TdcType::TdcTwoFallingEdge,
                TdcType::NoTdc => TdcType::NoTdc,
            };

            let ris_tdc_type = match self.tdc_choosen {
                TdcType::TdcOneRisingEdge | TdcType::TdcOneFallingEdge => TdcType::TdcOneRisingEdge,
                TdcType::TdcTwoRisingEdge | TdcType::TdcTwoFallingEdge => TdcType::TdcTwoRisingEdge,
                TdcType::NoTdc => TdcType::NoTdc,
            };

            let mut fal = self.get_timelist(&fal_tdc_type);
            let mut ris = self.get_timelist(&ris_tdc_type);
            //let last_fal = fal.pop().expect("Please get at least 01 falling Tdc");
            let last_fal = match fal.pop() {
                Some(val) => val,
                None => return None,
            };
            let last_ris = match ris.pop() {
                Some(val) => val,
                None => return None,
            };
            if last_fal > last_ris {
                Some(last_fal - last_ris)
            } else {
                let new_ris = match ris.pop () {
                    Some(val) => val,
                    None => return None,
                };
                Some(last_fal - new_ris)
            }
        }
        
        pub fn find_period(&self) -> Result<TIME, Tp3ErrorKind> {
            let mut tdc_time = self.get_auto_timelist();
            let last = tdc_time.pop().expect("Please get at least 02 Tdc's");
            let before_last = tdc_time.pop().expect("Please get at least 02 Tdc's");
            if last > before_last {
                Ok(last - before_last)
            } else {
                Err(Tp3ErrorKind::TdcBadPeriod)
            }
        }
        
        fn get_counter(&self) -> Result<COUNTER, Tp3ErrorKind> {
            let counter = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value()==self.tdc_choosen.associate_value())
                .count() as COUNTER;
            Ok(counter)
        }

        pub fn get_counter_offset(&self) -> COUNTER {
            self.initial_counter.expect("***Tdc Lib***: Tdc initial counter offset was not found.")
        }

        //fn get_last_hardware_counter(&self) -> u16 {
        //    self.last_counter
        //}

        pub fn get_lasttime(&self) -> TIME {
            let last_time = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value()==self.tdc_choosen.associate_value())
                .map(|(time, _tdct)| *time)
                .last().unwrap();
            last_time
        }

        pub fn get_begintime(&self) -> TIME {
            let begin_time = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value()==self.tdc_choosen.associate_value())
                .map(|(time, _tdct)| *time)
                .next().unwrap();
            begin_time
        }

        pub fn search_specific_tdc(&mut self, data: &[u8]) {
            data.chunks_exact(8).for_each(|x| {
                match *x {
                    [84, 80, 88, 51, _, _, _, _] => {},
                    _ => {
                        let packet = Packet::new(0, packet_change(x)[0]);
                        if packet.id() == 6 && self.tdc_choosen.is_same_inputline(packet.tdc_type()) {
                            self.add_tdc(&packet);
                        }
                    },
                };
            });
        }
    }

    ///This struct is using to estimate the size of the beam in the EELS camera. We need Ymax and
    ///Ymin in order to correct the time of arrival.
    pub struct OscillatorEstimate {
        data: Vec<POSITION>,
        how_many: usize,
    }
    impl OscillatorEstimate {
        pub fn new(how_many: usize) -> Self {
            OscillatorEstimate {
                data: Vec::new(),
                how_many,
            }
        }
        pub fn add_electron(&mut self, packet: Packet) {
            self.data.push(packet.y());
        }
        pub fn check(&self) -> bool {
            self.data.len() > self.how_many
        }
        pub fn search_for_electrons(&mut self, data: &[u8]) {
            data.chunks_exact(8).for_each(|x| {
                match *x {
                    [84, 80, 88, 51, _, _, _, _] => {},
                    _ => {
                        let packet = Packet::new(0, packet_change(x)[0]);
                        if packet.id() == 10 || packet.id() == 11 {
                            self.add_electron(packet);
                        }
                    }
                };
            })
        }
        pub fn extract_amplitude(&mut self, percentile_min: f64, percentile_max: f64) -> (POSITION, POSITION) {
            self.data.sort();
            let size = self.data.len();

            fn get_percentile(data: &[POSITION], length: usize, percentage: f64) -> POSITION {
                let to_advance = (length as f64 * percentage / 100.0) as usize;
                data.iter().nth(to_advance).copied().expect("***Tdc Lib***: Could not extract the value of the oscillator...")
            }

            let ymax = get_percentile(&self.data, size, percentile_max);
            let ymin = get_percentile(&self.data, size, percentile_min);
            (ymax, ymin)

        }
    }
}


///The four types of TDC's.
pub enum TdcType {
    TdcOneRisingEdge,
    TdcOneFallingEdge,
    TdcTwoRisingEdge,
    TdcTwoFallingEdge,
    NoTdc,
}

impl Clone for TdcType {
    fn clone(&self) -> TdcType {
        match self {
            TdcType::TdcOneRisingEdge => TdcType::TdcOneRisingEdge,
            TdcType::TdcOneFallingEdge => TdcType::TdcOneFallingEdge,
            TdcType::TdcTwoRisingEdge => TdcType::TdcTwoRisingEdge,
            TdcType::TdcTwoFallingEdge => TdcType::TdcTwoFallingEdge,
            TdcType::NoTdc => TdcType::NoTdc,
        }
        //match self {
    }
}

impl TdcType {
    ///Convenient method. Return value is the 4 bits associated to each TDC.
    pub fn associate_value(&self) -> u8 {
        match *self {
            TdcType::TdcOneRisingEdge => 15,
            TdcType::TdcOneFallingEdge => 10,
            TdcType::TdcTwoRisingEdge => 14,
            TdcType::TdcTwoFallingEdge => 11,
            TdcType::NoTdc => 0,
        }
    }

    fn associate_str(&self) -> String {
        match *self {
            TdcType::TdcOneRisingEdge => String::from("Tdc 01 Rising Edge"),
            TdcType::TdcOneFallingEdge => String::from("Tdc 01 Falling Edge"),
            TdcType::TdcTwoRisingEdge => String::from("Tdc 02 Rising Edge"),
            TdcType::TdcTwoFallingEdge => String::from("Tdc 02 Falling Edge"),
            TdcType::NoTdc => String::from("Tdc Disabled"),
        }
    }
    
    ///Check if a given tdc is from the same input line.
    fn is_same_inputline(&self, check: u8) -> bool {
        match *self {
            TdcType::TdcOneRisingEdge | TdcType::TdcOneFallingEdge if check == 15 || check == 10 => true,
            TdcType::TdcTwoRisingEdge | TdcType::TdcTwoFallingEdge if check == 14 || check == 11 => true,
            _ => false,
        }
    }

    ///From associate value to enum TdcType.
    pub fn associate_value_to_enum(value: u8) -> Option<TdcType> {
        match value {
            15 => Some(TdcType::TdcOneRisingEdge),
            10 => Some(TdcType::TdcOneFallingEdge),
            14 => Some(TdcType::TdcTwoRisingEdge),
            11 => Some(TdcType::TdcTwoFallingEdge),
            _ => None,
        }
    }
}

use std::time::{Duration, Instant};
use crate::errorlib::Tp3ErrorKind;
use crate::auxiliar::{Settings, misc::{check_if_in, TimepixRead}};
use crate::auxiliar::{value_types::*, FileManager};
use crate::constlib::*;
use crate::packetlib::Packet;
use std::io::Write;

#[derive(Debug, Clone)]
pub struct TdcRef {
    tdctype: u8,
    counter: COUNTER,
    counter_offset: COUNTER,
    last_hard_counter: u16,
    counter_overflow: COUNTER,
    begin_time: TIME,
    time: TIME,
    period: Option<TIME>,
    high_time: Option<TIME>,
    low_time: Option<TIME>,
    ticks_to_frame: Option<COUNTER>, //Here it means it is a scan reference.
    subsample: POSITION,
    video_delay: TIME,
    begin_frame: TIME,
    new_frame: bool,
    oscillator_size: Option<(POSITION, POSITION)>,
}

impl TdcRef {
    pub fn id(&self) -> u8 {
        self.tdctype
    }

    pub fn upt(&mut self, packet: &Packet) {
        let hard_counter = packet.tdc_counter();
        let time = packet.tdc_time_abs_norm();
        if hard_counter < self.last_hard_counter {
            self.counter_overflow += 1;
        }
        self.last_hard_counter = hard_counter;
        self.counter = self.last_hard_counter as COUNTER + self.counter_overflow * 4096 - self.counter_offset;
        let time_overflow = self.time > time;
        //println!("updating tdc {}. Counter is {}. Ticks to frame is {:?}. Line is {:?}", time - self.time, self.counter, self.ticks_to_frame, (self.counter / 2) % (self.subsample * self.ticks_to_frame.unwrap()));
        self.time = time;
        if let (Some(spimy), Some(period)) = (self.ticks_to_frame, self.period) {
            //New frame
            if (self.counter / 2) % (self.subsample * spimy) == 0 {
                //println!("new frame dT: {}", time - self.begin_frame);
                self.begin_frame = time;
                self.new_frame = true;
            //Not new frame but a time overflow
            } else if time_overflow {
                //I temporally correct the begin_frame time by supossing what is the next frame time. This
                //will be correctly updated in the next cycle.
                let frame_time = period * (self.subsample * spimy) as TIME;
                self.begin_frame = if frame_time > ELECTRON_OVERFLOW_IN_TDC_UNITS {
                    (self.begin_frame + frame_time) - ELECTRON_OVERFLOW_IN_TDC_UNITS
                } else {
                    (self.begin_frame + frame_time) % ELECTRON_OVERFLOW_IN_TDC_UNITS
                };
                self.new_frame = false 
            //Does nothing. No new frame and no time overflow
            } else {
                self.new_frame = false
            }
        }
    }

    pub fn counter(&self) -> COUNTER {
        self.counter
    }

    pub fn time(&self) -> TIME {
        self.time
    }

    pub fn period(&self) -> Option<TIME> {
        Some(self.period?)
    }

    pub fn new_frame(&self) -> bool {
        self.new_frame
    }
    
    pub fn frame(&self) -> Option<COUNTER> {
        Some(self.counter / 2 / self.ticks_to_frame?)
    }
    
    pub fn current_line(&self) -> Option<POSITION> {
        Some(((self.counter / 2) % self.ticks_to_frame?) as POSITION)
    }

    pub fn estimate_time(&self) -> Option<TIME> {
        Some((self.counter as TIME / 2) * self.period? + self.begin_time)
    }

    pub fn is_fast_oscillator(&self) -> bool {
        self.oscillator_size.is_some()
    }

    //pub fn electron_relative_time(&self, ele_time: TIME) -> TIME {
    //    ele_time - self.begin_frame - VIDEO_TIME
    //}
    
    //This recovers the position of the probe given the TDC and the electron ToA. dT is the time from
    //the last frame begin
    #[inline]
    pub fn get_positional_index(&self, dt: TIME, xspim: POSITION, yspim: POSITION, list_scan: SlType) -> Option<POSITION> {
        if let Some(custom_list) = list_scan {
         
            //exceding time is always with respect to the current pixel time, so this values varies
            //from 0 - pixel_time.
            fn exponential_interpolation(exceding_time: POSITION, pixel_time: u32, previous: POSITION, new: POSITION) -> POSITION {
                let ratio = -(1.0 * exceding_time as f32 / pixel_time as f32);
                if new > previous {
                    new - ((new - previous) as f32 * ratio.exp()) as POSITION
                } else {
                    new + ((previous - new) as f32 * ratio.exp()) as POSITION
                }
            }
            let get_xy_from_index = |value: POSITION| -> (POSITION, POSITION) {
                let x = (value & (1<<DACX_BITDEPTH)-1) * xspim / (1<<DACX_BITDEPTH);
                let y = (value >> DACX_BITDEPTH) * yspim / (1<<DACY_BITDEPTH);
                (x, y)
            };
            let mut index = (dt * (self.subsample * xspim) as TIME / self.period?) as POSITION;
            if index >= self.subsample * self.subsample * xspim * yspim {
                index %= self.subsample * self.subsample * xspim * yspim
            }
            let (x, y) = get_xy_from_index(custom_list[index as usize]);
            Some(y * xspim + x)
            //TODO: To be tested on the machine           
            /*
            let frac = (dt * (self.subsample * xspim) as TIME % self.period?) as POSITION / (self.subsample * xspim);
            let mut previous_index = index - 1;
            if previous_index >= self.subsample * self.subsample * xspim * yspim {
                previous_index %= self.subsample * self.subsample * xspim * yspim
            }
            let (xp, yp) = get_xy_from_index(custom_list[previous_index as usize]);
            
            let x_cor = exponential_interpolation(frac, self.period? as POSITION / (self.subsample * xspim), xp, x);
            let y_cor = exponential_interpolation(frac, self.period? as POSITION / (self.subsample * xspim), yp, y);
            Some(y_cor * xspim + x_cor)
            */
        } else {
            let determ = |dt: TIME, dt_partial: TIME, period: TIME, xspim: POSITION, low_time: TIME, yspim: POSITION| {
                let mut r = (dt / period) as POSITION / self.subsample; //how many periods -> which line to put.
                let rin = ((xspim as TIME * dt_partial) / low_time) as POSITION; //Column correction. Maybe not even needed.
                    
                if r > (yspim-1) {
                    if r > 4096 {return None;} //This removes overflow electrons. See add_electron_hit
                    r %= yspim;
                }
                    
                let index = r * xspim + rin;
                Some(index)
            };

            if REMOVE_RETURN {
                let val = dt % self.period?;
                if val < self.low_time? {
                    determ(dt, val, self.period?, xspim, self.low_time?, yspim)
                } else {
                    None
                }
            } else {
                let val = dt % self.period?;
                determ(dt, val, self.period?, xspim, self.low_time?, yspim)
            }
        }
    }
    
    //This recovers the position of the probe during the return given the TDC and the electron ToA
    #[inline]
    pub fn get_return_positional_index(&self, dt: TIME, xspim: POSITION, yspim: POSITION, _list_scan: SlType) -> Option<POSITION> {
        let val = dt % self.period?;
        if val >= self.low_time? {
            let mut r = (dt / self.period?) as POSITION; //how many periods -> which line to put.
            let rin = ((xspim as TIME * (val - self.low_time?)) / self.high_time?) as POSITION; //Column correction. Maybe not even needed.

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
    
    #[inline]
    pub fn sync_electron_frame_time(&self, pack: &Packet) -> Option<TIME> {
        let mut ele_time = pack.electron_time_in_tdc_units();
        if SYNC_MODE == 0 {
            if ele_time < self.begin_frame + VIDEO_TIME + self.video_delay{
                let factor = (self.begin_frame + VIDEO_TIME + self.video_delay - ele_time) / (self.period?*(self.subsample*self.ticks_to_frame?) as TIME) + 1;
                ele_time += self.period?*(self.subsample*self.ticks_to_frame?) as TIME * factor;
            }
            Some(ele_time - self.begin_frame - VIDEO_TIME - self.video_delay)
        } else {
            if ele_time < self.time + VIDEO_TIME + self.video_delay {
                let factor = (self.time + VIDEO_TIME + self.video_delay - ele_time) / (self.period?) + 1;
                ele_time += self.period? * factor * self.current_line()? as u64;
            }
            Some(ele_time - self.begin_frame - VIDEO_TIME - self.video_delay)
        }
    }

    #[inline]
    pub fn sync_tdc_frame_time(&self, pack: &Packet) -> Option<TIME> {
        let mut tdc_time = pack.tdc_time_abs_norm();
        if SYNC_MODE == 0 {
            if tdc_time < self.begin_frame + VIDEO_TIME + self.video_delay{
                let factor = (self.begin_frame + VIDEO_TIME + self.video_delay - tdc_time) / (self.period?*(self.subsample*self.ticks_to_frame?) as TIME) + 1;
                tdc_time += self.period?*(self.subsample*self.ticks_to_frame?) as TIME * factor;
            }
            Some(tdc_time - self.begin_frame - VIDEO_TIME - self.video_delay)
        } else {
            if tdc_time < self.time + VIDEO_TIME + self.video_delay {
                let factor = (self.time + VIDEO_TIME + self.video_delay - tdc_time) / (self.period?) + 1;
                tdc_time += self.period? * factor * self.current_line()? as u64;
            }
            Some(tdc_time - self.begin_frame - VIDEO_TIME - self.video_delay)
        }
    }

    //This gets the closest time for a period TDC. Here, the TDC time found is always greater than
    //the electron time. The tdc_offset argument is used with the fast oscillator.
    #[inline]
    fn get_closest_tdc(&self, time: TIME, tdc_offset: TIME) -> TIME {
        let period = self.period().expect("Period must exist in time-resolved mode.");
        let last_tdc_time = self.time() + tdc_offset;
     
        //This case TDC time is always greater than electron time
        let xper;
        let eff_tdc = if last_tdc_time > time {
            xper = ((last_tdc_time - time) * PERIOD_DIVIDER) / period;
            last_tdc_time - (xper * period) / PERIOD_DIVIDER
        } else {
            xper = ((time - last_tdc_time) * PERIOD_DIVIDER) / period + 1;
            last_tdc_time + (xper * period) / PERIOD_DIVIDER
        };
        eff_tdc
    } 

    //This checks if the electron is inside a given time_delay and time_width for a periodic tdc
    //and returns the closest TDC.
    pub fn tr_electron_check_if_in(&self, pack: &Packet, settings: &Settings) -> Option<TIME> {
        let ele_time = pack.electron_time_in_tdc_units();
        let eff_tdc = self.get_closest_tdc(ele_time, 0);
     
        //This case photon time is always greater than electron time
        if check_if_in(&ele_time, &eff_tdc, settings) {
            Some(eff_tdc)
        } else {
            None
        }
    }

    //This checks if the tdc is inside a given time_delay and time_width for a periodic tdc
    //and returns the closest TDC.
    pub fn tr_tdc_check_if_in(&self, pack: &Packet, settings: &Settings) -> Option<TIME> {
        let tdc_time = pack.tdc_time_abs_norm();
        let eff_tdc = self.get_closest_tdc(tdc_time, 0);
     
        //This case photon time is always greater than electron time
        if check_if_in(&tdc_time, &eff_tdc, settings) {
            Some(eff_tdc)
        } else {
            None
        }
    }

    //We only use the electron time to know the quadrant. We then afterwards use the Y to determine
    //the exact time of arrival.
    pub fn tr_electron_correct_by_blanking(&self, pack: &Packet) -> Option<TIME> {
        if let Some((ymax_osc, ymin_osc)) = self.oscillator_size {
            let ele_time = pack.electron_time_in_tdc_units();

            //Getting the offset (equivalent of phase shift the tdc)
            let offset = match pack.x() {
                0..=255 => 12,       // inclusive range 0 to 255
                256..=511 => 8,      // inclusive range 256 to 511
                512..=767 => 8,      // inclusive range 512 to 767
                768..=1023 => 8,     // example for the next range
                _ => return None,
            };

            let eff_tdc = self.get_closest_tdc(ele_time, offset);
            let delta = eff_tdc - ele_time;
            let quarter_period = ((delta * 4 * PERIOD_DIVIDER) / BLANKING_PERIOD) as usize;
            //let quarter_period_frac = ((delta * 4 * PERIOD_DIVIDER) as f64 / BLANKING_PERIOD as f64).fract();
            //if quarter_period_frac < 0.25 || quarter_period_frac > 0.75 {
            //    return None
            //}
            if quarter_period > 3 {
                return None;
            }

            const PI: f64 = std::f64::consts::PI;
            const INV_PI: f64 = 1.0 / PI;
            const QUADRANT_SIZE: f64 = (BLANKING_PERIOD as f64 / PERIOD_DIVIDER as f64) / 4.0;
            const SCALE: f64 = QUADRANT_SIZE * INV_PI;
            //const SCALE: f64 = 24.0 * INV_PI;
            const HALF_PI: f64 = PI / 2.0;

            let y = pack.y();
            if y < ymin_osc || y > ymax_osc {
                return None;
            }

            let y = y as f64;
            let ymin = ymin_osc as f64;
            let ymax = ymax_osc as f64;
    
            let y_normalized = 2.0 * (y - ymin) / (ymax - ymin) - 1.0;
            if y_normalized < -1.0 || y_normalized > 1.0 {
                return None;
            }
    
            let y_corr = match quarter_period {
                0 => ((y_normalized.asin() + HALF_PI) * SCALE) - 1.0 * QUADRANT_SIZE,
                1 => (y_normalized.acos() * SCALE) - 2.0 * QUADRANT_SIZE,
                2 => ((y_normalized.asin() + HALF_PI) * SCALE) - 3.0 * QUADRANT_SIZE,
                3 => (y_normalized.acos() * SCALE) - 4.0 * QUADRANT_SIZE,
                _ => return None,
            };
            
            //if pack.y() == ymax_osc && pack.tot() > 30 && quarter_period == 0 {
            //    println!("delta at maximum Y: {}. Tcor is {}. Original time is {}. New time is {}", delta, y_corr.abs().round() as TIME, ele_time, eff_tdc - y_corr.abs().round() as TIME);
            //}

            //Some(eff_tdc + y_corr.round() as TIME)
            Some(eff_tdc - y_corr.abs().round() as TIME)
        } else {
            None
        }
    }

    pub fn new_periodic<T: TimepixRead>(tdc_type: TdcType, sock: &mut T, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<Self, Tp3ErrorKind> {
        let mut buffer_pack_data = vec![0; 16384];
        let mut tdc_search = prepare_tdc::TdcSearch::new(&tdc_type, 3);
        let start = Instant::now();

        println!("***Tdc Lib***: Searching for Tdc: {}.", tdc_type.associate_str());
        loop {
            if start.elapsed() > Duration::from_secs(TDC_TIMEOUT) {return Err(Tp3ErrorKind::TdcNoReceived)}
            if let Ok(size) = sock.read_timepix(&mut buffer_pack_data) {
                file_to_write.write_all(&buffer_pack_data[0..size])?;
                tdc_search.search_specific_tdc(&buffer_pack_data[0..size]);
                if tdc_search.check_tdc()? {break;}
            }
        }
        println!("***Tdc Lib***: {} has been found.", tdc_type.associate_str());
        //TDC abs is used, so we should divide by a factor of 6 here to be back on electron clock
        //tick.
        
        //These are the modes that we use the spatia resolution:
        let mode = my_settings.mode;
        let ticks_to_frame = if mode == 2 || mode == 3 || mode == 12 || mode == 13 || mode == 14 {
            Some(my_settings.yspim_size)
        } else {
            None
        };

        let ratio = my_settings.xscan_size / my_settings.xspim_size;
        let counter_offset = tdc_search.get_counter_offset();
        let begin_time = tdc_search.get_begintime();
        let last_time = tdc_search.get_lasttime();
        let period = tdc_search.find_period()?;
        let high_time = tdc_search.find_high_time().map(|time| time);
        let low_time = tdc_search.find_high_time().map(|time| period - time);


        //If the TDC is periodic, we check if the fast oscillator is ON.
        let mut oscillator_size: Option<(POSITION, POSITION)> = None;
        if ((period as i64 - BLANKING_PERIOD as i64).abs() as TIME) < 100 {
            println!("***Tdc Lib***: The fast oscillator has been detected.");
            println!("***Tdc Lib***: Estimating the values of the beam...");
            let mut osc_estimate = prepare_tdc::OscillatorEstimate::new(NUMBER_OF_ELECTRONS_FOR_OSCILLATOR);
            let start = Instant::now();
            loop {
                if start.elapsed() > Duration::from_secs(TDC_TIMEOUT) {return Err(Tp3ErrorKind::TdcNoReceived)}
                if let Ok(size) = sock.read_timepix(&mut buffer_pack_data) {
                    file_to_write.write_all(&buffer_pack_data[0..size])?;
                    osc_estimate.search_for_electrons(&buffer_pack_data[0..size]);
                    if osc_estimate.check() {break;}
                }
            }
            oscillator_size = Some(osc_estimate.extract_amplitude(YMIN_PERCENTILE, YMAX_PERCENTILE));
            println!("***TdcLib***: The size for the YMAX and YMIN has been found as {:?}.", oscillator_size); 
        }

        let per_ref = Self {
            tdctype: tdc_type.associate_value(),
            counter: 0,
            counter_offset,
            last_hard_counter: 0,
            counter_overflow: 0,
            begin_time,
            begin_frame: begin_time,
            ticks_to_frame,
            subsample: ratio,
            video_delay: my_settings.video_time,
            period: Some(period),
            high_time,
            low_time,
            new_frame: false,
            time: last_time,
            oscillator_size,
        };
        println!("***Tdc Lib***: Creating a new tdc reference: {:?}.", per_ref);
        Ok(per_ref)
    }
    pub fn new_no_read(tdc_type: TdcType) -> Result<Self, Tp3ErrorKind> {
        println!("***Tdc Lib***: {} has been created (no read).", tdc_type.associate_str());
        let counter_offset = 0;
        let begin_time = 0;
        let last_time = 0;

        let per_ref = Self {
            tdctype: tdc_type.associate_value(),
            counter: 0,
            counter_offset,
            last_hard_counter: 0,
            counter_overflow: 0,
            begin_time,
            begin_frame: begin_time,
            ticks_to_frame: None,
            subsample: 1,
            video_delay: 0,
            period: None,
            high_time: None,
            low_time: None,
            new_frame: false,
            time: last_time,
            oscillator_size: None,
        };
        println!("***Tdc Lib***: Creating a new tdc reference: {:?}.", per_ref);
        Ok(per_ref)
    }
}

pub mod isi_box {
    use crate::errorlib::Tp3ErrorKind;
    use crate::auxiliar::misc::{as_int, as_bytes};
    use std::net::TcpStream;
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;
    use crate::constlib::*;

    pub trait IsiBoxTools {
        fn bind_and_connect(&mut self) -> Result<(), Tp3ErrorKind>;
        fn configure_scan_parameters(&self, xscan: u32, yscan: u32, pixel_time: u32) -> Result<(), Tp3ErrorKind>;
        fn configure_measurement_type(&self, save_locally: bool) -> Result<(), Tp3ErrorKind>;
        fn new() -> Self;
    }

    pub trait IsiBoxHand {
        type MyOutput;
        fn get_data(&self) -> Self::MyOutput;
        fn send_to_external(&self);
        fn start_threads(&mut self);
        fn stop_threads(&mut self);
    }

    pub struct IsiBoxType<T> {
        sockets: Vec<TcpStream>,
        ext_socket: Option<TcpStream>,
        nchannels: u32,
        data: Arc<Mutex<T>>,
        thread_stop: Arc<Mutex<bool>>,
        thread_handle: Vec<thread::JoinHandle<()>>,
    }

    #[macro_export]
    macro_rules! isi_box_new {
        (spec) => {isi_box::IsiBoxType::<[u32; CHANNELS]>::new()};
        (spim) => {isi_box::IsiBoxType::<Vec<u32>>::new()};
    }

    macro_rules! create_auxiliar {
        (spec) => {Arc::new(Mutex::new([0; CHANNELS]))};
        (spim) => {Arc::new(Mutex::new(Vec::new()))};
    }

    macro_rules! measurement_type {
        (spim) => {1};
        (spec) => {0};
    }
    
    macro_rules! impl_bind_connect {
        ($x: ident, $y: ty, $z: tt) => {
            impl IsiBoxTools for $x<$y> {
                fn bind_and_connect(&mut self) -> Result<(), Tp3ErrorKind>{
                    for _ in 0..self.nchannels {
                        let sock = match TcpStream::connect(ISI_IP_PORT) {
                            Ok(val) => val,
                            Err(_) => return Err(Tp3ErrorKind::IsiBoxCouldNotConnect),
                        };
                        self.sockets.push(sock);
                    }
                    let sock = match TcpStream::connect(ISI_IP_PORT) {
                        Ok(val) => val,
                        Err(_) => return Err(Tp3ErrorKind::IsiBoxCouldNotConnect),
                    };
                    self.ext_socket = Some(sock);
                    Ok(())
                }
                fn configure_scan_parameters(&self, xscan: u32, yscan: u32, pixel_time: u32) -> Result<(), Tp3ErrorKind> {
                    let mut config_array: [u32; 3] = [0; 3];
                    config_array[0] = xscan;
                    config_array[1] = yscan;
                    config_array[2] = pixel_time;
                    let mut sock = &self.sockets[0];
                    match sock.write(as_bytes(&config_array)) {
                        Ok(size) => {println!("data sent to configure scan parameters: {}", size);},
                        Err(_) => {return Err(Tp3ErrorKind::IsiBoxCouldNotSetParameters);},
                    };
                    Ok(())
                }
                fn configure_measurement_type(&self, save_locally: bool) -> Result<(), Tp3ErrorKind> {
                    let mut config_array: [u32; 1] = [0; 1];
                    config_array[0] = measurement_type!($z);
                    if save_locally {config_array[0] = 2;}
                    let mut sock = &self.sockets[0];
                    match sock.write(as_bytes(&config_array)) {
                        Ok(size) => {println!("data sent to configure the measurement type: {}", size);},
                        Err(_) => {return Err(Tp3ErrorKind::IsiBoxCouldNotConfigure);},
                    };
                    Ok(())
                }
                fn new() -> Self{
                    Self {
                        sockets: Vec::new(),
                        ext_socket: None,
                        nchannels: CHANNELS as u32,
                        data: create_auxiliar!($z),
                        thread_stop: Arc::new(Mutex::new(false)),
                        thread_handle: Vec::new(),
                    }
                }
            }
        }
    }

    impl_bind_connect!(IsiBoxType, [u32; CHANNELS], spec);
    impl_bind_connect!(IsiBoxType, Vec<u32>, spim);

    
    impl IsiBoxHand for IsiBoxType<Vec<u32>> {
        type MyOutput = Vec<u32>;
        fn get_data(&self) -> Vec<u32> {
            let nvec_arclist = Arc::clone(&self.data);
            let mut num = nvec_arclist.lock().unwrap();
            let output = (*num).clone();
            (*num).clear();
            output
        }
        fn send_to_external(&self) {
            let nvec_arclist = Arc::clone(&self.data);
            let mut num = nvec_arclist.lock().unwrap();
            //if (*num).len() > 0 {
            //    if (self.ext_socket.as_ref().expect("The external sockets is not present")).write(&*num).is_err() {println!("Could not send data through the external socket.")}
            //    println!("data sent size is: {}", (*num).len());
            //}
            (*num).clear();
        }
        fn start_threads(&mut self) {
            let mut channel_index = self.nchannels - 1;
            
            for _ in 0..self.nchannels {
                let nvec_arclist = Arc::clone(&self.data);
                let stop_arc = Arc::clone(&self.thread_stop);
                let mut val = self.sockets.pop().unwrap();
                let mut buffer = vec![0_u8; 8_192];
                val.set_nonblocking(true).unwrap();
                let handle = thread::spawn(move || {
                    loop {
                        let mut num = nvec_arclist.lock().unwrap();
                        while let Ok(size) = val.read(&mut buffer) {
                            as_int(&buffer[0..size]).iter().for_each(|&x| (*num).push((x * PIXELS_X) + 1025 + channel_index));
                        }
                        drop(num);
                        let stop_val = stop_arc.lock().unwrap();
                        if *stop_val {break;}
                        drop(stop_val);
                        thread::sleep(Duration::from_millis(THREAD_POOL_PERIOD));
                    };
                });
                self.thread_handle.push(handle);
                if channel_index>0 {channel_index-=1;}
            }
        }
        fn stop_threads(&mut self) {
            let val = Arc::clone(&self.thread_stop);
            let mut num = val.lock().unwrap();
            *num = true;
            drop(num);
            for _ in 0..self.nchannels {
                self.thread_handle.pop().unwrap().join().unwrap();
            }

        }
    }

    impl IsiBoxHand for IsiBoxType<[u32; CHANNELS]> {
        type MyOutput = [u32; CHANNELS];
        fn get_data(&self) -> [u32; CHANNELS] {
            let counter_arclist = Arc::clone(&self.data);
            let mut num = counter_arclist.lock().unwrap();
            let output = *num;
            (*num).iter_mut().for_each(|x| *x = 0);
            output
        }

        fn send_to_external(&self) {
            let counter_arclist = Arc::clone(&self.data);
            let mut num = counter_arclist.lock().unwrap();
            println!("data sent size is: {:?}", (*num));
            if (self.ext_socket.as_ref().expect("The external sockets is not present")).write(as_bytes(&*num)).is_err() {println!("Could not send data through the external socket.")}
            (*num).iter_mut().for_each(|x| *x = 0);
        }
        fn start_threads(&mut self) {
            let counter_arclist = Arc::clone(&self.data);
            let stop_arc = Arc::clone(&self.thread_stop);
            let mut val = self.sockets.remove(0);
            val.set_nonblocking(true).unwrap();
            let mut buffer = vec![0_u8; CHANNELS * 4];
            let handle = thread::spawn(move || {
                loop {
                    let mut num = counter_arclist.lock().unwrap();
                    while let Ok(size) = val.read(&mut buffer) {
                        //println!("{} and {}", size, stop_val);
                        //if *stop_val == true {
                        //    println!("leaving sinde");
                        //    break;
                        //}
                        (*num).iter_mut().zip(as_int(&buffer[0..size]).iter()).for_each(|(a, b)| *a+=*b);
                    }
                    drop(num);
                    let stop_val = stop_arc.lock().unwrap();
                    if *stop_val {break;}
                    drop(stop_val);
                    thread::sleep(Duration::from_millis(THREAD_POOL_PERIOD));
                }
            });
            self.thread_handle.push(handle);
        }
        fn stop_threads(&mut self) {
            let val = Arc::clone(&self.thread_stop);
            let mut num = val.lock().unwrap();
            *num = true;
            drop(num);
            self.thread_handle.pop().unwrap().join().unwrap();
        }
    }
}

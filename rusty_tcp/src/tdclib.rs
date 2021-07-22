//!`tdclib` is a collection of tools to facilitate manipulation and choice of tdcs. Module is built
//!in around `TdcType` enum.

use std::net::TcpStream;
use std::io::Read;

mod tdcvec {
    use crate::tdclib::TdcType;
    
    pub fn search_any_tdc(data: &[u8], tdc_vec: &mut Vec<(f64, TdcType)>, last_ci: &mut u8) {
        use crate::packetlib::{Packet, PacketEELS};
        
        let file_data = data;
        let mut packet_chunks = file_data.chunks_exact(8);

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => {*last_ci = nci},
                _ => {
                    let packet = PacketEELS { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        6 => {
                            let time = packet.tdc_time_norm();
                            if let Some(tdc) = TdcType::associate_value_to_enum(packet.tdc_type()) {
                                tdc_vec.push( (time, tdc) );
                            }
                        },
                        _ => {},
                    };
                },
            };
        };
    }

    pub fn check_tdc(tdc_vec: &Vec<(f64, TdcType)>, tdc_choosen: &TdcType) -> bool {
        let mut counter = 0;
        for (_time, tdc_type) in tdc_vec {
            if tdc_type.associate_value() == tdc_choosen.associate_value() {counter+=1;}
        }
        if counter>=5 {true} else {false}
    }
    
    ///Outputs the time list for a specific TDC.
    fn get_timelist(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: &TdcType) -> Vec<f64> {
        let result: Vec<_> = tdc_vec.iter()
            .filter(|(_time, tdct)| tdct.associate_value()==tdc_type.associate_value())
            .map(|(time, _tdct)| *time)
            .collect();
        result
    }
    
    ///Returns the + time of a periodic TDC.
    pub fn find_high_time(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: &TdcType) -> f64 {
        let fal_tdc_type = match tdc_type {
            TdcType::TdcOneRisingEdge | TdcType::TdcOneFallingEdge => TdcType::TdcOneFallingEdge,
            TdcType::TdcTwoRisingEdge | TdcType::TdcTwoFallingEdge => TdcType::TdcTwoFallingEdge,
        };
        
        let ris_tdc_type = match tdc_type {
            TdcType::TdcOneRisingEdge | TdcType::TdcOneFallingEdge => TdcType::TdcOneRisingEdge,
            TdcType::TdcTwoRisingEdge | TdcType::TdcTwoFallingEdge => TdcType::TdcTwoRisingEdge,
        };

        let mut fal = get_timelist(tdc_vec, &fal_tdc_type);
        let mut ris = get_timelist(tdc_vec, &ris_tdc_type);
        let last_fal = fal.pop().expect("Please get at least 01 falling Tdc");
        let last_ris = ris.pop().expect("Please get at least 01 rising Tdc");
        if last_fal - last_ris > 0.0 {
            last_fal - last_ris 
        } else {
            last_fal - ris.pop().expect("Please get at least 02 rising Tdc's.")
        }
    }
    
    ///Returns the period time interval between lines.
    pub fn find_period(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: &TdcType) -> f64 {
        let mut tdc_time = get_timelist(tdc_vec, tdc_type);
        tdc_time.pop().expect("Please get at least 02 Tdc's") - tdc_time.pop().expect("Please get at least 02 Tdc's")
    }
    
    pub fn get_counter(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: &TdcType) -> usize {
        let counter = tdc_vec.iter()
            .filter(|(_time, tdct)| tdct.associate_value()==tdc_type.associate_value())
            .count();
        counter
    }
    
    pub fn get_lasttime(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: &TdcType) -> f64 {
        let last_time = tdc_vec.iter()
            .filter(|(_time, tdct)| tdct.associate_value()==tdc_type.associate_value())
            .map(|(time, _tdct)| *time)
            .last().unwrap();
        last_time
    }
    
    pub fn get_begintime(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: &TdcType) -> f64 {
        let begin_time = tdc_vec.iter()
            .filter(|(_time, tdct)| tdct.associate_value()==tdc_type.associate_value())
            .map(|(time, _tdct)| *time)
            .next().unwrap();
        begin_time
    }
}


///The four types of TDC's.
pub enum TdcType {
    TdcOneRisingEdge,
    TdcOneFallingEdge,
    TdcTwoRisingEdge,
    TdcTwoFallingEdge,
}

impl TdcType {
    ///Convenient method. Return value is the 4 bits associated to each TDC.
    pub fn associate_value(&self) -> u8 {
        match *self {
            TdcType::TdcOneRisingEdge => 15,
            TdcType::TdcOneFallingEdge => 10,
            TdcType::TdcTwoRisingEdge => 14,
            TdcType::TdcTwoFallingEdge => 11,
        }
    }

    fn associate_str(&self) -> String {
        match *self {
            TdcType::TdcOneRisingEdge => String::from("Tdc 01 Rising Edge"),
            TdcType::TdcOneFallingEdge => String::from("Tdc 01 Falling Edge"),
            TdcType::TdcTwoRisingEdge => String::from("Tdc 02 Rising Edge"),
            TdcType::TdcTwoFallingEdge => String::from("Tdc 02 Falling Edge"),
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

pub trait TdcControl {
    fn id(&self) -> u8;
    fn upt(&mut self, time: f64);
    fn counter(&self) -> usize;
    fn time(&self) -> f64;
    fn period(&self) -> f64;
    fn is_periodic(&self) -> bool;
}

pub struct PeriodicTdcRef {
    pub tdctype: u8,
    pub counter: usize,
    pub begin: f64,
    pub period: f64,
    pub high_time: f64,
    pub low_time: f64,
    pub time: f64,
}

impl TdcControl for PeriodicTdcRef {
    fn id(&self) -> u8 {
        self.tdctype
    }

    fn upt(&mut self, time: f64) {
        self.time = time;
        self.counter+=1;
    }

    fn counter(&self) -> usize {
        self.counter
    }

    fn time(&self) -> f64 {
        self.time
    }

    fn period(&self) -> f64 {
        self.period
    }

    fn is_periodic(&self) -> bool {
        true
    }

}

impl PeriodicTdcRef {
    
    pub fn tcp_new_ref(tdc_type: TdcType, sock: &mut TcpStream) -> PeriodicTdcRef {

        let mut buffer_pack_data = vec![0; 16384];
        let mut tdc_vec:Vec<(f64, TdcType)> = Vec::new();
        let mut ci = 0u8;

        println!("***Tdc Lib***: Searching for Tdc: {}.", tdc_type.associate_str());
        loop {
            if let Ok(size) = sock.read(&mut buffer_pack_data) {
                if size>0 {
                    let new_data = &buffer_pack_data[0..size];
                    tdcvec::search_any_tdc(new_data, &mut tdc_vec, &mut ci);
                    if tdcvec::check_tdc(&tdc_vec, &tdc_type)==true {break;}
                }
            }
        }
        println!("***Tdc Lib***: {} has been found.", tdc_type.associate_str());
        let counter = tdcvec::get_counter(&tdc_vec, &tdc_type);
        let begin_time = tdcvec::get_begintime(&tdc_vec, &tdc_type);
        let last_time = tdcvec::get_lasttime(&tdc_vec, &tdc_type);
        let high_time = tdcvec::find_high_time(&tdc_vec, &tdc_type);
        let period = tdcvec::find_period(&tdc_vec, &tdc_type);
        let low_time = period - high_time;
        println!("***Tdc Lib***: Creating a new Tdc reference from {}. Number of detected triggers is {}. Last trigger time (ms) is {}. ON interval (ms) is {}. Period (ms) is {}.", tdc_type.associate_str(), counter, last_time*1.0e3, high_time*1.0e3, period*1.0e3);
        PeriodicTdcRef {
            tdctype: tdc_type.associate_value(),
            counter: counter,
            begin: begin_time,
            period: period,
            high_time: high_time,
            low_time: low_time,
            time: last_time,
        }
    }

}

pub struct NonPeriodicTdcRef {
    pub tdctype: u8,
    pub counter: usize,
    pub time: Vec<f64>,
}

impl TdcControl for NonPeriodicTdcRef {
    fn id(&self) -> u8 {
        self.tdctype
    }

    fn upt(&mut self, time: f64) {
        self.time.pop().expect("***Tdc Lib***: There is no element to exclude from NonPeriodicTDC.");
        self.time.insert(0, time);
        self.counter+=1;
    }

    fn counter(&self) -> usize {
        self.counter
    }

    fn time(&self) -> f64 {
        self.time[0]
    }

    fn period(&self) -> f64 {
        self.time[0] - self.time[1]
    }
    
    fn is_periodic(&self) -> bool {
        false
    }
}

impl NonPeriodicTdcRef {
    

    pub fn new_ref(tdc_type: TdcType) -> NonPeriodicTdcRef {
        NonPeriodicTdcRef {
            tdctype: tdc_type.associate_value(),
            counter: 0,
            time: vec![0.0; 100],
        }
    }
}

pub struct NonPeriodicTdcRefMonitor {
    pub tdctype: u8,
    pub counter: usize,
    pub time: Vec<f64>,
    average: usize,
}

impl TdcControl for NonPeriodicTdcRefMonitor {
    fn id(&self) -> u8 {
        self.tdctype
    }

    fn upt(&mut self, time: f64) {
        self.time.pop().expect("***Tdc Lib***: There is no element to exclude from NonPeriodicTDC.");
        self.time.insert(0, time);
        self.counter+=1;
        if self.counter % self.average == 0 {println!("{:?}", (self.average as f64 / (self.time[0] - self.time[self.average-1])) as usize );}
    }

    fn counter(&self) -> usize {
        self.counter
    }

    fn time(&self) -> f64 {
        self.time[0]
    }

    fn period(&self) -> f64 {
        self.time[0] - self.time[1]
    }
    
    fn is_periodic(&self) -> bool {
        false
    }
}

impl NonPeriodicTdcRefMonitor {
    

    pub fn new_ref(tdc_type: TdcType, avg: usize) -> NonPeriodicTdcRefMonitor {
        NonPeriodicTdcRefMonitor {
            tdctype: tdc_type.associate_value(),
            counter: 0,
            time: vec![0.0; avg],
            average: avg,
        }
    }
}

pub struct NoTdcRef {
}

impl TdcControl for NoTdcRef {
    fn id(&self) -> u8 {
        0
    }

    fn upt(&mut self, _: f64) {
    }

    fn counter(&self) -> usize {
        0
    }

    fn time(&self) -> f64 {
        0.0
    }

    fn period(&self) -> f64 {
        0.0
    }
    
    fn is_periodic(&self) -> bool {
        false
    }
}

impl NoTdcRef {
    pub fn new_ref() -> NoTdcRef {
        NoTdcRef{}
    }
}


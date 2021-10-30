//!`tdclib` is a collection of tools to facilitate manipulation and choice of tdcs. Module is built
//!in around `TdcType` enum.

use std::io::Read;
use std::time::{Duration, Instant};

mod tdcvec {
    use crate::tdclib::{TdcType, TdcError};
    use crate::packetlib::{Packet, PacketEELS as Pack};

    pub struct TdcSearch {
        data: Vec<(usize, TdcType)>,
        how_many: usize,
        tdc_choosen: TdcType,
        initial_counter: Option<usize>,
        last_counter: u16,
    }

    impl TdcSearch {

        pub fn new(tdc_choosen: TdcType, how_many: usize) -> Self {
            TdcSearch{
                data: Vec::new(),
                how_many: how_many,
                tdc_choosen: tdc_choosen,
                initial_counter: None,
                last_counter: 0,
            }
        }

        fn add_tdc(&mut self, packet: &Pack) {
            if let Some(tdc) = TdcType::associate_value_to_enum(packet.tdc_type()) {
                let time = packet.tdc_time_norm();
                self.data.push( (time, tdc) );
                if packet.tdc_type() == self.tdc_choosen.associate_value() {
                    self.last_counter = packet.tdc_counter();
                    self.initial_counter = match self.initial_counter {
                        None => Some(packet.tdc_counter() as usize),
                        Some(val) => Some(val),
                    };
                }
            }
        }

        pub fn check_tdc(&self) -> Result<bool, TdcError> {
            let mut counter = 0;
            for (_time, tdc_type) in &self.data {
                if tdc_type.associate_value() == self.tdc_choosen.associate_value() {counter+=1;}
            }
            if counter>self.how_many {
                self.check_ascending_order()?;
                Ok(true)
            } else {Ok(false)}
        }

        fn get_timelist(&self, which: &TdcType) -> Vec<usize> {
            let result: Vec<_> = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value() == which.associate_value())
                .map(|(time, _tdct)| *time)
                .collect();
            result
        }
        
        fn get_auto_timelist(&self) -> Vec<usize> {
            let result: Vec<_> = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value() == self.tdc_choosen.associate_value())
                .map(|(time, _tdct)| *time)
                .collect();
            result
        }

        fn check_ascending_order(&self) -> Result<(), TdcError> {
            let time_list = self.get_auto_timelist();
            let result = time_list.iter().zip(time_list.iter().skip(1)).find(|(a, b)| a>b);
            if result.is_some() {Err(TdcError::NotAscendingOrder)}
            else {Ok(())}
        }

        pub fn find_high_time(&self) -> usize {
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
            let last_fal = fal.pop().expect("Please get at least 01 falling Tdc");
            let last_ris = ris.pop().expect("Please get at least 01 rising Tdc");
            if last_fal > last_ris {
                last_fal - last_ris 
            } else {
                last_fal - ris.pop().expect("Please get at least 02 rising Tdc's.")
            }
        }
        
        pub fn find_period(&self) -> Result<usize, TdcError> {
            let mut tdc_time = self.get_auto_timelist();
            let last = tdc_time.pop().expect("Please get at least 02 Tdc's");
            let before_last = tdc_time.pop().expect("Please get at least 02 Tdc's");
            if last > before_last {
                Ok(last - before_last)
            } else {
                Err(TdcError::BadPeriod)
            }
        }
        
        pub fn get_counter(&self) -> Result<usize, TdcError> {
            let counter = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value()==self.tdc_choosen.associate_value())
                .count();
            Ok(counter)
        }

        pub fn get_counter_offset(&self) -> usize {
            self.initial_counter.expect("***Tdc Lib***: Tdc initial counter offset was not found.")
        }

        pub fn get_last_hardware_counter(&self) -> u16 {
            self.last_counter
        }

        pub fn get_lasttime(&self) -> usize {
            let last_time = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value()==self.tdc_choosen.associate_value())
                .map(|(time, _tdct)| *time)
                .last().unwrap();
            last_time
        }

        pub fn get_begintime(&self) -> usize {
            let begin_time = self.data.iter()
                .filter(|(_time, tdct)| tdct.associate_value()==self.tdc_choosen.associate_value())
                .map(|(time, _tdct)| *time)
                .next().unwrap();
            begin_time
        }
    }

    pub fn search_any_tdc(data: &[u8], tdc_struct: &mut TdcSearch, last_ci: &mut usize) {

        let file_data = data;
        let mut packet_chunks = file_data.chunks_exact(8);

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => {*last_ci = nci as usize},
                _ => {
                    let packet = Pack { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        6 => {
                            tdc_struct.add_tdc(&packet);
                        },
                        _ => {},
                    };
                },
            };
        };
    }

}


///The four types of TDC's.
#[derive(Copy, Clone)]
pub enum TdcType {
    TdcOneRisingEdge,
    TdcOneFallingEdge,
    TdcTwoRisingEdge,
    TdcTwoFallingEdge,
    NoTdc,
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

    ///Check if a given tdc is from the same input line.
    pub fn is_same_inputline(given: u8, check: u8) -> bool {
        match given {
            15 | 10 if check==15 || check==10 => true,
            14 | 11 if check==14 || check==11 => true,
            _ => false,
        }
    }
}


#[derive(Debug)]
pub enum TdcError {
    NoTdcReceived,
    BadPeriod,
    NotAscendingOrder,
    TimepixZeroBytes,
}

pub trait TdcControl {
    fn id(&self) -> u8;
    fn upt(&mut self, time: usize, hard_counter: u16);
    fn counter(&self) -> usize;
    fn time(&self) -> usize;
    fn period(&self) -> Option<usize>;
    fn new<T: Read>(tdc_type: TdcType, sock: &mut T) -> Result<Self, TdcError> where Self: Sized;
}

#[derive(Copy, Clone)]
pub struct PeriodicTdcRef {
    tdctype: u8,
    counter: usize,
    counter_offset: usize,
    last_hard_counter: u16,
    counter_overflow: usize,
    pub begin_frame: usize,
    pub period: usize,
    pub high_time: usize,
    pub low_time: usize,
    pub time: usize,
}

impl TdcControl for PeriodicTdcRef {
    fn id(&self) -> u8 {
        self.tdctype
    }

    fn upt(&mut self, time: usize, hard_counter: u16) {
        if hard_counter < self.last_hard_counter {
            self.counter_overflow += 1;
        }
        self.last_hard_counter = hard_counter;
        self.time = time;
        self.counter = self.last_hard_counter as usize + self.counter_overflow * 4096 - self.counter_offset;
    }

    fn counter(&self) -> usize {
        self.counter
    }

    fn time(&self) -> usize {
        self.time
    }

    fn period(&self) -> Option<usize> {
        Some(self.period)
    }

    fn new<T: Read>(tdc_type: TdcType, sock: &mut T) -> Result<Self, TdcError> {
        let mut buffer_pack_data = vec![0; 16384];
        let mut ci = 0usize;
        let mut tdc_search = tdcvec::TdcSearch::new(tdc_type, 5);
        let start = Instant::now();

        println!("***Tdc Lib***: Searching for Tdc: {}.", tdc_type.associate_str());
        loop {
            if start.elapsed() > Duration::from_secs(10) {return Err(TdcError::NoTdcReceived)}
            //if let Ok(size) = sock.read(&mut buffer_pack_data) {
            if let Ok(()) = sock.read_exact(&mut buffer_pack_data) {
                //if size == 0 {println!("Timepix3 sent zero bytes."); return Err(TdcError::TimepixZeroBytes)}
                //if size % 8 == 0 {
                tdcvec::search_any_tdc(&buffer_pack_data, &mut tdc_search, &mut ci);
                //}
                if tdc_search.check_tdc()? {break;}
            }
        }
        println!("***Tdc Lib***: {} has been found.", tdc_type.associate_str());
        let counter = tdc_search.get_counter()?;
        let counter_offset = tdc_search.get_counter_offset();
        let last_hard_counter = tdc_search.get_last_hardware_counter();
        let begin_time = tdc_search.get_begintime();
        let last_time = tdc_search.get_lasttime();
        let high_time = tdc_search.find_high_time();
        let period = tdc_search.find_period()?;
        let low_time = period - high_time;
        
        println!("***Tdc Lib***: Creating a new Tdc reference from {}. Number of detected triggers is {}. Last trigger time (ns) is {}. ON interval (ns) is {}. Period (ns) is {}. Low time (ns) is {}.", tdc_type.associate_str(), counter, last_time, high_time, period, low_time);
        Ok(Self {
            tdctype: tdc_type.associate_value(),
            counter: counter,
            counter_offset: counter_offset,
            last_hard_counter: last_hard_counter,
            counter_overflow: 0,
            begin_frame: begin_time,
            period: period,
            high_time: high_time,
            low_time: low_time,
            time: last_time,
        })
    }
}

pub struct NonPeriodicTdcRef {
    pub tdctype: u8,
    pub counter: usize,
    pub time: Vec<usize>,
}

impl TdcControl for NonPeriodicTdcRef {
    fn id(&self) -> u8 {
        self.tdctype
    }

    fn upt(&mut self, time: usize, _: u16) {
        self.time.pop().expect("***Tdc Lib***: There is no element to exclude from NonPeriodicTDC.");
        self.time.insert(0, time);
        self.counter+=1;
    }

    fn counter(&self) -> usize {
        self.counter
    }

    fn time(&self) -> usize {
        self.time[0]
    }

    fn period(&self) -> Option<usize> {
        None
    }
    
    fn new<T: Read>(tdc_type: TdcType, _sock: &mut T) -> Result<Self, TdcError> {
        Ok(Self {
            tdctype: tdc_type.associate_value(),
            counter: 0,
            time: vec![0; 5],
        })
    }
    
}

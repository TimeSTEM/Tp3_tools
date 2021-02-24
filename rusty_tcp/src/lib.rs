//!`timepix3` is a collection of tools to run and analyze the detector TimePix3 in live conditions. This software is
//!intented to be run in a different computer in which the data will be shown. Raw data is supossed to
//!be collected via a socket in localhost and be sent to a client prefentiably using a 10 Gbit/s
//!Ethernet.


///Describes how to run the program. DebugStem7482 will collect and send data using lo, while
///Tp3 collects in lo and sends to '192.168.199.11:8088'.
pub enum RunningMode {
    DebugStem7482,
    Tp3,
}

pub struct Config {
    pub data: [u8; 28],
}

impl Config {
    pub fn bin(&self) -> bool {
        match self.data[0] {
            0 => {
                println!("Bin is False.");
                false
            },
            1 => {
                println!("Bin is True.");
                true
            },
            _ => panic!("Binning choice must be 0 | 1."),
        }
    }

    pub fn bytedepth(&self) -> usize {
        match self.data[1] {
            0 => {
                println!("Bitdepth is 8.");
                1
            },
            1 => {
                println!("Bitdepth is 16.");
                2
            },
            2 => {
                println!("Bitdepth is 32.");
                4
            },
            _ => panic!("Bytedepth must be  1 | 2 | 4."),
        }
    }

    pub fn cumul(&self) -> bool {
        match self.data[2] {
            0 => {
                println!("Cumulation mode is OFF.");
                false
            },
            1 => {
                println!("Cumulation mode is ON.");
                true
            },
            _ => panic!("Cumulation must be 0 | 1."),
        }
    }

    pub fn mode(&self) -> u8 {
        match self.data[3] {
            0 => {
                println!("Mode is Focus/Cumul.");
            },
            1 => {
                println!("Mode is SpimTP.");
            },
            2 => {
                println!("Time resolved mode");
            },
            _ => panic!("Spim config must be 0 | 1."),
        };
        self.data[3]
    }

    pub fn xspim_size(&self) -> usize {
        let x = (self.data[4] as usize)<<8 | (self.data[5] as usize);
        println!("X Spim size is {}", x);
        x
    }
    
    pub fn yspim_size(&self) -> usize {
        let y = (self.data[6] as usize)<<8 | (self.data[7] as usize);
        println!("Y Spim size is {}", y);
        y
    }
    
    pub fn xscan_size(&self) -> usize {
        let x = (self.data[8] as usize)<<8 | (self.data[9] as usize);
        println!("X Scan size is {}", x);
        x
    }
    
    pub fn yscan_size(&self) -> usize {
        let y = (self.data[10] as usize)<<8 | (self.data[11] as usize);
        println!("Y Scan size is {}", y);
        y
    }

    pub fn spimoverscanx(&self) -> usize {
        let xspim = (self.data[4] as usize)<<8 | (self.data[5] as usize);
        let xscan = (self.data[8] as usize)<<8 | (self.data[9] as usize);
        let var = xscan / xspim;
        match var {
            0 => {
                println!("Xratio is 1.");
                1
            },
            _ => {
                println!("Xratio is {}.", var);
                var
            },
        }
    }
    
    pub fn spimoverscany(&self) -> usize {
        let yspim = (self.data[6] as usize)<<8 | (self.data[7] as usize);
        let yscan = (self.data[10] as usize)<<8 | (self.data[11] as usize);
        let var = yscan / yspim;
        match var {
            0 => {
                println!("Yratio is 1.");
                1
            },
            _ => {
                println!("Yratio is {}.", var);
                var
            },
        }
    }

    pub fn time_delay(&self) -> f64 {
        let mut array: [u8; 8] = [0; 8];
        for (i, val) in array.iter_mut().enumerate() {
            *val = self.data[i+12]
        }
        let val = f64::from_be_bytes(array);
        println!("Time delay is (ns) {}", val);
        val
    }
    
    pub fn time_width(&self) -> f64 {
        let mut array: [u8; 8] = [0; 8];
        for (i, val) in array.iter_mut().enumerate() {
            *val = self.data[i+20]
        }
        let val = f64::from_be_bytes(array);
        println!("Time width is (ns) {}", val);
        val
    }
}

pub enum TdcType {
    TdcOneRisingEdge,
    TdcOneFallingEdge,
    TdcTwoRisingEdge,
    TdcTwoFallingEdge,
}

impl TdcType {
    pub fn associate_value(&self) -> u8 {
        match *self {
            TdcType::TdcOneRisingEdge => 15,
            TdcType::TdcOneFallingEdge => 10,
            TdcType::TdcTwoRisingEdge => 14,
            TdcType::TdcTwoFallingEdge => 11,
        }
    }

    pub fn associate_string(&self) -> &str {
        match *self {
            TdcType::TdcOneRisingEdge => "One_Rising",
            TdcType::TdcOneFallingEdge => "One_Falling",
            TdcType::TdcTwoRisingEdge => "Two_Rising",
            TdcType::TdcTwoFallingEdge => "Two_Falling",
        }
    }
    
    fn count_tdcs(tdc_vec: &Vec<(f64, TdcType)>) -> [usize; 4] {
        let mut result = [0usize; 4];
        for (_time, tdc_type) in tdc_vec {
            match tdc_type {
                TdcType::TdcOneRisingEdge => result[0]+=1,
                TdcType::TdcOneFallingEdge => result[1]+=1,
                TdcType::TdcTwoRisingEdge => result[2]+=1,
                TdcType::TdcTwoFallingEdge => result[3]+=1,
            }
        }
        result
    }

    pub fn check_all_tdcs(min: &[usize; 4], tdc_vec: &Vec<(f64, TdcType)>) -> bool {
        let val = TdcType::count_tdcs(tdc_vec);
        let how_many = val.iter().zip(min.iter()).filter(|(min, val)| min>=val).count();
        match how_many {
            4 => true,
            _ => false,
        }
    }

    pub fn check_each_tdcs(min: &[usize; 4], tdc_vec: &Vec<(f64, TdcType)>) -> Vec<bool> {
        let val = TdcType::count_tdcs(tdc_vec);
        let seq = val.iter().zip(min.iter()).map(|(min, val)| min>=val).collect::<Vec<bool>>();
        seq
    }
    
    pub fn vec_from_tdc(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: u8) -> Vec<f64> {
        let result: Vec<_> = tdc_vec.iter()
            .filter(|(_time, tdct)| tdct.associate_value()==tdc_type)
            .map(|(time, _tdct)| *time)
            .collect();
        result
    }

    pub fn last_time_from_tdc(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: u8) -> f64 {
        let last_time = tdc_vec.iter()
            .filter(|(_time, tdct)| tdct.associate_value()==tdc_type)
            .map(|(time, _tdct)| time)
            .last().unwrap();
        *last_time
    }
                
    pub fn howmany_from_tdc(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: u8) -> usize {
        let counter = tdc_vec.iter()
            .filter(|(_time, tdct)| tdct.associate_value()==tdc_type)
            .count();
        counter
    }

}

pub struct Packet<'a> {
    pub chip_index: u8,
    pub data: &'a [u8],
}

impl<'a> Packet<'a> {
    
    pub fn x(&self) -> usize {
        let temp = ((((self.data[6] & 224))>>4 | ((self.data[7] & 15))<<4) | (((self.data[5] & 112)>>4)>>2)) as usize;
        match self.chip_index {
            0 => 255 - temp,
            1 => 255 * 4 - temp,
            2 => 255 * 3 - temp,
            3 => 255 * 2 - temp,
            _ => temp,
        }
    }
    
    pub fn x_unmod(&self) -> usize {
        !((((self.data[6] & 224))>>4 | ((self.data[7] & 15))<<4) | (((self.data[5] & 112)>>4)>>2)) as usize
    }
    
    pub fn y(&self) -> usize {
        (   ( ((self.data[5] & 128))>>5 | ((self.data[6] & 31))<<3 ) | ( (((self.data[5] & 112)>>4)) & 3 )   ) as usize
    }

    pub fn id(&self) -> u8 {
        (self.data[7] & 240) >> 4
    }

    pub fn spidr(&self) -> u16 {
        (self.data[0] as u16) | (self.data[1] as u16)<<8
    }

    pub fn ftoa(&self) -> u8 {
        self.data[2] & 15
    }

    pub fn tot(&self) -> u16 {
        ((self.data[2] & 240) as u16)>>4 | ((self.data[3] & 63) as u16)<<4
    }

    pub fn toa(&self) -> u16 {
        ((self.data[3] & 192) as u16)>>6 | (self.data[4] as u16)<<2 | ((self.data[5] & 15) as u16)<<10
    }

    pub fn ctoa(&self) -> u32 {
        let toa = ((self.data[3] & 192) as u32)>>6 | (self.data[4] as u32)<<2 | ((self.data[5] & 15) as u32)<<10;
        let ftoa = (self.data[2] & 15) as u32;
        (toa << 4) | (!ftoa & 15)
    }
    
    pub fn electron_time(&self) -> f64 {
        let spidr = (self.data[0] as u16) | (self.data[1] as u16)<<8;
        let toa = ((self.data[3] & 192) as u32)>>6 | (self.data[4] as u32)<<2 | ((self.data[5] & 15) as u32)<<10;
        let ftoa = (self.data[2] & 15) as u32;
        let ctoa = (toa << 4) | (!ftoa & 15);
        ((spidr as f64) * 25.0 * 16384.0 + (ctoa as f64) * 25.0 / 16.0) / 1e9
    }

    pub fn tdc_coarse(&self) -> u64 {
        ((self.data[1] & 254) as u64)>>1 | ((self.data[2]) as u64)<<7 | ((self.data[3]) as u64)<<15 | ((self.data[4]) as u64)<<23 | ((self.data[5] & 15) as u64)<<31
    }
    
    pub fn tdc_fine(&self) -> u8 {
        (self.data[0] & 224)>>5 | (self.data[1] & 1)<<3
    }

    pub fn tdc_counter(&self) -> u16 {
        ((self.data[5] & 240) as u16) >> 4 | (self.data[6] as u16) << 4
    }

    pub fn tdc_type(&self) -> u8 {
        self.data[7] & 15 
    }

    pub fn tdc_time(&self) -> f64 {
        let coarse = ((self.data[1] & 254) as u64)>>1 | ((self.data[2]) as u64)<<7 | ((self.data[3]) as u64)<<15 | ((self.data[4]) as u64)<<23 | ((self.data[5] & 15) as u64)<<31;
        let fine = (self.data[0] & 224)>>5 | (self.data[1] & 1)<<3;
        (coarse as f64) * (1.0/320e6) + (fine as f64) * 260e-12
    }
    
    pub fn tdc_time_norm(&self) -> f64 {
        let coarse = ((self.data[1] & 254) as u64)>>1 | ((self.data[2]) as u64)<<7 | ((self.data[3]) as u64)<<15 | ((self.data[4]) as u64)<<23 | ((self.data[5] & 15) as u64)<<31;
        let fine = (self.data[0] & 224)>>5 | (self.data[1] & 1)<<3;
        let time = (coarse as f64) * (1.0/320e6) + (fine as f64) * 260e-12;
        time - (time / (26843545600.0 * 1e-9)).floor() * 26843545600.0 * 1e-9
    }

    pub fn tdc_type_as_enum(&self) -> Result<TdcType, &str> {
        match self.data[7] & 15 {
            15 => Ok(TdcType::TdcOneRisingEdge),
            10 => Ok(TdcType::TdcOneFallingEdge),
            14 => Ok(TdcType::TdcTwoRisingEdge),
            11 => Ok(TdcType::TdcTwoFallingEdge),
            _ => Err("Bad TDC receival"),
        }
    }

    pub fn is_tdc_type_oneris(&self) -> Result<bool, &str> {
        match self.data[7] & 15 {
            15 => Ok(true),
            10 | 14 | 11 => Ok(false),
            _ => Err("Bad TDC receival"),
        }
    }
    
    pub fn is_tdc_type_onefal(&self) -> Result<bool, &str> {
        match self.data[7] & 15 {
            10 => Ok(true),
            15 | 14 | 11 => Ok(false),
            _ => Err("Bad TDC receival"),
        }
    }
    
    pub fn is_tdc_type_tworis(&self) -> Result<bool, &str> {
        match self.data[7] & 15 {
            14 => Ok(true),
            10 | 15 | 11 => Ok(false),
            _ => Err("Bad TDC receival"),
        }
    }

    pub fn is_tdc_type_twofal(&self) -> Result<bool, &str> {
        match self.data[7] & 15 {
            11 => Ok(true),
            10 | 14 | 15 => Ok(false),
            _ => Err("Bad TDC receival"),
        }
    }
    
    pub fn calc_elec_time(spidr: u16, toa: u16, ftoa: u8) -> f64 {
        let ctoa = ((toa as u32 )<<4) | (!(ftoa as u32) & 15);
        ((spidr as f64) * 25.0 * 16384.0 + (ctoa as f64) * 25.0 / 16.0) / 1e9
    }

    pub fn calc_tdc_time(coarse: u64, fine: u8) -> f64 {
        (coarse as f64) * (1.0/320e6) + (fine as f64) * 260e-12
    }

    pub fn append_to_array(data: &mut [u8], index:usize, bytedepth: usize) -> bool{
        let index = index * bytedepth;
        match bytedepth {
            4 => {
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
                false
            },
            2 => {
                data[index+1] = data[index+1].wrapping_add(1);
                if data[index+1]==0 {
                    data[index] = data[index].wrapping_add(1);
                    true
                } else {
                    false
                }
            },
            1 => {
                data[index] = data[index].wrapping_add(1);
                if data[index]==0 {
                    true
                } else {
                    false
                }
            },
            _ => {panic!("Bytedepth must be 1 | 2 | 4.");},
        }
    }

    pub fn append_to_index_array(data: &mut Vec<u8>, index: usize) {
        data.push(((index & 4_278_190_080)>>24) as u8);
        data.push(((index & 16_711_680)>>16) as u8);
        data.push(((index & 65_280)>>8) as u8);
        data.push((index & 255) as u8);
    }
}


pub mod spectral_image {
    pub fn test() {
        println!("test");
    }

    pub fn find_deadtime(start_line: &[f64], end_line: &[f64]) -> f64 {
        if (start_line[1] - end_line[1])>0.0 {start_line[1] - end_line[1]} else {start_line[2] - end_line[1]}
    }

    pub fn find_interval(start_line: &[f64], deadtime: f64) -> f64 {
        (start_line[2] - start_line[1]) - deadtime
    }
}


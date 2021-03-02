//!`timepix3` is a collection of tools to run and analyze the detector TimePix3 in live conditions. This software is
//!intented to be run in a different computer in which the data will be shown. Raw data is supossed to
//!be collected via a socket in localhost and be sent to a client prefentiably using a 10 Gbit/s
//!Ethernet.

pub mod auxiliar;
pub mod tdclib;

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
    use crate::Packet;

    
    fn build_spim_data(data: &[u8], last_ci: &mut u8, counter: &mut usize, sltdc: &mut f64, spim: (usize, usize), yratio: usize, interval: f64, tdc_kind: u8) -> Vec<u8> {
    
        let mut packet_chunks = data.chunks_exact(8);
        let mut index_data:Vec<u8> = Vec::new();

        while let Some(x) = packet_chunks.next() {
            match x {
                &[84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
                _ => {
                    let packet = Packet { chip_index: *last_ci, data: x};
                    
                    match packet.id() {
                        11 => {
                            let line = (*counter / yratio) % spim.1;
                            let ele_time = packet.electron_time() - 0.000007;
                            if check_if_in(ele_time, sltdc, interval) {
                                let xpos = (spim.0 as f64 * ((ele_time - *sltdc)/interval)) as usize;
                                let array_pos = packet.x() + 1024*spim.0*line + 1024*xpos;
                                Packet::append_to_index_array(&mut index_data, array_pos);
                            }
                        },
                        6 if packet.tdc_type() == tdc_kind => {
                            *sltdc = packet.tdc_time_norm();
                            *counter+=1;
                        },
                        _ => {},
                    };
                },
            };
        };
        index_data
    }
    

        pub fn find_deadtime(start_line: &[f64], end_line: &[f64]) -> f64 {
        if (start_line[1] - end_line[1])>0.0 {start_line[1] - end_line[1]} else {start_line[2] - end_line[1]}
    }

    pub fn find_interval(start_line: &[f64], deadtime: f64) -> f64 {
        (start_line[2] - start_line[1]) - deadtime
    }

    pub fn check_if_in(ele_time: f64, start_line: &f64, interval: f64) -> bool {
        if ele_time>*start_line && ele_time<(*start_line + interval) {
        true
        } else {false}
    }
}

pub mod tr_spectrum {
    
    pub fn check_if_in(time_vec: &Vec<f64>, time: f64, delay: f64, width: f64) -> bool {
        for val in time_vec {
            if time>val+delay && time<val+delay+width {
                return true
            }
        }
        false
    }

    pub fn create_start_vectime(mut at: Vec<f64>) -> Vec<f64> {
        let ref_time:Vec<f64> = [at.pop().unwrap(), at.pop().unwrap()].to_vec();
        ref_time
    }
}


//!`packetlib` is a collection of tools to facilitate manipulation of individual TP3 packets. Module is built
//!in around `Packet` struct.

use crate::auxiliar::value_types::*;
use crate::constlib::*;

#[derive(Copy, Clone, Eq, PartialEq)]
pub struct Packet {
    pub chip_index: u8,
    pub data: u64,
}

impl Packet {

    #[inline]
    pub fn data(&self) -> u64 {
        self.data
    }

    #[inline]
    pub fn ci(&self) -> u8 {
        self.chip_index
    }

    #[inline]
    pub fn x(&self) -> POSITION {
        let temp2 = (((self.data() & 0x0F_E0_00_00_00_00_00_00) >> 52) | ((self.data() & 0x00_00_40_00_00_00_00_00) >> 46)) as POSITION;
        match PIXELS_X {
            1024 | 1025 => {
                if !INVERSE_DETECTOR {
                    match self.ci() {
                        0 => 255 - temp2,
                        1 => 256 * 4 - 1 - temp2,
                        2 => 256 * 3 - 1 - temp2,
                        3 => 256 * 2 - 1 - temp2,
                        _ => panic!("More than four CIs."),
                    }
                } else {
                    match self.ci() {
                        0 => temp2 + 256 * 3,
                        1 => temp2,
                        2 => temp2 + 256,
                        3 => temp2 + 256 * 2,
                        _ => panic!("More than four CIs."),
                    }
                }
            },
            512 => {
                match self.chip_index {
                    0 => 255 - temp2,
                    1 => temp2,
                    2 => temp2 + 256,
                    3 => 256 * 2 - 1 - temp2,
                    _ => panic!("More than four CI."),
                }
            },
            _ => {
                temp2
            },
        }
    }
    
    #[inline]
    pub fn y(&self) -> POSITION {
        let temp = (((self.data() & 0x00_1F_80_00_00_00_00_00) >> 45) | ((self.data() & 0x00_00_30_00_00_00_00_00) >> 44)) as POSITION;
        match PIXELS_Y {
            1024 | 1025 => {
                temp
            },
            512 => {
                match self.chip_index {
                    0 => temp,
                    1 => 256 * 2 - 1 - temp,
                    2 => 256 * 2 - 1 - temp,
                    3 => temp,
                    _ => panic!("More than four CI."),
                }
            },
            _ => temp,
        }
                //(((self.data() >> 45) & 0xFC) | ((self.data() >> 44) & 0x03)) as POSITION
    }

    /*
    fn x_y(&self) -> (POSITION, POSITION) {
        let dcol = (self.data()[6] & 224)>>4 | (self.data()[7] << 4);
        let spix = (self.data()[5] & 128) >> 5 | (self.data()[6] & 31) << 3;
        let pix = (self.data()[5] & 112) >> 4;

        let temp = (dcol | (pix >> 2)) as POSITION;
        let y = (spix | (pix & 3)) as POSITION;
        
        match self.ci() {
            0 => (255 - temp, y),
            1 => (256 * 4 - 1 - temp, y),
            2 => (256 * 3 - 1 - temp, y),
            3 => (256 * 2 - 1 - temp, y),
            _ => panic!("More than four CIs."),
        }
    }
    */

    #[inline]
    pub fn id(&self) -> u8 {
        ((self.data() & 0xF0_00_00_00_00_00_00_00) >> 60) as u8
        //((self.data() >> 60)) as u8
    }

    #[inline]
    pub fn spidr(&self) -> TIME {
        (self.data() & 0x00_00_00_00_00_00_FF_FF) as TIME
    }

    #[inline]
    pub fn ftoa(&self) -> TIME {
        ((self.data() & 0x00_00_00_00_00_0F_00_00) >> 16) as TIME
        //((self.data() >> 16) & 0xF) as TIME
    }

    #[inline]
    pub fn tot(&self) -> u16 {
        ((self.data() & 0x00_00_00_00_3F_F0_00_00) >> 20) as u16
    }

    #[inline]
    pub fn toa(&self) -> TIME {
        ((self.data() & 0x00_00_0F_FF_C0_00_00_00) >> 30) as TIME
        //((self.data() >> 30) & 0x3F_FF) as TIME
    }

    #[inline]
    pub fn ctoa(&self) -> TIME {
        let toa = self.toa();
        let ftoa = self.ftoa();
        (toa << 4) | (!ftoa & 15)
    }

    #[inline]
    pub fn fast_electron_time(&self) -> TIME {
        let spidr = self.spidr();
        let toa = self.toa();
        spidr * 262_144 + toa * 16
    }

    #[inline]
    pub fn electron_time(&self) -> TIME {
        let spidr = self.spidr();
        let ctoa = self.ctoa();
        match CORRECT_ELECTRON_TIME_COARSE {
            false => spidr * 262_144 + ctoa,
            true => {
                match PIXELS_X {
                    1024 | 1025 => {
                        let mut x = self.x();
                        if INVERSE_DETECTOR {
                            x = (4 * 256 - 1) - x;
                        }
                        let t = spidr * 262_144 + ctoa;
                        match x {
                            52..=61 | 306..=317 | 324..=325 | 564..=573 | 820..=829 => t-16,
                            _ => t,
                        }
                    },
                    _ => spidr * 262_144 + ctoa
                }
            }
        }
    }

    #[inline]
    pub fn tdc_coarse(&self) -> TIME {
        ((self.data() & 0x00_00_0F_FF_FF_FF_FE_00) >> 9) as TIME
    }
    
    #[inline]
    pub fn tdc_fine(&self) -> TIME {
        ((self.data() & 0x00_00_00_00_00_00_01_E0) >> 5) as TIME
    }

    #[inline]
    pub fn tdc_counter(&self) -> u16 {
        ((self.data() & 0x00_FF_F0_00_00_00_00_00) >> 44) as u16
    }

    #[inline]
    pub fn tdc_type(&self) -> u8 {
        ((self.data() & 0x0F_00_00_00_00_00_00_00) >> 56) as u8
    }
    
    #[inline]
    pub fn frame_time(&self) -> u64 {
        ((self.data() & 0x00_00_3F_FF_FF_FF_F0_00) >> 12) as u64
    }
    
    #[inline]
    pub fn hit_count(&self) -> u8 {
        ((self.data() & 0x00_00_00_00_00_0F_00_00) >> 16) as u8
    }
    
    #[inline]
    pub fn shutter_packet_count(&self) -> u64 {
        (self.data() & 0x00_00_FF_FF_FF_FF_FF_FF) as u64
    }

    #[inline]
    pub fn tdc_time(&self) -> TIME {
        let coarse = self.tdc_coarse();
        let fine = self.tdc_fine();
        coarse * 2 + fine / 6
    }
    
    #[inline]
    pub fn tdc_time_abs(&self) -> TIME {
        let coarse = self.tdc_coarse();
        let fine = self.tdc_fine();
        coarse * 12 + fine
    }
    
    #[inline]
    pub fn tdc_time_norm(&self) -> TIME {
        let coarse = self.tdc_coarse();
        let fine = self.tdc_fine();
        let time = coarse * 2 + fine / 6;
        time - (time / (17_179_869_184)) * 17_179_869_184
    }

    #[inline]
    pub fn tdc_time_abs_norm(&self) -> TIME {
        let coarse = self.tdc_coarse();
        let fine = self.tdc_fine();
        let time = coarse * 12 + fine;
        time - (time / (103_079_215_104)) * 103_079_215_104
    }

    #[inline]
    pub const fn chip_array() -> (POSITION, POSITION) {
        (PIXELS_X, PIXELS_Y)
    }
}

/*
pub struct InversePacket {
    pub x: usize,
    pub y: usize,
    pub time: usize,
    pub id: usize,
}

use crate::tdclib::TdcType;

impl InversePacket {

    pub fn new_inverse_electron(x: usize, y: usize, time: usize) -> Self {
        
        let x = (!x & 255) | (x & 768);

        InversePacket {
            x,
            y,
            time,
            id: 11
        }
    }

    pub fn new_inverse_tdc(time: usize) -> Self {

        InversePacket {
            x: 0,
            y: 0,
            time,
            id: 6,
        }
    }


    pub fn create_electron_array(&self) -> [u8; 16] {
        let (spidr, toa_ticks, ftoa_ticks) = self.time_to_ticks();
        let tot_ticks = 1023;
        let x_raw = self.x % 256;
        let mut ci: u8 = (self.x >> 8) as u8;

        ci = if ci == 1 {
            3
        } else if ci == 3 {
            1
        } else {
            ci
        };

        let data0: u8 = (spidr & 255) as u8;
        let data1: u8 = ((spidr & 65_280) >> 8) as u8;
        let data2: u8 = (!ftoa_ticks | (tot_ticks & 15) << 4) as u8;
        let data3: u8 = ((tot_ticks & 1_008) >> 4 | (toa_ticks & 3) << 6) as u8;
        let data4: u8 = ((toa_ticks & 1_020) >> 2) as u8;
        let data5: u8 = ((x_raw & 1) << 6 | (self.y & 4) << 5 | (self.y & 3) << 4 | (toa_ticks & 15_360) >> 10) as u8;
        let data6: u8 = ((self.y & 248) >> 3 | (x_raw & 14) << 4) as u8;
        let data7: u8 = ((self.id & 15) << 4 | (x_raw & 240) >> 4) as u8;
        [84, 80, 88, 51, ci, 0, 8, 0, data0, data1, data2, data3, data4, data5, data6, data7]
    }
    
    pub fn create_tdc_array(&self, counter: usize, kind: TdcType) -> [u8; 16] {
        let (ct, ft) = self.tdc_time_to_ticks();
        let res = 0;
        let tdc_type: u8 = kind.associate_value();

        let data0: u8 = ((res & 248) >> 3 | (ft & 7) << 5) as u8;
        let data1: u8 = ((ct & 127) << 1 | (ft & 8) >> 3) as u8;
        let data2: u8 = ((ct & 32_640) >> 7 ) as u8;
        let data3: u8 = ((ct & 8_355_840) >> 15) as u8;
        let data4: u8 = ((ct & 2_139_095_040) >> 23) as u8;
        let data5: u8 = ((counter & 15) << 4 | (ct & 32_212_254_720) >> 31) as u8;
        let data6: u8 = ((counter & 4080) >> 4) as u8;
        let data7: u8 = ((self.id & 15) << 4) as u8 | (tdc_type & 15);
        [84, 80, 88, 51, 0, 0, 8, 0, data0, data1, data2, data3, data4, data5, data6, data7]
    }

    pub fn time_to_ticks(&self) -> (usize, usize, usize) {
        let spidr_ticks = self.time / 409_600;
        let ctoa = self.time % 409_600;
        let toa_ticks = ctoa / 25;
        let ftoa_ticks = (ctoa % 25) * 16 / 25;
        (spidr_ticks, toa_ticks, ftoa_ticks)
    }
    
    pub fn tdc_time_to_ticks(&self) -> (usize, usize) {
        let coarse_ticks = self.time * 320 / 1_000;
        let fine_time = ((self.time * 1000) % 3125) / 1000;
        let fine_ticks = fine_time * 1_000 / 260;
 
        (coarse_ticks, fine_ticks)
    }
}
*/

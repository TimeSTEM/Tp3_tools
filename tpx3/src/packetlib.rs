//!`packetlib` is a collection of tools to facilitate manipulation of individual TP3 packets. Module is built
//!in around `Packet` struct.

use crate::auxiliar::value_types::*;

pub fn packet_change(v: &[u8]) -> &[u64] {
    unsafe {
        std::slice::from_raw_parts(
            v.as_ptr() as *const u64,
            v.len() * std::mem::size_of::<u8>() / std::mem::size_of::<u64>())
    }
}

pub trait Packet {
    fn ci(&self) -> u8;
    fn data(&self) -> u64;

    #[inline]
    fn x(&self) -> POSITION {
        let temp2 = (((self.data() & 0x0F_E0_00_00_00_00_00_00) >> 52) | ((self.data() & 0x00_00_40_00_00_00_00_00) >> 46)) as POSITION;
        
        match self.ci() {
            0 => 255 - temp2,
            1 => 256 * 4 - 1 - temp2,
            2 => 256 * 3 - 1 - temp2,
            3 => 256 * 2 - 1 - temp2,
            _ => panic!("More than four CIs."),
        }
    }
    
    #[inline]
    fn x_raw(&self) -> POSITION {
        (((self.data() & 0x0F_E0_00_00_00_00_00_00) >> 52) | ((self.data() & 0x00_00_40_00_00_00_00_00) >> 46)) as POSITION
    }
    
    #[inline]
    fn y(&self) -> POSITION {
        (((self.data() & 0x00_1F_80_00_00_00_00_00) >> 45) | ((self.data() & 0x00_00_30_00_00_00_00_00) >> 44)) as POSITION
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
    fn id(&self) -> u8 {
        ((self.data() & 0xF0_00_00_00_00_00_00_00) >> 60) as u8
    }

    #[inline]
    fn spidr(&self) -> TIME {
        (self.data() & 0x00_00_00_00_00_00_FF_FF) as TIME
    }

    #[inline]
    fn ftoa(&self) -> TIME {
        ((self.data() & 0x00_00_00_00_00_0F_00_00) >> 16) as TIME
    }

    #[inline]
    fn tot(&self) -> u16 {
        ((self.data() & 0x00_00_00_00_3F_F0_00_00) >> 20) as u16
    }

    #[inline]
    fn toa(&self) -> TIME {
        ((self.data() & 0x00_00_0F_FF_C0_00_00_00) >> 30) as TIME
    }

    #[inline]
    fn ctoa(&self) -> TIME {
        let toa = self.toa();
        let ftoa = self.ftoa();
        (toa << 4) | (!ftoa & 15)
    }

    #[inline]
    fn fast_electron_time(&self) -> TIME {
        let spidr = self.spidr();
        let toa = self.toa();
        spidr * 262_144 + toa * 16
    }
    
    #[inline]
    fn electron_time(&self) -> TIME {
        let spidr = self.spidr();
        let ctoa = self.ctoa();
        spidr * 262_144 + ctoa
    }

    #[inline]
    fn tdc_coarse(&self) -> TIME {
        ((self.data() & 0x00_00_0F_FF_FF_FF_FE_00) >> 9) as TIME
    }
    
    #[inline]
    fn tdc_fine(&self) -> TIME {
        ((self.data() & 0x00_00_00_00_00_00_01_E0) >> 5) as TIME
    }

    #[inline]
    fn tdc_counter(&self) -> u16 {
        ((self.data() & 0x00_FF_F0_00_00_00_00_00) >> 44) as u16
    }

    #[inline]
    fn tdc_type(&self) -> u8 {
        ((self.data() & 0x0F_00_00_00_00_00_00_00) >> 56) as u8
    }
    
    #[inline]
    fn frame_time(&self) -> u64 {
        ((self.data() & 0x00_00_3F_FF_FF_FF_F0_00) >> 12) as u64
    }
    
    #[inline]
    fn hit_count(&self) -> u8 {
        ((self.data() & 0x00_00_00_00_00_0F_00_00) >> 16) as u8
    }
    
    #[inline]
    fn shutter_packet_count(&self) -> u64 {
        (self.data() & 0x00_00_FF_FF_FF_FF_FF_FF) as u64
    }

    #[inline]
    fn tdc_time(&self) -> TIME {
        let coarse = self.tdc_coarse();
        let fine = self.tdc_fine();
        coarse * 2 + fine / 6
    }
    
    #[inline]
    fn tdc_time_abs(&self) -> TIME {
        let coarse = self.tdc_coarse();
        let fine = self.tdc_fine();
        coarse * 12 + fine
    }
    
    #[inline]
    fn tdc_time_norm(&self) -> TIME {
        let coarse = self.tdc_coarse();
        let fine = self.tdc_fine();
        let time = coarse * 2 + fine / 6;
        time - (time / (17_179_869_184)) * 17_179_869_184
    }

    #[inline]
    fn tdc_time_abs_norm(&self) -> TIME {
        let coarse = self.tdc_coarse();
        let fine = self.tdc_fine();
        let time = coarse * 12 + fine;
        time - (time / (103_079_215_104)) * 103_079_215_104
    }

    #[inline]
    //In units of 1.5625 ns
    fn electron_overflow() -> TIME where Self: Sized{
        17_179_869_184
    }

    #[inline]
    //In units of 1.5625 ns
    fn tdc_overflow() -> TIME where Self: Sized {
        68_719_476_736
    }
}

pub struct PacketEELS {
    pub chip_index: u8,
    pub data: u64,
}

impl Packet for PacketEELS {
    fn ci(&self) -> u8 {
        self.chip_index
    }
    fn data(&self) -> u64 {
        self.data
    }
}

impl PacketEELS {
    pub const fn chip_array() -> (POSITION, POSITION) {
        (1025, 256)
    }
}

pub struct PacketEELSInverted {
    pub chip_index: u8,
    pub data: u64,
}

impl Packet for PacketEELSInverted {
    fn ci(&self) -> u8 {
        self.chip_index
    }
    fn data(&self) -> u64 {
        self.data
    }
    
    #[inline]
    fn x(&self) -> POSITION {
        let temp2 = (((self.data() & 0x0F_E0_00_00_00_00_00_00) >> 52) | ((self.data() & 0x00_00_40_00_00_00_00_00) >> 46)) as POSITION;
        
        match self.ci() {
            0 => temp2 + 256 * 3,
            1 => temp2 + 256 * 0,
            2 => temp2 + 256 * 1,
            3 => temp2 + 256 * 2,
            _ => panic!("More than four CIs."),
        }
    }
}

impl PacketEELSInverted {
    pub const fn chip_array() -> (POSITION, POSITION) {
        (1025, 256)
    }
}


pub struct PacketSheerEELS {
    pub chip_index: u8,
    pub data: u64,
}

impl Packet for PacketSheerEELS {
    fn ci(&self) -> u8 {
        self.chip_index
    }
    fn data(&self) -> u64 {
        self.data
    }
    
    #[inline]
    fn x(&self) -> POSITION {
        let hor_shift = self.y() / 16;
        let temp2 = (((self.data() & 0x0F_E0_00_00_00_00_00_00) >> 52) | ((self.data() & 0x00_00_40_00_00_00_00_00) >> 46)) as POSITION;
        
        match self.ci() {
            0 => 255 - temp2 + hor_shift,
            1 => 256 * 4 - 1 - temp2 + hor_shift,
            2 => 256 * 3 - 1 - temp2 + hor_shift,
            3 => 256 * 2 - 1 - temp2 + hor_shift,
            _ => panic!("More than four CIs."),
        }
    }
}

impl PacketSheerEELS {
    pub const fn chip_array() -> (POSITION, POSITION) {
        PacketEELS::chip_array()
    }
}


pub struct TimeCorrectedPacketEELS {
    pub chip_index: u8,
    pub data: u64,
}

impl Packet for TimeCorrectedPacketEELS {
    fn ci(&self) -> u8 {
        self.chip_index
    }
    fn data(&self) -> u64 {
        self.data
    }
    
    #[inline]
    fn x(&self) -> POSITION {
        let temp2 = (((self.data() & 0x0F_E0_00_00_00_00_00_00) >> 52) | ((self.data() & 0x00_00_40_00_00_00_00_00) >> 46)) as POSITION;
        
        match self.ci() {
            0 => temp2 + 256 * 3,
            1 => temp2 + 256 * 0,
            2 => temp2 + 256 * 1,
            3 => temp2 + 256 * 2,
            _ => panic!("More than four CIs."),
        }
    }
    
    fn fast_electron_time(&self) -> TIME {
        let spidr = self.spidr();
        let toa = self.toa();
        spidr * 262_144 + toa * 16
    }
    
    fn electron_time(&self) -> TIME {
        let spidr = self.spidr();
        let ctoa = self.ctoa();
        let x = self.x();
        let t = spidr * 262_144 + ctoa;
        match x {
            //52..=61 | 308..=317 | 560..=573 | 580..=581 | 584..=585 | 592..=593 | 820..=829 => t-16,
            52..=61 | 306..=317 | 324..=325 | 564..=573 | 820..=829 => t-16,
            _ => t,
        }
    }
}

impl TimeCorrectedPacketEELS {
    pub const fn chip_array() -> (POSITION, POSITION) {
        PacketEELS::chip_array()
    }
}

pub struct PacketDiffraction {
    pub chip_index: u8,
    pub data: u64,
}

impl Packet for PacketDiffraction {
    fn ci(&self) -> u8 {
        self.chip_index
    }
    fn data(&self) -> u64 {
        self.data
    }
    fn x(&self) -> POSITION {
        let temp2 = (((self.data() & 0x0F_E0_00_00_00_00_00_00) >> 52) | ((self.data() & 0x00_00_40_00_00_00_00_00) >> 46)) as POSITION;
        match self.chip_index {
            0 => 255 - temp2,
            1 => temp2,
            2 => temp2 + 256,
            3 => 256 * 2 - 1 - temp2,
            _ => panic!("More than four CI."),
        }
    }

    fn y(&self) -> POSITION {
        let temp = (((self.data() & 0x00_1F_80_00_00_00_00_00) >> 45) | ((self.data() & 0x00_00_30_00_00_00_00_00) >> 44)) as POSITION;
        match self.chip_index {
            0 => temp,
            1 => 256 * 2 - 1 - temp,
            2 => 256 * 2 - 1 - temp,
            3 => temp,
            _ => panic!("More than four CI."),
        }
    }
}

impl PacketDiffraction {
    pub const fn chip_array() -> (POSITION, POSITION) {
        (512, 512)
    }
}

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


    /*
    pub fn test_func(&self) {
        let my_inv_packet = InversePacket::new_inverse_electron(128, 100, 3_111_005);

        let my_data = my_inv_packet.create_electron_array();
        let my_packet = PacketEELS {
            chip_index: my_data[4],
            data: &my_data[8..16].try_into().unwrap()
        };

        println!("{} and {} and {} and {} and {} and {}", my_packet.x(), my_packet.y(), my_packet.x_raw(), my_packet.electron_time(), !my_packet.ftoa() & 15, my_packet.tot());
    }

    pub fn tdc_test_func(&self) {
        let my_inv_packet = InversePacket::new_inverse_tdc(26_844_000_000);

        let my_data = my_inv_packet.create_tdc_array(1024, TdcType::TdcTwoFallingEdge);
        let my_packet = PacketEELS {
            chip_index: my_data[4],
            data: &my_data[8..16].try_into().unwrap()
        };

        println!("{} and {} and {} and {}", my_packet.id(), my_packet.tdc_time_norm(), my_packet.tdc_time(), my_packet.tdc_type());
    }
    */

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

//!`packetlib` is a collection of tools to facilitate manipulation of individual TP3 packets. Module is built
//!in around `Packet` struct.

pub trait Packet {
    fn ci(&self) -> usize;
    fn data(&self) -> &[u8; 8];
    fn x(&self) -> usize {
        let temp = ((self.data()[6] & 224)>>4 | (self.data()[7] << 4) | ((self.data()[5] & 112) >> 6)) as usize;
        //(!temp & 255) | (temp & 768)

        match self.ci() {
            0 => 255 - temp,
            1 => 256 * 4 - 1 - temp,
            2 => 256 * 3 - 1 - temp,
            3 => 256 * 2 - 1 - temp,
            _ => panic!("More than four CIs."),
        }
    }
    
    fn x_raw(&self) -> usize {
        let x = (((self.data()[6] & 224)>>4 | (self.data()[7] & 15)<<4) | ((self.data()[5] & 64)>>6)) as usize;
        x
    }
    
    fn y(&self) -> usize {
        let y = (   ( (self.data()[5] & 128)>>5 | (self.data()[6] & 31)<<3 ) | ( ((self.data()[5] & 112)>>4) & 3 )   ) as usize;
        y
    }

    fn x_y(&self) -> (usize, usize) {
        let dcol = (self.data()[6] & 224)>>4 | (self.data()[7] << 4);
        let spix = (self.data()[5] & 128) >> 5 | (self.data()[6] & 31) << 3;
        let pix = (self.data()[5] & 112) >> 4;

        let temp = (dcol | (pix >> 2)) as usize;
        let y = (spix | (pix & 3)) as usize;
        
        match self.ci() {
            0 => (255 - temp, y),
            1 => (256 * 4 - 1 - temp, y),
            2 => (256 * 3 - 1 - temp, y),
            3 => (256 * 2 - 1 - temp, y),
            _ => panic!("More than four CIs."),
        }
    }

    fn id(&self) -> u8 {
        (self.data()[7] & 240) >> 4
    }

    fn spidr(&self) -> u16 {
        (self.data()[0] as u16) | (self.data()[1] as u16) << 8
    }

    fn ftoa(&self) -> u8 {
        self.data()[2] & 15
    }

    fn tot(&self) -> u16 {
        ((self.data()[2] & 240) as u16)>>4 | ((self.data()[3] & 63) as u16)<<4
    }

    fn toa(&self) -> u16 {
        ((self.data()[3] & 192) as u16)>>6 | (self.data()[4] as u16)<<2 | ((self.data()[5] & 15) as u16)<<10
    }

    fn ctoa(&self) -> u32 {
        let toa = ((self.data()[3] & 192) as u32)>>6 | (self.data()[4] as u32)<<2 | ((self.data()[5] & 15) as u32)<<10;
        let ftoa = (self.data()[2] & 15) as u32;
        (toa << 4) | (!ftoa & 15)
    }

    fn fast_electron_time(&self) -> usize {
        let spidr = (self.data()[0] as usize) | (self.data()[1] as usize)<<8;
        let toa = ((self.data()[3] >> 6) as usize) | (self.data()[4] as usize)<<2 | ((self.data()[5] & 15) as usize)<<10;
        spidr * 25 * 16384 + toa * 25
    }
    
    fn electron_time(&self) -> usize {
        let spidr = (self.data()[0] as usize) | (self.data()[1] as usize)<<8;
        
        let toa = ((self.data()[3] >> 6) as usize) | (self.data()[4] as usize)<<2 | ((self.data()[5] & 15) as usize)<<10;
        
        let ftoa = (self.data()[2] & 15) as usize;
        //let ftoa2 = (!self.data()[2] & 15) as usize;
        let ctoa = (toa << 4) | (!ftoa & 15);
        //let ctoa2 = (toa << 4) | ftoa2;
        
        spidr * 25 * 16384 + ctoa * 25 / 16
    }

    fn tdc_coarse(&self) -> u64 {
        ((self.data()[1] & 254) as u64)>>1 | ((self.data()[2]) as u64)<<7 | ((self.data()[3]) as u64)<<15 | ((self.data()[4]) as u64)<<23 | ((self.data()[5] & 15) as u64)<<31
    }
    
    fn tdc_fine(&self) -> u8 {
        (self.data()[0] & 224)>>5 | (self.data()[1] & 1)<<3
    }

    fn tdc_counter(&self) -> u16 {
        ((self.data()[5] & 240) as u16) >> 4 | (self.data()[6] as u16) << 4
    }

    fn tdc_type(&self) -> u8 {
        self.data()[7] & 15 
    }

    fn tdc_time(&self) -> usize {
        let coarse = ((self.data()[1] & 254) as usize)>>1 | ((self.data()[2]) as usize)<<7 | ((self.data()[3]) as usize)<<15 | ((self.data()[4]) as usize)<<23 | ((self.data()[5] & 15) as usize)<<31;
        let fine = ((self.data()[0] & 224) as usize >> 5) | ((self.data()[1] & 1) as usize) << 3;
        coarse * 1_000 / 320 + fine * 260 / 1_000
    }
    
    fn tdc_time_norm(&self) -> usize {
        let coarse = ((self.data()[1] & 254) as usize)>>1 | ((self.data()[2]) as usize)<<7 | ((self.data()[3]) as usize)<<15 | ((self.data()[4]) as usize)<<23 | ((self.data()[5] & 15) as usize)<<31;
        let fine = ((self.data()[0] & 224) as usize) >> 5 | ((self.data()[1] & 1) as usize) << 3;
        let time = coarse * 1_000 / 320 + fine * 260 / 1_000;
        time - (time / (26843545600)) * 26843545600
    }

    fn electron_reset_time() -> usize {
        //26843545600.0 * 1e-9
        26_843_545_600
    }
}

pub struct PacketEELS<'a> {
    pub chip_index: usize,
    pub data: &'a [u8; 8],
}

impl<'a> Packet for PacketEELS<'a> {
    fn ci(&self) -> usize {
        self.chip_index
    }
    fn data(&self) -> &[u8; 8] {
        self.data
    }
}

impl<'a> PacketEELS<'a> {
    pub const fn chip_array() -> (usize, usize) {
        (1025, 256)
    }
}


pub struct PacketDiffraction<'a> {
    pub chip_index: usize,
    pub data: &'a [u8; 8],
}

impl<'a> Packet for PacketDiffraction<'a> {
    fn ci(&self) -> usize {
        self.chip_index
    }
    fn data(&self) -> &[u8; 8] {
        self.data
    }
    fn x(&self) -> usize {
        let temp = (((self.data[6] & 224)>>4 | (self.data[7] & 15)<<4) | (((self.data[5] & 112)>>4)>>2)) as usize;
        match self.chip_index {
            0 => 255 - temp,
            1 => temp,
            2 => temp + 256,
            3 => 256 * 2 - 1 - temp,
            _ => panic!("More than four CI."),
        }
    }

    fn y(&self) -> usize {
        let temp = (   ( (self.data[5] & 128)>>5 | (self.data[6] & 31)<<3 ) | ( ((self.data[5] & 112)>>4) & 3 )   ) as usize;
        match self.chip_index {
            0 => temp,
            1 => 256 * 2 - 1 - temp,
            2 => 256 * 2 - 1 - temp,
            3 => temp,
            _ => panic!("More than four CI."),
        }
    }
}

impl<'a> PacketDiffraction<'a> {
    pub const fn chip_array() -> (usize, usize) {
        (512, 512)
    }
}

pub struct InversePacket {
    pub x: usize,
    pub y: usize,
    pub time: usize,
    pub id: usize,
}

use std::convert::TryInto;
use crate::tdclib::TdcType;

impl InversePacket {

    pub fn new_inverse_electron(x: usize, y: usize, time: usize) -> Self {
        
        let x = (!x & 255) | (x & 768);

        let my_inv_packet = InversePacket {
            x,
            y,
            time,
            id: 11
        };
        my_inv_packet
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
    
    pub fn create_tdc_array(&self) -> [u8; 16] {
        let (ct, ft) = self.tdc_time_to_ticks();
        let counter = 4095;
        let res = 0;
        let tdc_type: u8 = TdcType::TdcTwoRisingEdge.associate_value();

        let data0: u8 = ((res & 248) >> 3 | (ft & 7) << 5) as u8;
        let data1: u8 = ((ct & 127) << 1 | (ft & 8) >> 3) as u8;
        let data2: u8 = ((ct * 32_640) >> 7 ) as u8;
        let data3: u8 = ((ct * 8_355_840) >> 15) as u8;
        let data4: u8 = ((ct & 2_139_095_040) >> 23) as u8;
        let data5: u8 = ((counter & 15) << 4 | (ct & 32_212_254_720) >> 31) as u8;
        let data6: u8 = ((counter & 4080) >> 4) as u8;
        let data7: u8 = ((self.id & 15) << 4) as u8 | (tdc_type & 15);
        [84, 80, 88, 51, 0, 0, 8, 0, data0, data1, data2, data3, data4, data5, data6, data7]
    }


    pub fn test_func(&self) {
        let my_inv_packet = InversePacket::new_inverse_electron(128, 100, 3_111_005);

        let my_data = my_inv_packet.create_electron_array();
        let my_packet = PacketEELS {
            chip_index: my_data[4] as usize,
            data: &my_data[8..16].try_into().unwrap()
        };

        println!("{} and {} and {} and {} and {} and {}", my_packet.x(), my_packet.y(), my_packet.x_raw(), my_packet.electron_time(), !my_packet.ftoa() & 15, my_packet.tot());
    }


    pub fn time_to_ticks(&self) -> (usize, usize, usize) {
        let spidr_ticks = self.time / 409_600;
        let ctoa = self.time % 409_600;
        let toa_ticks = ctoa / 25;
        let ftoa_ticks = (ctoa % 25) * 16 / 25;
        (spidr_ticks, toa_ticks, ftoa_ticks)
    }
    
    pub fn tdc_time_to_ticks(&self) -> (usize, usize) {
        let coarseT = self.time / (1_000/320);
        let fine_time = self.time % (1_000 / 320);
        let fineT = fine_time * (1_000 / 260);
 
        (coarseT, fineT)



    }
}

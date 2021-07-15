//!`packetlib` is a collection of tools to facilitate manipulation of individual TP3 packets. Module is built
//!in around `Packet` struct.

const DEAD_PIXELS: usize = 5;

pub trait Packet {
    fn ci(&self) -> u8;
    fn data(&self) -> &[u8];
    fn x(&self) -> Option<usize> {
        let temp = ((((self.data()[6] & 224))>>4 | ((self.data()[7] & 15))<<4) | (((self.data()[5] & 112)>>4)>>2)) as usize;
        if temp<DEAD_PIXELS || temp>255-DEAD_PIXELS {
            return None
        }
        match self.ci() {
            0 => Some(255 - temp),
            1 => Some(256 * 4 - 1 - temp),
            2 => Some(256 * 3 - 1 - temp),
            3 => Some(256 * 2 - 1 - temp),
            _ => None,
        }
    }
    fn y(&self) -> Option<usize> {
        let y = (   ( ((self.data()[5] & 128))>>5 | ((self.data()[6] & 31))<<3 ) | ( (((self.data()[5] & 112)>>4)) & 3 )   ) as usize;
        Some(y)
    }

    fn id(&self) -> u8 {
        (self.data()[7] & 240) >> 4
    }

    fn spidr(&self) -> u16 {
        (self.data()[0] as u16) | (self.data()[1] as u16)<<8
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
    
    fn electron_time(&self) -> f64 {
        let spidr = (self.data()[0] as u16) | (self.data()[1] as u16)<<8;
        let toa = ((self.data()[3] & 192) as u32)>>6 | (self.data()[4] as u32)<<2 | ((self.data()[5] & 15) as u32)<<10;
        let ftoa = (self.data()[2] & 15) as u32;
        let ctoa = (toa << 4) | (!ftoa & 15);
        ((spidr as f64) * 25.0 * 16384.0 + (ctoa as f64) * 25.0 / 16.0) / 1e9
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

    fn tdc_time(&self) -> f64 {
        let coarse = ((self.data()[1] & 254) as u64)>>1 | ((self.data()[2]) as u64)<<7 | ((self.data()[3]) as u64)<<15 | ((self.data()[4]) as u64)<<23 | ((self.data()[5] & 15) as u64)<<31;
        let fine = (self.data()[0] & 224)>>5 | (self.data()[1] & 1)<<3;
        (coarse as f64) * (1.0/320.0e6) + (fine as f64) * 260.0e-12
    }
    
    fn tdc_time_norm(&self) -> f64 {
        let coarse = ((self.data()[1] & 254) as u64)>>1 | ((self.data()[2]) as u64)<<7 | ((self.data()[3]) as u64)<<15 | ((self.data()[4]) as u64)<<23 | ((self.data()[5] & 15) as u64)<<31;
        let fine = (self.data()[0] & 224)>>5 | (self.data()[1] & 1)<<3;
        let time = (coarse as f64) * (1.0/320e6) + (fine as f64) * 260e-12;
        time - (time / (26843545600.0 * 1e-9)).floor() * 26843545600.0 * 1e-9
    }

}

pub struct PacketEELS<'a> {
    pub chip_index: u8,
    pub data: &'a [u8],
}

impl<'a> Packet for PacketEELS<'a> {
    fn ci(&self) -> u8 {
        self.chip_index
    }
    fn data(&self) -> &[u8] {
        self.data
    }
}

impl<'a> PacketEELS<'a> {
    pub const fn chip_array() -> (usize, usize) {
        (1024, 256)
    }
}


pub struct PacketDiffraction<'a> {
    pub chip_index: u8,
    pub data: &'a [u8],
}

impl<'a> Packet for PacketDiffraction<'a> {
    fn ci(&self) -> u8 {
        self.chip_index
    }
    fn data(&self) -> &[u8] {
        self.data
    }
    fn x(&self) -> Option<usize> {
        let temp = ((((self.data[6] & 224))>>4 | ((self.data[7] & 15))<<4) | (((self.data[5] & 112)>>4)>>2)) as usize;
        if temp<DEAD_PIXELS || temp>255-DEAD_PIXELS {
            return None
        }
        match self.chip_index {
            0 => Some(255 - temp),
            1 => Some(temp),
            2 => Some(temp + 256),
            3 => Some(256 * 2 - 1 - temp),
            _ => None,
        }
    }

    fn y(&self) -> Option<usize> {
        let temp = (   ( ((self.data[5] & 128))>>5 | ((self.data[6] & 31))<<3 ) | ( (((self.data[5] & 112)>>4)) & 3 )   ) as usize;
        if temp<DEAD_PIXELS || temp>255-DEAD_PIXELS {
            return None
        }
        match self.chip_index {
            0 => Some(temp),
            1 => Some(256 * 2 - 1 - temp),
            2 => Some(256 * 2 - 1 - temp),
            3 => Some(temp),
            _ => None,
        }
    }
}

impl<'a> PacketDiffraction<'a> {
    pub const fn chip_array() -> (usize, usize) {
        (512, 512)
    }
}


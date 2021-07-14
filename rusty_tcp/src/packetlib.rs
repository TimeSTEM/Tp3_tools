//!`packetlib` is a collection of tools to facilitate manipulation of individual TP3 packets. Module is built
//!in around `Packet` struct.

const DEAD_PIXELS: usize = 5;

pub trait PacketTrait {
    fn ci(&self) -> u8;
    fn data(&self) -> &[u8];
    fn x(&self) -> Option<usize> {
        let temp = ((((self.data()[6] & 224))>>4 | ((self.data()[7] & 15))<<4) | (((self.data()[5] & 112)>>4)>>2)) as usize;
        if temp<DEAD_PIXELS || temp>255-DEAD_PIXELS {
            return None
        }
        match self.ci() {
            0 => Some(255 - temp),
            1 => Some(temp),
            2 => Some(temp + 255),
            3 => Some(256 * 2 - 1 - temp),
            _ => None,
        }
    }
    fn y(&self) -> Option<usize> {
        let temp = (   ( ((self.data()[5] & 128))>>5 | ((self.data()[6] & 31))<<3 ) | ( (((self.data()[5] & 112)>>4)) & 3 )   ) as usize;
        if temp<DEAD_PIXELS || temp>255-DEAD_PIXELS {
            return None
        }
        match self.ci() {
            0 => Some(temp),
            1 => Some(256 * 2 - 1 - temp),
            2 => Some(256 * 2 - 1 - temp),
            3 => Some(temp),
            _ => None,
        }
    }
    fn id(&self) -> u8 {
        (self.data()[7] & 240) >> 4
    }
    fn tdc_type(&self) -> u8 {
        self.data()[7] & 15 
    }

    fn tdc_time(&self) -> f64 {
        let coarse = ((self.data()[1] & 254) as u64)>>1 | ((self.data()[2]) as u64)<<7 | ((self.data()[3]) as u64)<<15 | ((self.data()[4]) as u64)<<23 | ((self.data()[5] & 15) as u64)<<31;
        let fine = (self.data()[0] & 224)>>5 | (self.data()[1] & 1)<<3;
        (coarse as f64) * (1.0/320.0e6) + (fine as f64) * 260.0e-12
    }

}

pub struct PacketTest<'a> {
    pub chip_index: u8,
    pub data: &'a [u8],
}

impl<'a> PacketTrait for PacketTest<'a> {
    fn ci(&self) -> u8 {
        self.chip_index
    }
    fn data(&self) -> &[u8] {
        self.data
    }
}





pub struct Packet<'a> {
    pub chip_index: u8,
    pub data: &'a [u8],
}

impl<'a> Packet<'a> {
    
    pub fn x2(&self) -> Option<usize> {
        let temp = ((((self.data[6] & 224))>>4 | ((self.data[7] & 15))<<4) | (((self.data[5] & 112)>>4)>>2)) as usize;
        if temp<DEAD_PIXELS || temp>255-DEAD_PIXELS {
            return None
        }
        match self.chip_index {
            0 => Some(255 - temp),
            1 => Some(256 * 4 - 1 - temp),
            2 => Some(256 * 3 - 1 - temp),
            3 => Some(256 * 2 - 1 - temp),
            _ => None,
        }
    }
    
    pub fn x(&self) -> Option<usize> {
        let temp = ((((self.data[6] & 224))>>4 | ((self.data[7] & 15))<<4) | (((self.data[5] & 112)>>4)>>2)) as usize;
        if temp<DEAD_PIXELS || temp>255-DEAD_PIXELS {
            return None
        }
        match self.chip_index {
            0 => Some(255 - temp),
            1 => Some(temp),
            2 => Some(temp + 255),
            3 => Some(256 * 2 - 1 - temp),
            _ => None,
        }
    }
    
    pub fn x_unmod(&self) -> usize {
        !((((self.data[6] & 224))>>4 | ((self.data[7] & 15))<<4) | (((self.data[5] & 112)>>4)>>2)) as usize
    }
    
    pub fn y2(&self) -> Option<usize> {
        let y = (   ( ((self.data[5] & 128))>>5 | ((self.data[6] & 31))<<3 ) | ( (((self.data[5] & 112)>>4)) & 3 )   ) as usize;
        Some(y)
    }
    
    pub fn y(&self) -> Option<usize> {
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
        (coarse as f64) * (1.0/320.0e6) + (fine as f64) * 260.0e-12
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
}

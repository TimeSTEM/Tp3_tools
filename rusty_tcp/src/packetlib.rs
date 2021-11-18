//!`packetlib` is a collection of tools to facilitate manipulation of individual TP3 packets. Module is built
//!in around `Packet` struct.

pub trait Packet {
    fn ci(&self) -> usize;
    fn data(&self) -> &[u8];
    fn x(&self) -> usize {
        
        //let temp = (((self.data()[6] & 224)>>4 | (self.data()[7] & 15)<<4) | (((self.data()[5] & 112)>>4)>>2)) as usize;
        let temp = ((self.data()[6] & 224)>>4 | (self.data()[7] << 4) | ((self.data()[5] & 112) >> 6)) as usize;

        match self.ci() {
            0 => 255 - temp,
            1 => 256 * 4 - 1 - temp,
            2 => 256 * 3 - 1 - temp,
            3 => 256 * 2 - 1 - temp,
            _ => panic!("More than four CI."),
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
    
    fn electron_time(&self) -> usize {
        let spidr = (self.data()[0] as usize) | (self.data()[1] as usize)<<8;
        
        //let toa = ((self.data()[3] & 192) as usize)>>6 | (self.data()[4] as usize)<<2 | ((self.data()[5] & 15) as usize)<<10;
        //let toa = ((self.data()[3] >> 6) as usize) | (self.data()[4] as usize)<<2 | ((self.data()[5] << 4) as usize)<<6;
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

    fn electron_reset_time() -> f64 {
        26843545600.0 * 1e-9
    }


}

pub struct PacketEELS<'a> {
    pub chip_index: usize,
    pub data: &'a [u8],
}

impl<'a> Packet for PacketEELS<'a> {
    fn ci(&self) -> usize {
        self.chip_index
    }
    fn data(&self) -> &[u8] {
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
    pub data: &'a [u8],
}

impl<'a> Packet for PacketDiffraction<'a> {
    fn ci(&self) -> usize {
        self.chip_index
    }
    fn data(&self) -> &[u8] {
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


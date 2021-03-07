//!`auxiliar` is a collection of tools to set acquisition conditions.

///Describes how to run the program. This is a very general enumeration and can be used for setting
///your program in debug mode.
pub enum RunningMode {
    DebugStem7482,
    Tp3,
}

///Configures the detector for acquisition. Each new measurement must send 28 bytes
///containing instructions.
pub struct BytesConfig {
    pub data: [u8; 28],
}

impl BytesConfig {
    ///Set binning mode for 1x4 detector. `\x00` for unbinned and `\x01` for binned. Panics otherwise. Byte[0].
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

    ///Set bytedepth. `\x00` for 1, `\x01` for 2 and `\x02` for 4. Panics otherwise. Byte[1].
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

    ///Sums all arriving data. `\x00` for False, `\x01` for True. Panics otherwise. Byte[2].
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

    ///Acquisition Mode. `\x00` for normal, `\x01` for spectral image and `\x02` for time-resolved. Panics otherwise. Byte[2..4].
    pub fn mode(&self) -> u8 {
        match self.data[3] {
            0 => {
                println!("Mode is Focus/Cumul.");
            },
            1 => {
                println!("Mode is SpimTP.");
            },
            2 => {
                println!("Time width is not zero. Entering in time resolved mode.");
            },
            _ => panic!("Spim config must be 0 | 1."),
        };
        self.data[3]
    }

    ///X spim size. Must be sent with 2 bytes in big-endian mode. Byte[4..6]
    pub fn xspim_size(&self) -> usize {
        let x = (self.data[4] as usize)<<8 | (self.data[5] as usize);
        println!("X Spim size is: {}.", x);
        x
    }
    
    ///Y spim size. Must be sent with 2 bytes in big-endian mode. Byte[6..8]
    pub fn yspim_size(&self) -> usize {
        let y = (self.data[6] as usize)<<8 | (self.data[7] as usize);
        println!("Y Spim size is: {}.", y);
        y
    }
    
    ///X scan size. Must be sent with 2 bytes in big-endian mode. Byte[8..10]
    pub fn xscan_size(&self) -> usize {
        let x = (self.data[8] as usize)<<8 | (self.data[9] as usize);
        println!("X Scan size is: {}.", x);
        x
    }
    
    ///Y scan size. Must be sent with 2 bytes in big-endian mode. Byte[10..12]
    pub fn yscan_size(&self) -> usize {
        let y = (self.data[10] as usize)<<8 | (self.data[11] as usize);
        println!("Y Scan size is: {}.", y);
        y
    }
    
    ///Time delay. Must be sent with 8 bytes in big-endian mode. Similar to double in C.
    ///Byte[12..20]
    pub fn time_delay(&self) -> f64 {
        let mut array: [u8; 8] = [0; 8];
        for (i, val) in array.iter_mut().enumerate() {
            *val = self.data[i+12]
        }
        let val = f64::from_be_bytes(array);
        println!("Time delay is (ns): {}.", val);
        val / 1.0e9
    }
    
    ///Time width. Must be sent with 8 bytes in big-endian mode. Similar to double in C. Byte[20..28].
    pub fn time_width(&self) -> f64 {
        let mut array: [u8; 8] = [0; 8];
        for (i, val) in array.iter_mut().enumerate() {
            *val = self.data[i+20]
        }
        let val = f64::from_be_bytes(array);
        println!("Time width is (ns): {}.", val);
        val / 1.0e9
    }
    
    ///Convenience method. Returns the ratio between scan and spim size in X.
    pub fn spimoverscanx(&self) -> usize {
        let xspim = self.xspim_size();
        let xscan = self.xscan_size();
        let var = xscan / xspim;
        match var {
            0 => {
                println!("Xratio is: 1.");
                1
            },
            _ => {
                println!("Xratio is: {}.", var);
                var
            },
        }
    }
    
    ///Convenience method. Returns the ratio between scan and spim size in Y.
    pub fn spimoverscany(&self) -> usize {
        let yspim = self.yspim_size();
        let yscan = self.yscan_size();
        let var = yscan / yspim;
        match var {
            0 => {
                println!("Yratio is: 1.");
                1
            },
            _ => {
                println!("Yratio is: {}.", var);
                var
            },
        }
    }

    pub fn create_settings(&self) -> Settings {
        Settings {
            bin: self.bin(),
            bytedepth: self.bytedepth(),
            cumul: self.cumul(),
            xspim_size: self.xspim_size(),
            yspim_size: self.yspim_size(),
            xscan_size: self.xscan_size(),
            yscan_size: self.yscan_size(),
            time_delay: self.time_delay(),
            time_width: self.time_width(),
            spimoverscanx: self.spimoverscanx(),
            spimoverscany: self.spimoverscany(),
        }
    }
}

pub struct Settings {
    pub bin: bool,
    pub bytedepth: usize,
    pub cumul: bool,
    pub xspim_size: usize,
    pub yspim_size: usize,
    pub xscan_size: usize,
    pub yscan_size: usize,
    pub time_delay: f64,
    pub time_width: f64,
    pub spimoverscanx: usize,
    pub spimoverscany: usize,
}

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
                println!("Time width is not zero. Entering in time resolved mode.");
            },
            _ => panic!("Spim config must be 0 | 1."),
        };
        self.data[3]
    }

    pub fn xspim_size(&self) -> usize {
        let x = (self.data[4] as usize)<<8 | (self.data[5] as usize);
        println!("X Spim size is: {}.", x);
        x
    }
    
    pub fn yspim_size(&self) -> usize {
        let y = (self.data[6] as usize)<<8 | (self.data[7] as usize);
        println!("Y Spim size is: {}.", y);
        y
    }
    
    pub fn xscan_size(&self) -> usize {
        let x = (self.data[8] as usize)<<8 | (self.data[9] as usize);
        println!("X Scan size is: {}.", x);
        x
    }
    
    pub fn yscan_size(&self) -> usize {
        let y = (self.data[10] as usize)<<8 | (self.data[11] as usize);
        println!("Y Scan size is: {}.", y);
        y
    }

    
    pub fn spimoverscanx(&self) -> usize {
        let xspim = (self.data[4] as usize)<<8 | (self.data[5] as usize);
        let xscan = (self.data[8] as usize)<<8 | (self.data[9] as usize);
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
    
    pub fn spimoverscany(&self) -> usize {
        let yspim = (self.data[6] as usize)<<8 | (self.data[7] as usize);
        let yscan = (self.data[10] as usize)<<8 | (self.data[11] as usize);
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

    pub fn time_delay(&self) -> f64 {
        let mut array: [u8; 8] = [0; 8];
        for (i, val) in array.iter_mut().enumerate() {
            *val = self.data[i+12]
        }
        let val = f64::from_be_bytes(array);
        println!("Time delay is (ns): {}.", val);
        val / 1.0e9
    }
    
    pub fn time_width(&self) -> f64 {
        let mut array: [u8; 8] = [0; 8];
        for (i, val) in array.iter_mut().enumerate() {
            *val = self.data[i+20]
        }
        let val = f64::from_be_bytes(array);
        println!("Time width is (ns): {}.", val);
        val / 1.0e9
    }
}

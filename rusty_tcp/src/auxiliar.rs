//!`auxiliar` is a collection of tools to set acquisition conditions.

const CONFIG_SIZE: usize = 30;

#[derive(Debug)]
pub enum BytesConfigError {
    Bin,
    ByteDepth,
    Cumul,
    Mode,
    XSize,
    YSize,
    NbSockets,
}

///Configures the detector for acquisition. Each new measurement must send 28 bytes
///containing instructions.
struct BytesConfig {
    pub data: [u8; CONFIG_SIZE],
}

impl BytesConfig {
    ///Set binning mode for 1x4 detector. `\x00` for unbinned and `\x01` for binned. Panics otherwise. Byte[0].
    fn bin(&self) -> Result<bool, BytesConfigError> {
        match self.data[0] {
            0 => {
                println!("Bin is False.");
                Ok(false)
            },
            1 => {
                println!("Bin is True.");
                Ok(true)
            },
            _ => Err(BytesConfigError::Bin),
        }
    }

    ///Set bytedepth. `\x00` for 1, `\x01` for 2 and `\x02` for 4. Panics otherwise. Byte[1].
    fn bytedepth(&self) -> Result<usize, BytesConfigError> {
        match self.data[1] {
            0 => {
                println!("Bitdepth is 8.");
                Ok(1)
            },
            1 => {
                println!("Bitdepth is 16.");
                Ok(2)
            },
            2 => {
                println!("Bitdepth is 32.");
                Ok(4)
            },
            3 => {
                println!("Bitdepth is 32.");
                Ok(8)
            },
            _ => Err(BytesConfigError::ByteDepth),
        }
    }

    ///Sums all arriving data. `\x00` for False, `\x01` for True. Panics otherwise. Byte[2].
    fn cumul(&self) -> Result<bool, BytesConfigError> {
        match self.data[2] {
            0 => {
                println!("Cumulation mode is OFF.");
                Ok(false)
            },
            1 => {
                println!("Cumulation mode is ON.");
                Ok(true)
            },
            _ => Err(BytesConfigError::Cumul),
        }
    }

    ///Acquisition Mode. `\x00` for normal, `\x01` for spectral image and `\x02` for time-resolved. Panics otherwise. Byte[2..4].
    fn mode(&self) -> Result<u8, BytesConfigError> {
        match self.data[3] {
            0 => {
                println!("Mode is Focus/Cumul.");
                Ok(self.data[3])
            },
            1 => {
                println!("Entering in time resolved mode (Focus/Cumul).");
                Ok(self.data[3])
            },
            2 => {
                println!("Entering in Spectral Image (SpimTP).");
                Ok(self.data[3])
            },
            3 => {
                println!("Entering in time resolved mode (SpimTP).");
                Ok(self.data[3])
            },
            4 => {
                println!("Entering in Spectral Image [TDC Mode] (SpimTP).");
                Ok(self.data[3])
            },
            5 => {
                println!("Entering in Spectral Image [Save Locally] (SpimTP).");
                Ok(self.data[3])
            },
            6 => {
                println!("Entering in Chrono Mode.");
                Ok(self.data[3])
            },
            _ => Err(BytesConfigError::Mode),
        }
    }


    ///X spim size. Must be sent with 2 bytes in big-endian mode. Byte[4..6]
    fn xspim_size(&self) -> usize {
        let x = (self.data[4] as usize)<<8 | (self.data[5] as usize);
        println!("X Spim size is: {}.", x);
        x
    }
    
    ///Y spim size. Must be sent with 2 bytes in big-endian mode. Byte[6..8]
    fn yspim_size(&self) -> usize {
        let y = (self.data[6] as usize)<<8 | (self.data[7] as usize);
        println!("Y Spim size is: {}.", y);
        y
    }
    
    ///X scan size. Must be sent with 2 bytes in big-endian mode. Byte[8..10]
    fn xscan_size(&self) -> usize {
        let x = (self.data[8] as usize)<<8 | (self.data[9] as usize);
        println!("X Scan size is: {}.", x);
        x
    }
    
    ///Y scan size. Must be sent with 2 bytes in big-endian mode. Byte[10..12]
    fn yscan_size(&self) -> usize {
        let y = (self.data[10] as usize)<<8 | (self.data[11] as usize);
        println!("Y Scan size is: {}.", y);
        y
    }
    
    ///Time delay. Must be sent with 8 bytes in big-endian mode. Similar to double in C.
    ///Byte[12..20]
    fn time_delay(&self) -> f64 {
        let mut array: [u8; 8] = [0; 8];
        for (i, val) in array.iter_mut().enumerate() {
            *val = self.data[i+12]
        }
        let val = f64::from_be_bytes(array);
        println!("Time delay is (ns): {}.", val);
        val / 1.0e9
    }
    
    ///Time width. Must be sent with 8 bytes in big-endian mode. Similar to double in C. Byte[20..28].
    fn time_width(&self) -> f64 {
        let mut array: [u8; 8] = [0; 8];
        for (i, val) in array.iter_mut().enumerate() {
            *val = self.data[i+20]
        }
        let val = f64::from_be_bytes(array);
        println!("Time width is (ns): {}.", val);
        val / 1.0e9
    }
    
    ///Counter for general use. Can be used to finish accumulation automatically or to create a
    ///Chrono measurement for example. Must be sent with 2 bytes in big-endian mode. Byte[28..30]
    fn counter(&self) -> usize {
        let counter = (self.data[28] as usize)<<8 | (self.data[29] as usize);
        println!("Counter value is: {}.", counter);
        counter
    }
    
    ///Convenience method. Returns the ratio between scan and spim size in X.
    fn spimoverscanx(&self) -> Result<usize, BytesConfigError> {
        let xspim = self.xspim_size();
        let xscan = self.xscan_size();
        if xspim == 0 {return Err(BytesConfigError::XSize);}
        let var = xscan / xspim;
        match var {
            0 => {
                println!("Xratio is: 1.");
                Ok(1)
            },
            _ => {
                println!("Xratio is: {}.", var);
                Ok(var)
            },
        }
    }
    
    ///Convenience method. Returns the ratio between scan and spim size in Y.
    fn spimoverscany(&self) -> Result<usize, BytesConfigError> {
        let yspim = self.yspim_size();
        let yscan = self.yscan_size();
        if yspim == 0 {return Err(BytesConfigError::YSize);}
        let var = yscan / yspim;
        match var {
            0 => {
                println!("Yratio is: 1.");
                Ok(1)
            },
            _ => {
                println!("Yratio is: {}.", var);
                Ok(var)
            },
        }
    }

    fn nsockets(&self) -> Result<usize, BytesConfigError> {
        let number_sockets = (self.data[28] as usize)<<8 | (self.data[29] as usize);
        if number_sockets == 1 || number_sockets % 4 == 0{
            println!("Number of sockets is: {}", number_sockets);
            Ok(number_sockets)
        } else {Err(BytesConfigError::NbSockets)}
    }

    ///Create Settings struct from BytesConfig
    fn create_settings(&self) -> Result<Settings, BytesConfigError> {
        let my_set = Settings {
            bin: self.bin()?,
            bytedepth: self.bytedepth()?,
            cumul: self.cumul()?,
            mode: self.mode()?,
            xspim_size: self.xspim_size(),
            yspim_size: self.yspim_size(),
            xscan_size: self.xscan_size(),
            yscan_size: self.yscan_size(),
            time_delay: self.time_delay(),
            time_width: self.time_width(),
            counter: self.counter(),
            spimoverscanx: self.spimoverscanx()?,
            spimoverscany: self.spimoverscany()?,
            number_sockets: self.nsockets()?,
        };
        Ok(my_set)
    }

}

use std::net::{TcpListener, TcpStream, SocketAddr};
use std::io::Read;

///Settings contains all relevant parameters for a given acquistion
pub struct Settings {
    pub bin: bool,
    pub bytedepth: usize,
    pub cumul: bool,
    pub mode: u8,
    pub xspim_size: usize,
    pub yspim_size: usize,
    pub xscan_size: usize,
    pub yscan_size: usize,
    pub time_delay: f64,
    pub time_width: f64,
    pub counter: usize,
    pub spimoverscanx: usize,
    pub spimoverscany: usize,
    pub number_sockets: usize,
}

impl Settings {

    ///Create Settings structure reading from a TCP.
    pub fn create_settings(host_computer: [u8; 4], port: u16) -> Result<(Settings, Box<dyn Read + Send>, Vec<TcpStream>), BytesConfigError> {
    
        let mut sock_vec: Vec<TcpStream> = Vec::new();
        
        let addrs = [
            SocketAddr::from((host_computer, port)),
            SocketAddr::from(([127, 0, 0, 1], port)),
        ];
        
        let pack_listener = TcpListener::bind("127.0.0.1:8098").expect("Could not bind to TP3.");
        let ns_listener = TcpListener::bind(&addrs[..]).expect("Could not bind to NS.");
        println!("Packet Tcp socket connected at: {:?}", pack_listener);
        println!("Nionswift Tcp socket connected at: {:?}", ns_listener);

        let debug: bool = match ns_listener.local_addr() {
            Ok(val) if val == addrs[1] => true,
            _ => false,
        };


        let (mut ns_sock, ns_addr) = ns_listener.accept().expect("Could not connect to Nionswift.");
        println!("Nionswift connected at {:?} and {:?}.", ns_addr, ns_sock);
        let (pack_sock, packet_addr) = pack_listener.accept().expect("Could not connect to TP3.");
        println!("Localhost TP3 detected at {:?} and {:?}.", packet_addr, pack_sock);
 
        let mut cam_settings = [0 as u8; CONFIG_SIZE];
        
        let my_config = {
            match ns_sock.read(&mut cam_settings){
                Ok(size) => {
                    println!("Received {} bytes from NS.", size);
                    BytesConfig{data: cam_settings}
                },
                Err(_) => panic!("Could not read cam initial settings."),
            }
        };
        
        let my_settings = my_config.create_settings()?;
        sock_vec.push(ns_sock);
        
        println!("Connecting auxiliar sockets...");
        for i in 0..my_settings.number_sockets-1 {
            let (ns_sock, ns_addr) = ns_listener.accept().expect("Could not connect to Nionswift.");
            println!("Nionswift ({}) connected at {:?} and {:?}.", i, ns_addr, ns_sock);
            sock_vec.push(ns_sock);
        }

        println!("Received settings is {:?}. Mode is {}.", cam_settings, my_settings.mode);
        Ok((my_settings, Box::new(pack_sock), sock_vec))
    }

    pub fn create_debug_settings() -> Settings {
        Settings {
            bin: false,
            bytedepth: 4,
            cumul: false,
            mode: 02,
            xspim_size: 512,
            yspim_size: 512,
            xscan_size: 512,
            yscan_size: 512,
            time_delay: 0.0,
            time_width: 1000.0,
            counter: 128,
            spimoverscanx: 1,
            spimoverscany: 1,
            number_sockets: 1,
        }
    }

}

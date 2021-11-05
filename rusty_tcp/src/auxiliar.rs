//!`auxiliar` is a collection of tools to set acquisition conditions.
use crate::errorlib::Tp3ErrorKind;
use std::net::{TcpListener, TcpStream, SocketAddr};
use std::io::{Read, Write};
use std::{fs::{File, OpenOptions, create_dir_all}, path::Path};
use std::time::Duration;

const CONFIG_SIZE: usize = 20;

///Configures the detector for acquisition. Each new measurement must send 20 bytes
///containing instructions.
struct BytesConfig {
    pub data: [u8; CONFIG_SIZE],
}

impl BytesConfig {
    ///Set binning mode for 1x4 detector. `\x00` for unbinned and `\x01` for binned. Panics otherwise. Byte[0].
    fn bin(&self) -> Result<bool, Tp3ErrorKind> {
        match self.data[0] {
            0 => {
                println!("Bin is False.");
                Ok(false)
            },
            1 => {
                println!("Bin is True.");
                Ok(true)
            },
            _ => Err(Tp3ErrorKind::SetBin),
        }
    }

    ///Set bytedepth. `\x00` for 1, `\x01` for 2 and `\x02` for 4. Panics otherwise. Byte[1].
    fn bytedepth(&self) -> Result<usize, Tp3ErrorKind> {
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
            _ => Err(Tp3ErrorKind::SetByteDepth),
        }
    }

    ///Sums all arriving data. `\x00` for False, `\x01` for True. Panics otherwise. Byte[2].
    fn cumul(&self) -> Result<bool, Tp3ErrorKind> {
        match self.data[2] {
            0 => {
                println!("Cumulation mode is OFF.");
                Ok(false)
            },
            1 => {
                println!("Cumulation mode is ON.");
                Ok(true)
            },
            _ => Err(Tp3ErrorKind::SetCumul),
        }
    }

    ///Acquisition Mode. `\x00` for normal, `\x01` for spectral image and `\x02` for time-resolved. Panics otherwise. Byte[2..4].
    fn mode(&self) -> Result<u8, Tp3ErrorKind> {
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
            _ => Err(Tp3ErrorKind::SetMode),
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
    
    ///Time delay. Must be sent with 2 bytes in big-endian mode. Byte[12..14]
    fn time_delay(&self) -> usize {
        let td = (self.data[12] as usize)<<8 | (self.data[13] as usize);
        println!("Time delay is (ns): {}.", td);
        td
    }
    
    ///Time width. Must be sent with 2 bytes in big-endian mode. Byte[14..16].
    fn time_width(&self) -> usize {
        let tw = (self.data[14] as usize)<<8 | (self.data[15] as usize);
        println!("Time delay is (ns): {}.", tw);
        tw
    }
    
    ///Counter for general use. Can be used to finish accumulation automatically or to create a
    ///Chrono measurement for example. Must be sent with 2 bytes in big-endian mode. Byte[16..18]
    fn counter(&self) -> usize {
        let counter = (self.data[16] as usize)<<8 | (self.data[17] as usize);
        println!("Counter value is: {}.", counter);
        counter
    }
    
    
    ///Number of sockets. Must be sent with 2 bytes in big-endian mode. Byte[18..20].
    fn nsockets(&self) -> Result<usize, Tp3ErrorKind> {
        let number_sockets = (self.data[18] as usize)<<8 | (self.data[19] as usize);
        if number_sockets == 1 || number_sockets % 4 == 0{
            println!("Number of sockets is: {}", number_sockets);
            Ok(number_sockets)
        } else {Err(Tp3ErrorKind::SetNbSockets)}
    }
    
    ///Convenience method. Returns the ratio between scan and spim size in X.
    fn spimoverscanx(&self) -> Result<usize, Tp3ErrorKind> {
        let xspim = self.xspim_size();
        let xscan = self.xscan_size();
        if xspim == 0 {return Err(Tp3ErrorKind::SetXSize);}
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
    fn spimoverscany(&self) -> Result<usize, Tp3ErrorKind> {
        let yspim = self.yspim_size();
        let yscan = self.yscan_size();
        if yspim == 0 {return Err(Tp3ErrorKind::SetYSize);}
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


    ///Create Settings struct from BytesConfig
    fn create_settings(&self) -> Result<Settings, Tp3ErrorKind> {
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


///Settings contains all relevant parameters for a given acquistion
#[derive(Copy, Clone, Debug)]
pub struct Settings {
    pub bin: bool,
    pub bytedepth: usize,
    pub cumul: bool,
    pub mode: u8,
    pub xspim_size: usize,
    pub yspim_size: usize,
    pub xscan_size: usize,
    pub yscan_size: usize,
    pub time_delay: usize,
    pub time_width: usize,
    pub counter: usize,
    pub spimoverscanx: usize,
    pub spimoverscany: usize,
    pub number_sockets: usize,
}

impl Settings {

    ///Create Settings structure reading from a TCP.
    pub fn create_settings(host_computer: [u8; 4], port: u16) -> Result<(Settings, Box<dyn Read + Send>, Box<dyn Write + Send>), Tp3ErrorKind> {
    
        let mut _sock_vec: Vec<TcpStream> = Vec::new();
        
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
        //sock_vec.push(ns_sock);
        println!("Connecting auxiliar sockets...***VEC IS NOT CREATED***");
        for i in 0..my_settings.number_sockets-1 {
            let (ns_sock, ns_addr) = ns_listener.accept().expect("Could not connect to Nionswift.");
            println!("Nionswift ({}) connected at {:?} and {:?}.", i, ns_addr, ns_sock);
            //sock_vec.push(ns_sock);
        }
        println!("Received settings is {:?}. Mode is {}.", cam_settings, my_settings.mode);

        match debug {
            false => {
                let (pack_sock, packet_addr) = pack_listener.accept().expect("Could not connect to TP3.");
                println!("Localhost TP3 detected at {:?} and {:?}.", packet_addr, pack_sock);
                pack_sock.set_read_timeout(Some(Duration::from_millis(100))).expect("Could not set read timeout.");
                Ok((my_settings, Box::new(pack_sock), Box::new(ns_sock)))
            },
            true => {
                let file = match File::open("bin/Data/raw000000.tpx3") {
                    Ok(file) => file,
                    Err(_) => return Err(Tp3ErrorKind::SetNoReadFile),
                };
                println!("Debug mode. Will one file a single time.");
                Ok((my_settings, Box::new(file), Box::new(ns_sock)))
            },
        }

    }

    fn create_spec_debug_settings() -> Settings  {
        Settings {
            bin: false,
            bytedepth: 4,
            cumul: false,
            mode: 00,
            xspim_size: 512,
            yspim_size: 512,
            xscan_size: 512,
            yscan_size: 512,
            time_delay: 0,
            time_width: 1000,
            counter: 128,
            spimoverscanx: 1,
            spimoverscany: 1,
            number_sockets: 1,
        }
    }
    
    fn create_spim_debug_settings() -> Settings  {
        Settings {
            bin: true,
            bytedepth: 4,
            cumul: false,
            mode: 02,
            xspim_size: 512,
            yspim_size: 512,
            xscan_size: 512,
            yscan_size: 512,
            time_delay: 0,
            time_width: 1000,
            counter: 128,
            spimoverscanx: 1,
            spimoverscany: 1,
            number_sockets: 1,
        }
    }

    
    pub fn create_debug_settings(spim: bool) -> Result<(Settings, Box<dyn Read + Send>, Box<dyn Write + Send>), Tp3ErrorKind> {
    
        let my_settings = match spim {
            true => Settings::create_spim_debug_settings(),
            false => Settings::create_spec_debug_settings(),
        };
        
        println!("Received settings is {:?}. Mode is {}.", my_settings, my_settings.mode);

        let in_file = match File::open("bin/Data/raw000000.tpx3") {
            Ok(file) => file,
            Err(_) => return Err(Tp3ErrorKind::SetNoReadFile),
        };
        
        let out_dir = Path::new("Microscope/Output/");
        create_dir_all(&out_dir).unwrap();
        let out_file_name = "out.txt";
        let file_path = out_dir.join(&out_file_name);
        let out_file = OpenOptions::new().write(true).create(true).truncate(true).open(file_path).unwrap();

        println!("Spectra Debug mode. Will one file a single time.");
        Ok((my_settings, Box::new(in_file), Box::new(out_file)))
    }

}


pub mod simple_log {
    use chrono::prelude::*;
    use std::{fs::{File, OpenOptions, create_dir_all}, path::Path};
    use std::io::Write;
    use std::io;
    use crate::errorlib::Tp3ErrorKind;

    pub fn start() -> io::Result<File> {
        let dir = Path::new("Microscope/Log/");
        create_dir_all(&dir)?;
        let date = Local::now().format("%Y-%m-%d").to_string() + ".txt";
        let file_path = dir.join(&date);
        let mut file = OpenOptions::new().write(true).truncate(false).create(true).append(true).open(file_path)?;
        let date = Local::now().to_string();
        file.write(date.as_bytes())?;
        file.write(b" - Starting new loop\n")?;
        Ok(file)
    }

    pub fn ok(file: &mut File, mode: u8) -> io::Result<()> {
        let date = Local::now().to_string();
        file.write(date.as_bytes())?;
        file.write(b" - OK ")?;
        let mode = format!("{:?}", mode);
        file.write(mode.as_bytes())?;
        file.write(b"\n")?;
        Ok(())
    }

    pub fn error(file: &mut File, error: Tp3ErrorKind) -> io::Result<()> {
        let date = Local::now().to_string();
        file.write(date.as_bytes())?;
        file.write(b" - ERROR ")?;
        let error = format!("{:?}", error);
        file.write(error.as_bytes())?;
        file.write(b"\n")?;
        Ok(())
    }
}

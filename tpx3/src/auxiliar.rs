//!`auxiliar` is a collection of tools to set acquisition conditions.
use crate::errorlib::Tp3ErrorKind;
use std::net::{TcpListener, TcpStream, SocketAddr};
use crate::auxiliar::misc::TimepixRead;
use crate::clusterlib::cluster::ClusterCorrection;
use std::io::{Read, Write, BufWriter};
use std::fs::File;
use crate::constlib::*;
use crate::auxiliar::value_types::*;
use std::fs::OpenOptions;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;



pub mod aux_func {
    pub fn from_bytes<T>(v: &[u8]) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const T,
                v.len() * std::mem::size_of::<u8>() / std::mem::size_of::<T>())
        }
    }

    ///This is little endian
    pub fn as_bytes<T>(v: &[T]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u8,
                v.len() * std::mem::size_of::<T>())
        }
    }

    ///This is little endian
    pub fn as_bytes_mut<T>(v: &mut [T]) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(
                v.as_ptr() as *mut u8,
                v.len() * std::mem::size_of::<T>())
        }
    }
    
    pub fn as_int(v: &[u8]) -> &[u32] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u32,
                v.len() * std::mem::size_of::<u8>() / std::mem::size_of::<u32>())
        }
    }

}

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
    fn bytedepth(&self) -> Result<POSITION, Tp3ErrorKind> {
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
            4 => {
                println!("Bitdepth is 64.");
                Ok(8)
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
        println!("Mode is: {}", self.data[3]);
        Ok(self.data[3])
    }

    ///X spim size. Must be sent with 2 bytes in big-endian mode. Byte[4..6]
    fn xspim_size(&self) -> POSITION {
        let x: u16 = aux_func::from_bytes(&self.data[4..6])[0];
        println!("X Spim size is: {}.", x);
        x as POSITION
    }
    
    ///Y spim size. Must be sent with 2 bytes in big-endian mode. Byte[6..8]
    fn yspim_size(&self) -> POSITION {
        let y: u16 = aux_func::from_bytes(&self.data[6..8])[0];
        println!("Y Spim size is: {}.", y);
        y as POSITION
    }
    
    ///X scan size. Must be sent with 2 bytes in big-endian mode. Byte[8..10]
    fn xscan_size(&self) -> POSITION {
        let x: u16 = aux_func::from_bytes(&self.data[8..10])[0];
        println!("X Scan size is: {}.", x);
        x as POSITION
    }
    
    ///Y scan size. Must be sent with 2 bytes in big-endian mode. Byte[10..12]
    fn yscan_size(&self) -> POSITION {
        let y: u16 = aux_func::from_bytes(&self.data[10..12])[0];
        println!("Y Scan size is: {}.", y);
        y as POSITION
    }
    
    ///Pixel time. Must be sent with 2 bytes in big-endian mode. Byte[12..15]
    fn pixel_time(&self) -> POSITION {
        //let pt = (self.data[12] as POSITION)<<8 | (self.data[13] as POSITION);
        let pt: u32 = aux_func::from_bytes(&self.data[12..16])[0];
        println!("Pixel time is (units of 1.5625 ns): {}.", pt);
        pt as POSITION
    }
    
    ///Time delay. Must be sent with 2 bytes in big-endian mode. Byte[16..17]
    fn time_delay(&self) -> TIME {
        let td: u16 = aux_func::from_bytes(&self.data[16..18])[0];
        println!("Time delay is (units of 1.5625 ns): {}.", td);
        td as TIME
    }
    
    ///Time width. Must be sent with 2 bytes in big-endian mode. Byte[18..19].
    fn time_width(&self) -> TIME {
        let tw: u16 = aux_func::from_bytes(&self.data[18..20])[0];
        println!("Time width is (units of 1.5625 ns): {}.", tw);
        tw as TIME
    }

    ///Should we also save-locally the data. Byte[20]
    fn save_locally(&self) -> Result<bool, Tp3ErrorKind> {
        let save_locally = self.data[20] == 1;
        println!("Save locally is activated: {}.", save_locally);
        Ok(save_locally)
    }
    
    ///Should we also save-locally the data. Byte[21..24]
    fn sup_val0(&self) -> f32 {
        let sup = aux_func::from_bytes(&self.data[21..25])[0];
        println!("Supplementary value 0: {}.", sup);
        sup
    }
    
    ///Should we also save-locally the data. Byte[25..28]
    fn sup_val1(&self) -> f32 {
        let sup = aux_func::from_bytes(&self.data[25..29])[0];
        println!("Supplementary value 0: {}.", sup);
        sup
    }
    
    ///Convenience method. Returns the ratio between scan and spim size in X.
    fn spimoverscanx(&self) -> Result<POSITION, Tp3ErrorKind> {
        let xspim = (self.data[4] as POSITION)<<8 | (self.data[5] as POSITION);
        let xscan = (self.data[8] as POSITION)<<8 | (self.data[9] as POSITION);
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
    fn spimoverscany(&self) -> Result<POSITION, Tp3ErrorKind> {
        let yspim = (self.data[6] as POSITION)<<8 | (self.data[7] as POSITION);
        let yscan = (self.data[10] as POSITION)<<8 | (self.data[11] as POSITION);
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
            pixel_time: self.pixel_time(),
            time_delay: self.time_delay(),
            time_width: self.time_width(),
            spimoverscanx: self.spimoverscanx()?,
            spimoverscany: self.spimoverscany()?,
            save_locally: self.save_locally()?,
            sup0: self.sup_val0(),
            sup1: self.sup_val1(),
        };
        Ok(my_set)
    }
}


struct DebugIO {}
impl Write for DebugIO {
    fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
        Ok(0)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl TimepixRead for DebugIO {}
impl Read for DebugIO {
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
}

///`Settings` contains all relevant parameters for a given acquistion
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub bin: bool,
    pub bytedepth: POSITION,
    pub cumul: bool,
    pub mode: u8,
    pub xspim_size: POSITION,
    pub yspim_size: POSITION,
    pub xscan_size: POSITION,
    pub yscan_size: POSITION,
    pub pixel_time: POSITION,
    pub time_delay: TIME,
    pub time_width: TIME,
    pub spimoverscanx: POSITION,
    pub spimoverscany: POSITION,
    pub save_locally: bool,
    sup0: f32,
    sup1: f32,
}

impl Settings {

    fn create_savefile_header(&self) -> String {
        let now: DateTime<Utc> = Utc::now();
        let mut val = String::new();
        let custom_datetime_format = now.format("%Y_%m_%y_%H_%M_%S").to_string();
        val.push_str(SAVE_LOCALLY_FILE);
        val.push_str(&custom_datetime_format);
        val
    }

    pub fn get_settings_from_json(file: &str) -> Result<Self, Tp3ErrorKind> {

        let mut json_file = File::open(file.to_owned() + ".json")?;
        let mut json_buffer: Vec<u8> = Vec::new();
        json_file.read_to_end(&mut json_buffer)?;
        let my_settings: Settings = serde_json::from_slice(&json_buffer)?;
        Ok(my_settings)
    }

    pub fn create_file(&self) -> Option<BufWriter<File>> {
        match self.save_locally {
            false => {None},
            true => {
            let mut jsonfile = 
                OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.create_savefile_header() + ".json").
                unwrap();
            let jsondata = serde_json::to_vec(&self).expect("Could not serialize data to JSON.");
            jsonfile.write(&jsondata).expect("Could not write to JSON data file.");
            let file =
                OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.create_savefile_header() + ".tpx3").
                unwrap();
            Some(BufWriter::new(file))
            }
        }
    }

    ///Create Settings structure reading from a TCP.
    pub fn create_settings(host_computer: [u8; 4], port: u16) -> Result<(Settings, Box<dyn misc::TimepixRead + Send>, TcpStream), Tp3ErrorKind> {
    
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
        
        let mut cam_settings = [0_u8; CONFIG_SIZE];
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

        match debug {
            false => {
                let (pack_sock, packet_addr) = pack_listener.accept().expect("Could not connect to TP3.");
                println!("Localhost TP3 detected at {:?} and {:?}.", packet_addr, pack_sock);
                Ok((my_settings, Box::new(pack_sock), ns_sock))
            },
            true => {
                let file = match File::open(READ_DEBUG_FILE) {
                    Ok(file) => file,
                    Err(_) => return Err(Tp3ErrorKind::SetNoReadFile),
                };
                println!("Debug mode. Will one file a single time.");
                Ok((my_settings, Box::new(file), ns_sock))
            },
        }

    }

    fn create_spec_debug_settings<T: ClusterCorrection>(_config: &ConfigAcquisition<T>) -> Settings  {
        Settings {
            bin: false,
            bytedepth: 4,
            cumul: false,
            mode: 00,
            xspim_size: 512,
            yspim_size: 512,
            xscan_size: 512,
            yscan_size: 512,
            pixel_time: 2560,
            time_delay: 104,
            time_width: 100,
            spimoverscanx: 1,
            spimoverscany: 1,
            save_locally: false,
            sup0: 0.0,
            sup1: 0.0,
        }
    }
    
    fn create_spim_debug_settings<T: ClusterCorrection>(config: &ConfigAcquisition<T>) -> Settings  {
        Settings {
            bin: true,
            bytedepth: 4,
            cumul: false,
            mode: 2,
            xspim_size: config.xspim,
            yspim_size: config.yspim,
            xscan_size: config.xspim,
            yscan_size: config.yspim,
            pixel_time: 2560,
            time_delay: 0,
            time_width: 1000,
            spimoverscanx: 1,
            spimoverscany: 1,
            save_locally: false,
            sup0: 0.0,
            sup1: 0.0,
        }
    }

    
    pub fn create_debug_settings<T: ClusterCorrection>(config: &ConfigAcquisition<T>) -> Result<(Settings, Box<dyn misc::TimepixRead + Send>, Box<dyn Write + Send>), Tp3ErrorKind> {
    
        let my_settings = match config.is_spim {
            true => Settings::create_spim_debug_settings(config),
            false => Settings::create_spec_debug_settings(config),
        };
        
        println!("Received settings is {:?}. Mode is {}.", my_settings, my_settings.mode);

        let in_file = match File::open(&config.file) {
            Ok(file) => file,
            Err(_) => return Err(Tp3ErrorKind::SetNoReadFile),
        };

        println!("Spectra Debug mode. Will one file a single time.");
        Ok((my_settings, Box::new(in_file), Box::new(DebugIO{})))
    }
    
}

///`ConfigAcquisition` is used for post-processing, where reading external TPX3 files is necessary.
#[derive(Debug)]
pub struct ConfigAcquisition<T: ClusterCorrection> {
    pub file: String,
    pub is_spim: bool,
    pub xspim: POSITION,
    pub yspim: POSITION,
    pub correction_type: T,
}

impl<T: ClusterCorrection> ConfigAcquisition<T> {
    pub fn file(&self) -> &str {
        &self.file
    }

    pub fn new(args: &[String], correction_type: T) -> Self {
        //if args.len() != 4+1 {
        //    panic!("One must provide 5 ({} detected) arguments (file, is_spim, xspim, yspim).", args.len()-1);
        //}
        let file = args[1].clone();
        let is_spim = args[2] == "1";
        let xspim = args[3].parse::<POSITION>().unwrap();
        let yspim = args[4].parse::<POSITION>().unwrap();
        //let value = args[5].parse::<usize>().unwrap();
        
        ConfigAcquisition {
            file,
            is_spim,
            xspim,
            yspim,
            correction_type,
        }
    }
}


///`simple_log` is used for post-processing, where reading external TPX3 files is necessary.
pub mod simple_log {
    use chrono::prelude::*;
    use std::{fs::{File, OpenOptions, create_dir_all}, path::Path};
    use std::io::Write;
    use std::io;
    use crate::errorlib::Tp3ErrorKind;

    pub fn start() -> io::Result<File> {
        let dir = Path::new("Microscope/Log/");
        create_dir_all(dir)?;
        let date = Local::now().format("%Y-%m-%d").to_string() + ".txt";
        let file_path = dir.join(&date);
        let mut file = OpenOptions::new().write(true).truncate(false).create(true).append(true).open(file_path)?;
        let date = Local::now().to_string();
        file.write_all(date.as_bytes())?;
        file.write_all(b" - Starting new loop\n")?;
        Ok(file)
    }

    pub fn ok(file: &mut File, mode: u8) -> io::Result<()> {
        let date = Local::now().to_string();
        file.write_all(date.as_bytes())?;
        file.write_all(b" - OK ")?;
        let mode = format!("{:?}", mode);
        file.write_all(mode.as_bytes())?;
        file.write_all(b"\n")?;
        Ok(())
    }

    pub fn error(file: &mut File, error: Tp3ErrorKind) -> io::Result<()> {
        let date = Local::now().to_string();
        file.write_all(date.as_bytes())?;
        file.write_all(b" - ERROR ")?;
        let error = format!("{:?}", error);
        file.write_all(error.as_bytes())?;
        file.write_all(b"\n")?;
        Ok(())
    }
}

///`misc` are miscellaneous functions.
pub mod misc {
    use std::fs::OpenOptions;
    use std::io::{Read, Write};
    use crate::errorlib::Tp3ErrorKind;
    use std::net::TcpStream;
    use std::fs::File;

    pub fn default_read_exact<R: Read + ?Sized>(this: &mut R, mut buf: &mut [u8]) -> Result<usize, Tp3ErrorKind> {
        let mut size = 0;
        while size == 0 || size % 8 != 0 {
            match this.read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    size += n;
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
                Err(_) => return Err(Tp3ErrorKind::TimepixReadLoop),
            };
        };
        if size != 0 && size % 8 == 0 {
            Ok(size)
        } else {
            Err(Tp3ErrorKind::TimepixReadOver)
        }
    }

        /*
        while !buf.is_empty() {
            match this.read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    let tmp = buf;
                    buf = &mut tmp[n..];
                }
                Err(_) => return Err(Tp3ErrorKind::TimepixRead),
            };
        };
        if buf.is_empty() {
            Ok(())
        } else {
            Err(Tp3ErrorKind::TimepixRead)
        }
    }
    */
    
    ///A modified `Read` trait. Guarantee to read at least 8 bytes.
    pub trait TimepixRead: Read {
        fn read_timepix(&mut self, buf: &mut [u8]) -> Result<usize, Tp3ErrorKind> {
            default_read_exact(self, buf)
        }
    }

    impl<R: Read + ?Sized> TimepixRead for Box<R> {}
    impl TimepixRead for TcpStream {}
    impl TimepixRead for File {}


    //General function to convert any type slice to bytes
    pub fn as_bytes<T>(v: &[T]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u8,
                v.len() * std::mem::size_of::<T>())
        }
    }

    //Convert u8 slice to u32 slice
    pub fn as_int(v: &[u8]) -> &[u32] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u32,
                v.len() * std::mem::size_of::<u8>() / std::mem::size_of::<u32>())
        }
    }

    //Convert u8 to u64. Used to get the packet_values
    pub fn packet_change(v: &[u8]) -> &[u64] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u64,
                v.len() * std::mem::size_of::<u8>() / std::mem::size_of::<u64>())
        }
    }
    
    //Creates a file and appends over. Filename must be a .tpx3 file. Data is appended in a folder
    //of the same name, that must be previously created
    pub fn output_data<T>(data: &[T], filename: String, name: &str) {
        let len = filename.len();
        let complete_filename = filename[..len-5].to_string() + "/" + name;
        let mut tfile = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(complete_filename).unwrap();
        tfile.write_all(as_bytes(data)).unwrap();
        //println!("Outputting data under {:?} name. Vector len is {}", name, data.len());
    }

}

pub mod value_types {
    pub type POSITION = u32;
    pub type INDEXHYPERSPEC = u32;
    pub type INDEX4D = u64;
    pub type COUNTER = u32;
    pub type TIME = u64;
    pub type SlType<'a> = Option<&'a [POSITION]>; //ScanList type
}

pub mod compressing {
    
    use std::io::{Read, Write};
    use std::fs;
    use std::fs::OpenOptions;
    
    fn as_int(v: &[u8]) -> &[usize] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const usize,
                //v.len() )
                v.len() * std::mem::size_of::<u8>() / std::mem::size_of::<usize>())
        }
    }
    
    fn as_bytes<T>(v: &[T]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u8,
                v.len() * std::mem::size_of::<T>())
        }
    }
    
    pub fn compress_file(file: &str) {
        let mut my_file = fs::File::open(file).expect("Could not open desired file.");
        let mut buffer: Vec<u8> = vec![0; 512_000_000];

        let mut repetitions = 0;

        let mut index: Vec<usize> = Vec::new();
        let mut count: Vec<u16> = Vec::new();
        let mut last_index = 0;
        let mut counter = 0;
        while let Ok(size) = my_file.read(&mut buffer) {
            if size == 0 {break}
            for val in as_int(&buffer) {
                if last_index != *val {
                    index.push(last_index);
                    count.push(counter);
                    counter = 0;
                } else {
                    repetitions += 1;
                    counter += 1;
                }
                last_index = *val;

            }
        }
        let mut tfile = OpenOptions::new()
            .append(true)
            .create(true)
            .open("si_complete_index.txt").expect("Could not output time histogram.");
        tfile.write_all(as_bytes(&index)).expect("Could not write time to file.");
        
        let mut tfile = OpenOptions::new()
            .append(true)
            .create(true)
            .open("si_complete_count.txt").expect("Could not output time histogram.");
        tfile.write_all(as_bytes(&count)).expect("Could not write time to file.");
        
        println!("{} and {} and {}", index.len(), count.len(), repetitions);
        
    }

}

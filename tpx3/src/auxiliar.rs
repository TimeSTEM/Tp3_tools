//!`auxiliar` is a collection of tools to set acquisition conditions.
use crate::errorlib::Tp3ErrorKind;
use std::net::{TcpListener, TcpStream, SocketAddr};
use crate::auxiliar::misc::TimepixRead;
use crate::clusterlib::cluster::ClusterCorrectionTypes;
use crate::errorlib;
use std::io::{Read, Write, BufWriter};
use std::fs::File;
use crate::constlib::*;
use crate::auxiliar::value_types::*;
use std::fs::OpenOptions;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json;

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


pub struct FileManager (Option<BufWriter<File>>);

impl Write for FileManager {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        if let Some(buffer) = &mut self.0 {
            buffer.write(buf) //Write to buffer.
        } else {
            Ok(buf.len()) //this is the behaviour as if the buffer is completely written, altough no written operation has been performed.
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        if let Some(buffer) = &mut self.0 {
            buffer.flush()
        } else {
            Ok(())
        }
    }
}

impl FileManager {
    pub fn new_empty() -> Self {
        FileManager(None)
    }
}

///`Settings` contains all relevant parameters for a given acquistion
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub bin: bool,
    pub bytedepth: POSITION,
    pub cumul: bool,
    pub mode: u8,
    pub xspim_size: POSITION, //Size returned (must be smaller than xscan_size).
    pub yspim_size: POSITION,
    pub xscan_size: POSITION, //Size of the scanning.
    pub yscan_size: POSITION,
    pub pixel_time: POSITION,
    pub time_delay: TIME,
    pub time_width: TIME,
    pub video_time: TIME,
    pub time_resolved: bool,
    save_locally: bool,
    pixel_mask: u8,
    threshold: u8,
    bias_voltage: u8,
    destination_port: u8,
    pub acquisition_us: TIME,
    sup0: f32,
    sup1: f32,
}

impl Settings {
    fn create_savefile_header(&self) -> String {
        let now: DateTime<Utc> = Utc::now();
        let mut val = String::new();
        let custom_datetime_format = now.format("%Y_%m_%d_%H_%M_%S").to_string();
        val.push_str(SAVE_LOCALLY_FILE);
        val.push_str(&custom_datetime_format);
        val
    }

    //Used a lot for postprocessing to open the correct Settings file
    pub fn get_settings_from_json(file: &str) -> Result<Self, Tp3ErrorKind> {
        let mut json_file = File::open(file.to_owned() + ".json")?;
        let mut json_buffer: Vec<u8> = Vec::new();
        json_file.read_to_end(&mut json_buffer)?;
        let my_settings: Settings = serde_json::from_slice(&json_buffer)?;
        Ok(my_settings)
    }

    pub fn create_file(&self) -> Result<FileManager, errorlib::Tp3ErrorKind> {
        match self.save_locally {
            false => {Ok(FileManager(None))},
            true => {
            let mut jsonfile = 
                OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.create_savefile_header() + ".json")?;
            let jsondata = serde_json::to_vec(&self).expect("Could not serialize data to JSON.");
            jsonfile.write(&jsondata).expect("Could not write to JSON data file.");
            let file =
                OpenOptions::new()
                .create(true)
                .append(true)
                .open(self.create_savefile_header() + ".tpx3")?;
            Ok(FileManager(Some(BufWriter::new(file))))
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
        
        //Reading from a JSON over TCP
        let mut cam_settings = [0_u8; CONFIG_SIZE];
        let size =  ns_sock.read(&mut cam_settings)?;
        let my_settings: Settings = serde_json::from_str(std::str::from_utf8(&cam_settings[0..size])?)?;
        println!("***Settings***: value is: {:?}.", my_settings);

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

    /*
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
            video_time: 0,
            save_locally: false,
            pixel_mask: 0,
            threshold: 0,
            bias_voltage: 0,
            destination_port: 0,
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
            video_time: 0,
            save_locally: false,
            pixel_mask: 0,
            threshold: 0,
            bias_voltage: 0,
            destination_port: 0,
            sup0: 0.0,
            sup1: 0.0,
        }
    }
    */

    
    pub fn create_debug_settings() -> Result<(Settings, Box<dyn misc::TimepixRead + Send>, Box<dyn Write + Send>), Tp3ErrorKind> {
    
        println!("{:?}", READ_DEBUG_FILE_JSON);
        let my_settings = Settings::get_settings_from_json(READ_DEBUG_FILE_JSON)?;
        println!("Received settings is {:?}. Mode is {}.", my_settings, my_settings.mode);

        let in_file = match File::open(READ_DEBUG_FILE) {
            Ok(file) => file,
            Err(_) => return Err(Tp3ErrorKind::SetNoReadFile),
        };

        println!("Spectra Debug mode. Will one file a single time.");
        Ok((my_settings, Box::new(in_file), Box::new(DebugIO{})))
    }
    
}

///`ConfigAcquisition` is used for post-processing, where reading external TPX3 files is necessary.
#[derive(Debug)]
pub struct ConfigAcquisition {
    pub file: String,
    pub is_spim: bool,
    pub xspim: POSITION,
    pub yspim: POSITION,
    pub correction_type: ClusterCorrectionTypes,
}

impl ConfigAcquisition {
    pub fn file(&self) -> &str {
        &self.file
    }

    pub fn new(args: &[String], correction_type: ClusterCorrectionTypes) -> Self {
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
        let file_path = dir.join(date);
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
    use crate::auxiliar::Settings;
    use crate::auxiliar::value_types::*;
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
                std::mem::size_of_val(v))
        }
    }
    
    //General function to convert any type slice to mut bytes
    pub fn as_bytes_mut<T>(v: &mut [T]) -> &mut [u8] {
        unsafe {
            std::slice::from_raw_parts_mut(
                v.as_ptr() as *mut u8,
                std::mem::size_of_val(v))
        }
    }
    
    //General function to convert bytes to any type of slice.
    pub fn from_bytes<T>(v: &[u8]) -> &[T] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const T,
                std::mem::size_of_val(v) / std::mem::size_of::<T>())
        }
    }

    //Convert u8 slice to u32 slice
    pub fn as_int(v: &[u8]) -> &[u32] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u32,
                std::mem::size_of_val(v) / std::mem::size_of::<u32>())
        }
    }

    //Convert u8 to u64. Used to get the packet_values
    pub fn packet_change(v: &[u8]) -> &[u64] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u64,
                std::mem::size_of_val(v) / std::mem::size_of::<u64>())
        }
    }

    //This checks if the electron is inside a given time_delay and time_width for a non-periodic
    //tdc reference. This is used with stocastic events.
    #[inline]
    pub fn check_if_in(etime: &TIME, phtime: &TIME, settings: &Settings) -> bool {
        (*phtime < etime + settings.time_delay + settings.time_width) && (etime + settings.time_delay < *phtime + settings.time_width)
    }
    
    //This creates the scan_list used for decoding non-trivial scan patterns
    pub fn create_list<R: std::io::Read>(mut array: R, points: POSITION) -> Result<Vec<POSITION>, Tp3ErrorKind> {
        let mut list_scan: Vec<POSITION> = vec![0; points as usize];
        array.read_exact(as_bytes_mut(&mut list_scan))?;
        Ok(list_scan)
    }
    
    //Creates a file and appends over. Filename must be a .tpx3 file. Data is appended in a folder
    //of the same name, that must be previously created
    pub fn output_data<T>(data: &[T], filename: String, name: &str) {
        let len = filename.len();
        let complete_filename = filename[..len-5].to_string() + "/" + name;
        output_data_raw(data, complete_filename);
    }

    //Creates a file and appends over. Filename must be a .tpx3 file. Data is appended in a folder
    //of the same name, that must be previously created
    pub fn output_data_raw<T>(data: &[T], filename: String) {
        let mut tfile = OpenOptions::new()
            .write(true)
            .append(true)
            .create(true)
            .open(filename).unwrap();
        tfile.write_all(as_bytes(data)).unwrap();
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

pub mod raw_into_readable {
    use std::fs;
    use std::io::prelude::*;
    use crate::constlib::*;
    use indicatif::{ProgressBar, ProgressStyle};
    use crate::clusterlib::cluster::{SinglePhoton, SingleElectron};
    use crate::packetlib::Packet;
    use crate::auxiliar::{value_types::*, misc::{output_data, packet_change}};
    use crate::tdclib::TdcType;
    use crate::errorlib::*;

    struct ToReadable {
        x: Vec<POSITION>, //If None -> TDC hit
        y: Vec<POSITION>, //If None -> TDC hit
        time: Vec<TIME>, // For both Electron 
        file: String,
    }
    impl ToReadable {
        fn add_electron(&mut self, ele: SingleElectron) {
            self.x.push(ele.x());
            self.y.push(ele.y());
            self.time.push(ele.time() * 6);
        }
        
        fn try_create_folder(&self) -> Result<(), Tp3ErrorKind> {
            let path_length = &self.file.len();
            match fs::create_dir(&self.file[..path_length - 5]) {
                Ok(_) => {Ok(())},
                Err(_) => { Err(Tp3ErrorKind::FolderAlreadyCreated) }
            }
        }
        fn add_tdc(&mut self, tdc: SinglePhoton) {
            let associate_value = TdcType::associate_value_to_enum(tdc.raw_packet_data().tdc_type()).unwrap();
            let x;
            match associate_value {
                TdcType::TdcOneRisingEdge => x = 1025,
                TdcType::TdcOneFallingEdge => x = 1026,
                TdcType::TdcTwoRisingEdge => x = 1027,
                TdcType::TdcTwoFallingEdge => x = 1028,
                TdcType::NoTdc => x = 1029,
            }
            self.x.push(x);
            self.y.push(0);
            self.time.push(tdc.time());
        }
        fn early_output_data(&mut self) {
            output_data(&self.x, self.file.clone(), "xH.txt");
            output_data(&self.y, self.file.clone(), "yH.txt");
            output_data(&self.time, self.file.clone(), "tH.txt");
            self.x.clear();
            self.y.clear();
            self.time.clear();
        }
        fn new(file: String) -> Self {
            ToReadable {
                x: Vec::new(),
                y: Vec::new(),
                time: Vec::new(),
                file
            }
        }
    }

    pub fn build_data(path: &str, limit_read_size: u32) -> Result<(), Tp3ErrorKind> {
        //Opening the raw data file. We have already checked if the file opens so no worries here.
        let mut file = fs::File::open(path).unwrap();

        let progress_size = file.metadata().unwrap().len();
        let mut ci = 0;

        let mut buffer: Vec<u8> = vec![0; TP3_BUFFER_SIZE];
        let mut total_size = 0;

        let mut data_handler = ToReadable::new(path.to_string());
        data_handler.try_create_folder()?;
        
        let bar = ProgressBar::new(progress_size);
        bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:40.white/black} {percent}% {pos:>7}/{len:7} [ETA: {eta}] Searching electron photon coincidences")
                      .unwrap()
                      .progress_chars("=>-"));

        while let Ok(size) = file.read(&mut buffer) {
            if size == 0 {println!("Finished Reading."); break;}
            total_size += size;
            if limit_read_size != 0 && total_size as u32 >= limit_read_size {break;}
            bar.inc(TP3_BUFFER_SIZE as u64);
            buffer[0..size].chunks_exact(8).enumerate().for_each(|(current_raw_index, pack_oct)| {
                let packet = Packet::new(ci, packet_change(pack_oct)[0]);
                match *pack_oct {
                    [84, 80, 88, 51, nci, _, _, _] => { ci=nci; },
                    _ => {
                        match packet.id() {
                            6 => { //TDC hit
                                let photon = SinglePhoton::new(packet, 0, None, current_raw_index);
                                data_handler.add_tdc(photon);
                            },
                            11 => { //Electron hit
                                let se = SingleElectron::new(packet, None, current_raw_index);
                                data_handler.add_electron(se);
                            },
                            _ => {},
                        };
                    },
                };
            });
            data_handler.early_output_data();
        }
        Ok(())
    }
}

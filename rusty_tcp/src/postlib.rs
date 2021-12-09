
pub mod coincidence {

    use crate::packetlib::{Packet, PacketEELS as Pack};
    use crate::tdclib::{TdcControl, TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
    use std::io;
    use std::io::prelude::*;
    use std::fs;
    use rayon::prelude::*;
    use std::time::Instant;

    const TIME_WIDTH: usize = 200; //Time width to correlate (ns).
    const TIME_DELAY: usize = 160; //Time delay to correlate (ns).
    const MIN_LEN: usize = 100; // Sliding time window size.
    const SPIM_PIXELS: usize = 1025; //Number of pixels in the spim. Last pixel is currently off.
    const VIDEO_TIME: usize = 5000; //Video time for spim (ns).
    const CLUSTER_DET:usize = 50; //Cluster time window (ns).

    #[derive(Debug)]
    pub struct Config {
        file: String,
        is_spim: bool,
        xspim: usize,
        yspim: usize,
    }

    impl Config {
        pub fn file(&self) -> &str {
            &self.file
        }

        pub fn new(args: &[String]) -> Self {
            if args.len() != 4+1 {
                panic!("One must provide 04 ({} detected) arguments (file, is_spim, xspim, yspim).", args.len()-1);
            }
            let file = args[1].clone();
            let is_spim = args[2] == "1";
            let xspim = args[3].parse::<usize>().unwrap();
            let yspim = args[4].parse::<usize>().unwrap();
            let my_config = 
            Config {
                file,
                is_spim,
                xspim,
                yspim,
            };
            println!("Configuration for the coincidence measurement is {:?}", my_config);
            my_config
        }
    }

    pub struct ElectronData {
        pub time: Vec<usize>,
        pub rel_time: Vec<isize>,
        pub x: Vec<usize>,
        pub y: Vec<usize>,
        pub tot: Vec<u16>,
        pub cluster_size: Vec<usize>,
        pub spectrum: Vec<usize>,
        pub corr_spectrum: Vec<usize>,
        pub is_spim: bool,
        pub spim_size: (usize, usize),
        pub begin_frame: Option<usize>,
        pub spim_period: Option<usize>,
        pub spim_low_time: Option<usize>,
        pub spim_index: Vec<usize>,
    }

    impl ElectronData {
        fn add_electron(&mut self, val: SingleElectron) {
            self.spectrum[val.data.1 + 1024*val.data.2] += 1;
        }

        fn add_spim_line<T: TdcControl + ?Sized >(&mut self, pack: &Pack, spim_tdc: &mut T) {
            if self.is_spim {
                spim_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
                if (spim_tdc.counter() / 2) % self.spim_size.1 == 0 {
                    self.begin_frame = Some(spim_tdc.time());
                }
            }
        }

        fn add_coincident_electron(&mut self, val: SingleElectron, photon_time: usize) {
            self.corr_spectrum[val.data.1 + 1024*val.data.2] += 1;
            self.time.push(val.data.0);
            self.rel_time.push(val.data.0 as isize - photon_time as isize);
            self.x.push(val.data.1);
            self.y.push(val.data.2);
            if self.is_spim {
                if let Some(index) = self.calculate_index(val.data.3, val.data.1) {
                    self.spim_index.push(index);
                }
            }
        }

        
        fn calculate_index(&self, dt: usize, x: usize) -> Option<usize> {
            let per = self.spim_period.expect("Spim period was none.");
            let lt = self.spim_low_time.expect("Spim low time was none.");
            let val = dt % per;
            if val < lt {
                let mut r = dt / per; //how many periods -> which line to put.
                let rin = self.spim_size.0 * val / lt; //Column
        
                if r > (self.spim_size.1-1) {
                    r %= self.spim_size.1
                }
                
                Some((r * self.spim_size.0 + rin) * SPIM_PIXELS + x)
            } else {
                None
            }
        }

        fn add_events(&mut self, mut temp_edata: TempElectronData, mut temp_tdc: TempTdcData) {
            temp_edata.sort();
            temp_tdc.sort();
            let nelectrons = temp_edata.electron.len();
            let nphotons = temp_tdc.tdc.len();
            
            let mut cs = temp_edata.remove_clusters();
            let nclusters = cs.len();
            self.cluster_size.append(&mut cs);
        
            println!("Electron events: {}. Number of clusters: {}, Number of photons: {}", nelectrons, nclusters, nphotons);

            for val in temp_edata.electron {
                self.add_electron(val);
                if let Some(pht) = temp_tdc.check(val) {
                    self.add_coincident_electron(val, pht);
                }
            };
            
            println!("Number of coincident electrons: {:?}", self.x.len());
        }

        fn prepare_spim(&mut self, spim_tdc: &PeriodicTdcRef) {
            assert!(self.is_spim);
            self.spim_period = Some(spim_tdc.period);
            self.spim_low_time = Some(spim_tdc.low_time);
        }

        pub fn new(my_config: &Config) -> Self {
            Self {
                time: Vec::new(),
                rel_time: Vec::new(),
                x: Vec::new(),
                y: Vec::new(),
                tot: Vec::new(),
                cluster_size: Vec::new(),
                spectrum: vec![0; 1024*256],
                corr_spectrum: vec![0; 1024*256],
                is_spim: my_config.is_spim,
                spim_size: (my_config.xspim, my_config.yspim),
                begin_frame: None,
                spim_period: None,
                spim_low_time: None,
                spim_index: Vec::new(),
            }
        }
        
        pub fn output_corr_spectrum(&self, bin: bool) {
            let out: String = match bin {
                true => {
                    let mut spec: Vec<usize> = vec![0; 1024];
                    for val in self.corr_spectrum.chunks_exact(1024) {
                        spec.iter_mut().zip(val.iter()).map(|(a, b)| *a += b).count();
                    }
                    spec.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ")
                },
                false => {
                    self.corr_spectrum.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ")
                },
            };
            fs::write("cspec.txt", out).unwrap();
        }
        
        pub fn output_spectrum(&self, bin: bool) {
            let out: String = match bin {
                true => {
                    let mut spec: Vec<usize> = vec![0; 1024];
                    for val in self.spectrum.chunks_exact(1024) {
                        spec.iter_mut().zip(val.iter()).map(|(a, b)| *a += b).count();
                    }
                    spec.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ")
                },
                false => {
                    self.spectrum.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ")
                },
            };
            fs::write("spec.txt", out).unwrap();
        }

        pub fn output_relative_time(&self) {
            println!("Outputting relative time under tH name. Vector len is {}", self.rel_time.len());
            let out: String = self.rel_time.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ");
            fs::write("tH.txt", out).unwrap();
        }
        
        pub fn output_dispersive(&self) {
            println!("Outputting each dispersive value under xH name. Vector len is {}", self.rel_time.len());
            let out: String = self.x.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ");
            fs::write("xH.txt", out).unwrap();
        }
        
        pub fn output_non_dispersive(&self) {
            println!("Outputting each non-dispersive value under yH name. Vector len is {}", self.rel_time.len());
            let out: String = self.y.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ");
            fs::write("yH.txt", out).unwrap();
        }
        
        pub fn output_spim_index(&self) {
            println!("Outputting each spim index value under si name. Vector len is {}", self.spim_index.len());
            let out: String = self.spim_index.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ");
            fs::write("si.txt", out).unwrap();
        }

        pub fn output_cluster_size(&self) {
            let out: String = self.cluster_size.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ");
            fs::write("cs.txt", out).unwrap();
        }

        pub fn output_tot(&self, sum_cluster: bool) {
            let out: String = match sum_cluster {
                false => {
                    self.tot.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ")
                },
                true => {
                    self.tot.iter().zip(self.cluster_size.iter()).map(|(tot, cs)| (*tot as usize * cs).to_string()).collect::<Vec<String>>().join(", ")
                },
            };
            fs::write("tot.txt", out).unwrap();
        }

            
    }

    pub struct TempTdcData {
        pub tdc: Vec<usize>,
        pub min_index: usize,
    }

    impl TempTdcData {
        fn new() -> Self {
            Self {
                tdc: Vec::new(),
                min_index: 0,
            }
        }

        fn add_tdc(&mut self, my_pack: &Pack) {
            self.tdc.push(my_pack.tdc_time_norm() - TIME_DELAY);
        }

        fn sort(&mut self) {
            self.tdc.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        }

        fn check(&mut self, value: SingleElectron) -> Option<usize> {

            //Sometimes you have less photons than the min_index. That would panic.
            if self.min_index + MIN_LEN > self.tdc.len() {
                return None
            }
            
            let result = self.tdc[self.min_index..self.min_index+MIN_LEN].iter()
                .enumerate()
                .find(|(_, x)| ((**x as isize - value.data.0 as isize).abs() as usize) < TIME_WIDTH);
            
            match result {
                Some((index, pht_value)) => {
                    if index > MIN_LEN/10 && self.tdc.len()>self.min_index + MIN_LEN + index {
                       self.min_index += index/2;
                    }
                    Some(*pht_value)
                },
                None => None,
            }
        }
    }

    #[derive(Copy, Clone)]
    pub struct SingleElectron {
        pub data: (usize, usize, usize, usize),
    }


    impl SingleElectron {
        fn try_new(pack: &Pack, begin_frame: Option<usize>) -> Option<Self> {
            let ele_time = pack.electron_time();
            match begin_frame {
                Some(frame_time) if ele_time > frame_time + VIDEO_TIME => {
                    Some(SingleElectron {
                        data: (ele_time, pack.x(), pack.y(), ele_time - frame_time - VIDEO_TIME),
                    })
                },
                None => {
                    Some(SingleElectron {
                        data: (ele_time, pack.x(), pack.y(), 0),
                    })
                },
                _ => None,
            }
        }

        fn is_new_cluster(f: &SingleElectron, s: &SingleElectron) -> bool {
            if f.data.0 > s.data.0 + CLUSTER_DET || (f.data.1 as isize - s.data.1 as isize).abs() > 2 || (f.data.2 as isize - s.data.2 as isize).abs() > 2 {
                true
            } else {
                false
            }
        }


        fn new_from_cluster(cluster: &[SingleElectron]) -> SingleElectron {
            let cluster_size: usize = cluster.len();
            
            let t_mean:usize = cluster.iter().map(|se| se.data.0).sum::<usize>() / cluster_size as usize;
            //let t_mean:usize = cluster.iter().map(|se| se.data.0).next().unwrap();
            
            let x_mean:usize = cluster.iter().map(|se| se.data.1).sum::<usize>() / cluster_size;
            //let x_mean:usize = cluster.iter().map(|se| se.data.1).next().unwrap();
            
            let y_mean:usize = cluster.iter().map(|se| se.data.2).sum::<usize>() / cluster_size;
            //let y_mean:usize = cluster.iter().map(|se| se.data.2).next().unwrap();
            
            //let tot_mean: u16 = (cluster_vec.iter().map(|&(_, _, _, tot, _)| tot as usize).sum::<usize>() / cluster_size) as u16;
            
            let time_dif: usize = cluster.iter().map(|se| se.data.3).next().unwrap();
            
            SingleElectron {
                data: (t_mean, x_mean, y_mean, time_dif),
            }
        }
    }


    pub struct TempElectronData {
        pub electron: Vec<SingleElectron>, //Time, X, Y and ToT and Time difference (for Spim positioning)
        pub min_index: usize,
    }

    impl TempElectronData {
        fn new() -> Self {
            Self {
                electron: Vec::new(),
                min_index: 0,
            }
        }

        fn remove_clusters(&mut self) -> Vec<usize> {
            let mut nelist:Vec<SingleElectron> = Vec::new();
            let mut cs_list: Vec<usize> = Vec::new();

            let mut last: SingleElectron = self.electron[0];
            let mut cluster_vec: Vec<SingleElectron> = Vec::new();
            for x in &self.electron {
                if SingleElectron::is_new_cluster(x, &last) {
                    let cluster_size: usize = cluster_vec.len();
                    let new_from_cluster = SingleElectron::new_from_cluster(&cluster_vec);
                    nelist.push(new_from_cluster);
                    cs_list.push(cluster_size);
                    cluster_vec = Vec::new();
                }
                last = *x;
                cluster_vec.push(*x);
            }
            self.electron = nelist;
            cs_list
        }


        fn add_temp_electron(&mut self, my_pack: &Pack, frame_time: Option<usize>) {
            if let Some(se) = SingleElectron::try_new(my_pack, frame_time) {
                self.electron.push(se);
            }
        }

        fn sort(&mut self) {
            self.electron.par_sort_unstable_by(|a, b| (a.data).partial_cmp(&b.data).unwrap());
        }
    }
            

    pub fn search_coincidence(file: &str, coinc_data: &mut ElectronData) -> io::Result<()> {
        
        let mut file0 = fs::File::open(file)?;
        
        let mut spim_tdc: Box<dyn TdcControl> = if coinc_data.is_spim {
            if coinc_data.spim_size.0 == 0 || coinc_data.spim_size.1 == 0 {
                panic!("Spim mode is on. X and Y pixels must be greater than 0.");
            }
            let temp = PeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut file0).expect("Could not create period TDC reference.");
            coinc_data.prepare_spim(&temp);
            Box::new(temp)
        } else {
            Box::new(NonPeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut file0).expect("Could not create non periodic TDC reference."))
        };
        let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoRisingEdge, &mut file0).expect("Could not create non periodic (photon) TDC reference.");

        
        
        let mut ci = 0;
        let mut file = fs::File::open(file)?;
        let mut buffer: Vec<u8> = vec![0; 256_000_000];
        let mut total_size = 0;
        let start = Instant::now();
        
        while let Ok(size) = file.read(&mut buffer) {
            if size == 0 {println!("Finished Reading."); break;}
            total_size += size;
            println!("MB Read: {}", total_size / 1_000_000 );
            let mut temp_edata = TempElectronData::new();
            let mut temp_tdc = TempTdcData::new();
            //let mut packet_chunks = buffer[0..size].chunks_exact(8);
            buffer[0..size].chunks_exact(8).for_each(|pack_oct| {
                match *pack_oct {
                    [84, 80, 88, 51, nci, _, _, _] => {ci=nci as usize;},
                    _ => {
                        let packet = Pack { chip_index: ci, data: pack_oct };
                        match packet.id() {
                            6 if packet.tdc_type() == np_tdc.id() => {
                                temp_tdc.add_tdc(&packet);
                            },
                            6 if packet.tdc_type() == spim_tdc.id() => {
                                coinc_data.add_spim_line(&packet, &mut *spim_tdc);
                            },
                            11 => {
                                temp_edata.add_temp_electron(&packet, coinc_data.begin_frame);
                            },
                            _ => {},
                        };
                    },
                };
            });
        coinc_data.add_events(temp_edata, temp_tdc);
        println!("Time elapsed: {:?}", start.elapsed());

        }
        println!("Total number of bytes read {}", total_size);
        Ok(())
    }
}


pub mod time_resolved {
    use crate::packetlib::{Packet, PacketEELS as Pack};
    use crate::tdclib::{TdcControl, TdcType, PeriodicTdcRef};
    use std::io::prelude::*;
    use rayon::prelude::*;
    use std::fs;
    
    const CLUSTER_DET:usize = 100; //Cluster time window (ns).

    #[derive(Debug)]
    pub enum ErrorType {
        OutOfBounds,
        FolderDoesNotExist,
        FolderNotCreated,
        ScanOutofBounds,
        MinGreaterThanMax,
    }

    const VIDEO_TIME: usize = 5_000;

    pub trait TimeTypes {
        fn prepare(&mut self, file: &mut fs::File);
        fn add_packet(&mut self, packet: &Pack);
        fn add_tdc(&mut self, packet: &Pack);
        fn output(&mut self) -> Result<(), ErrorType>;
        fn display_info(&self) -> Result<(), ErrorType>;
    }

    pub struct TimeSet {
        pub set: Vec<Box<dyn TimeTypes>>,
    }

    /// This enables spectral analysis in a certain spectral window.
    pub struct TimeSpectral {
        pub spectra: Vec<[usize; 1024]>,
        pub initial_time: Option<usize>,
        pub interval: usize,
        pub counter: Vec<usize>,
        pub min: usize,
        pub max: usize,
        pub folder: String,
    }

    impl TimeTypes for TimeSpectral {
        fn prepare(&mut self, _file: &mut fs::File) {
        }

        fn add_packet(&mut self, packet: &Pack) {
            self.initial_time = match self.initial_time {
                Some(t) => {Some(t)},
                None => {Some(packet.electron_time())},
            };

            if let Some(offset) = self.initial_time {
                let vec_index = (packet.electron_time()-offset) / self.interval;
                while self.spectra.len() < vec_index+1 {
                    self.spectra.push([0; 1024]);
                    self.counter.push(0);
                }
                if packet.x()>self.min && packet.x()<self.max {
                    self.spectra[vec_index][packet.x()] += 1;
                    self.counter[vec_index] += 1;
                };
            }
        }

        fn add_tdc(&mut self, _packet: &Pack) {
        }
        
        fn output(&mut self) -> Result<(), ErrorType> {
            if let Err(_) = fs::read_dir(&self.folder) {
                if let Err(_) = fs::create_dir(&self.folder) {
                    return Err(ErrorType::FolderNotCreated);
                }
            }
            /*
            let mut entries = match fs::read_dir(&self.folder) {
                Ok(e) => e,
                Err(_) => return Err(ErrorType::FolderDoesNotExist),
            };
            
            while let Some(x) = entries.next() {
                let path = x.unwrap().path();
                let dir = path.to_str().unwrap();
                fs::remove_file(dir).unwrap();
            };
            */
            let mut folder: String = String::from(&self.folder);
            folder.push_str("\\");
            folder.push_str(&(self.spectra.len()).to_string());
            folder.push_str("_");
            folder.push_str(&self.min.to_string());
            folder.push_str("_");
            folder.push_str(&self.max.to_string());

            let out = self.spectra.iter().flatten().map(|x| x.to_string()).collect::<Vec<String>>().join(", ");
            if let Err(_) = fs::write(&folder, out) {
                return Err(ErrorType::FolderDoesNotExist);
            }
            
            folder.push_str("_");
            folder.push_str("counter");

            let out = self.counter.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ");
            if let Err(_) = fs::write(folder, out) {
                return Err(ErrorType::FolderDoesNotExist);
            }
            Ok(())
        }

        fn display_info(&self) -> Result<(), ErrorType> {
            let number = self.counter.iter().sum::<usize>();
            println!("Total number of spectra are: {}. Total number of electrons are: {}. Electrons / spectra is {}. First electron detected at {:?}", self.spectra.len(), number, number / self.spectra.len(), self.initial_time);
            Ok(())
        }
    }

    
    impl TimeSpectral {

        pub fn new(interval: usize, xmin: usize, xmax: usize, folder: String) -> Result<Self, ErrorType> {
            if xmax>1024 {return Err(ErrorType::OutOfBounds)}
            Ok(Self {
                spectra: Vec::new(),
                interval: interval,
                counter: Vec::new(),
                initial_time: None,
                min: xmin,
                max: xmax,
                folder: folder,
            })
        }
    }

    #[derive(Copy, Clone, Debug)]
    pub struct SingleElectron {
        pub data: (usize, usize, usize, usize, usize), //Time, X, Y, spim slice, array_pos;
    }

    impl SingleElectron {
        fn new(pack: &Pack, slice: usize, array_pos: usize) -> SingleElectron {
            SingleElectron {
                data: (pack.electron_time(), pack.x(), pack.y(), slice, array_pos)
            }
        }
        
        fn is_new_cluster(f: &SingleElectron, s: &SingleElectron) -> bool {
            if f.data.0 > s.data.0 + CLUSTER_DET || (f.data.1 as isize - s.data.1 as isize).abs() > 2 || (f.data.2 as isize - s.data.2 as isize).abs() > 2 {
                true
            } else {
                false
            }
        }
        
        fn new_from_cluster(cluster: &[SingleElectron]) -> SingleElectron {
            let cluster_size = cluster.len();
            
            //let t:usize = cluster.iter().map(|se| se.data.0).next().unwrap();
            let t_mean:usize = cluster.iter().map(|se| se.data.0).sum::<usize>() / cluster_size as usize;
            
            //let x:usize = cluster.iter().map(|se| se.data.1).next().unwrap();
            let x_mean:usize = cluster.iter().map(|se| se.data.1).sum::<usize>() / cluster_size as usize;
            
            //let y:usize = cluster.iter().map(|se| se.data.2).next().unwrap();
            let y_mean:usize = cluster.iter().map(|se| se.data.2).sum::<usize>() / cluster_size as usize;
            
            let slice:usize = cluster.iter().map(|se| se.data.3).next().unwrap();
            let array_pos:usize = cluster.iter().map(|se| se.data.4).next().unwrap();
            
            SingleElectron {
                data: (t_mean, x_mean, y_mean, slice, array_pos),
            }
        }
    }

    /// This enables spatial+spectral analysis in a certain spectral window.
    pub struct TimeSpectralSpatial {
        pub spectra: Vec<Vec<usize>>,
        pub ensemble: Vec<SingleElectron>,
        pub initial_time: Option<usize>,
        pub cycle_counter: usize, //Electron overflow counter
        pub cycle_trigger: bool, //Electron overflow control
        pub interval: usize, //time interval you want to form spims
        pub counter: Vec<usize>,
        pub min: usize,
        pub max: usize,
        pub folder: String,
        pub spimx: usize,
        pub spimy: usize,
        pub scanx: Option<usize>,
        pub scany: Option<usize>,
        pub line_offset: usize,
        pub is_image: bool,
        pub is_spim: bool,
        pub spec_bin: Option<usize>,
        pub tdc_periodic: Option<PeriodicTdcRef>,
        pub tdc_type: TdcType,
    }
    
    impl TimeTypes for TimeSpectralSpatial {
        fn prepare(&mut self, file: &mut fs::File) {
            self.tdc_periodic = match self.tdc_periodic {
                None => {
                    let val = Some(PeriodicTdcRef::new(self.tdc_type, file).expect("Problem in creating periodic tdc ref."));
                    val
                },
                Some(val) => Some(val),
            };
        }
        
        fn add_packet(&mut self, packet: &Pack) {
            //Getting Initial Time
            self.initial_time = match self.initial_time {
                Some(t) => {Some(t)},
                None => {Some(packet.electron_time())},
            };

            //Correcting Electron Time
            let el = packet.electron_time();
            if el > 26_700_000_000 && self.cycle_trigger {
                self.cycle_counter += 1;
                self.cycle_trigger = false;
            }
            else if el > 100_000_000 && packet.electron_time() < 13_000_000_000 && !self.cycle_trigger {
                self.cycle_trigger = true;
            }
            //let corrected_el = if !self.cycle_trigger && (el + self.cycle_counter * Pack::electron_reset_time()) > ((0.5 + self.cycle_counter) * Pack::electron_reset_time()) {
            let corrected_el = if !self.cycle_trigger && (el + self.cycle_counter * Pack::electron_reset_time()) > (self.cycle_counter * Pack::electron_reset_time() + Pack::electron_reset_time() / 2) {
                el
            } else {
                el + self.cycle_counter * Pack::electron_reset_time()
            };

            //Creating the array using the electron corrected time. Note that you dont need to use
            //it in the 'spim_detector' if you synchronize the clocks.
            if let Some(offset) = self.initial_time {
                let vec_index = (corrected_el-offset) / self.interval;
                while self.spectra.len() < vec_index + 1 {
                    self.expand_data();
                    self.counter.push(0);
                    self.try_clean_and_append();
                }
                match self.spim_detector(packet.electron_time() - VIDEO_TIME) {
                    Some(array_pos) if packet.x()>self.min && packet.x()<self.max => {
                        self.counter[vec_index] += 1;
                        let se = SingleElectron::new(packet, vec_index, array_pos);
                        self.ensemble.push(se);
                    },
                    _ => {},
                };
            }
        }

        fn add_tdc(&mut self, packet: &Pack) {
            //Synchronizing clocks using two different approaches. It is always better to use a
            //multiple of 2 and use the FPGA counter.
            match &mut self.tdc_periodic {
                Some(my_tdc_periodic) if packet.tdc_type() == self.tdc_type.associate_value() => {
                    my_tdc_periodic.upt(packet.tdc_time_norm(), packet.tdc_counter());
                    if  (my_tdc_periodic.counter() / 2) % (self.spimy) == 0 {
                        my_tdc_periodic.begin_frame = my_tdc_periodic.time();
                    }
                },
                _ => {},
            };
        }

        fn output(&mut self) -> Result<(), ErrorType> {
            if let Err(_) = fs::read_dir(&self.folder) {
                if let Err(_) = fs::create_dir(&self.folder) {
                    return Err(ErrorType::FolderNotCreated);
                }
            }
            
            self.try_clean_and_append();
            
            let mut folder: String = String::from(&self.folder);
            folder.push_str("\\");
            folder.push_str(&(self.spectra.len()).to_string());
            folder.push_str("_");
            folder.push_str(&self.min.to_string());
            folder.push_str("_");
            folder.push_str(&self.max.to_string());
            if !self.is_image && !self.is_spim {
                folder.push_str("_");
                folder.push_str(&self.scanx.unwrap().to_string());
                folder.push_str("_");
                folder.push_str(&self.scany.unwrap().to_string());
                folder.push_str("_");
                folder.push_str(&self.spec_bin.unwrap().to_string());
            } else {
                if self.is_image {folder.push_str("_spim");}
                else {folder.push_str("_spimComplete");}
            }


            let out = self.spectra.iter().flatten().map(|x| x.to_string()).collect::<Vec<String>>().join(", ");
            if let Err(_) = fs::write(&folder, out) {
                return Err(ErrorType::FolderDoesNotExist);
            }
         
            if !self.is_image && !self.is_spim {
                folder.push_str("_");
                folder.push_str("counter");
                let out = self.counter.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ");
                if let Err(_) = fs::write(folder, out) {
                    return Err(ErrorType::FolderDoesNotExist);
                }
            }


            Ok(())
        }

        fn display_info(&self) -> Result<(), ErrorType> {
            let number = self.counter.iter().sum::<usize>();
            println!("Total number of spims are: {}. Total number of electrons are: {}. Electrons / spim are {}. First electron detected at {:?}. TDC period (ns) is {}. TDC low time (ns) is {}. Output is image: {}. Scanx, Scany and Spec_bin is {:?}, {:?} and {:?} (must be all None is is_image). Is a complete spim: {}.", self.spectra.len(), number, number / self.spectra.len(), self.initial_time, self.tdc_periodic.expect("TDC periodic is None during display_info.").period, self.tdc_periodic.expect("TDC periodic is None during display_info.").low_time, self.is_image, self.scanx, self.scany, self.spec_bin, self.is_spim);
            Ok(())
        }
    }
    
    impl TimeSpectralSpatial {

        pub fn new(interval: usize, xmin: usize, xmax: usize, spimx: usize, spimy: usize, lineoffset: usize, scan_parameters: Option<(usize, usize, usize)>, tdc_type: TdcType, folder: String) -> Result<Self, ErrorType> {
            if xmax>1024 {return Err(ErrorType::OutOfBounds)};
            if xmin>xmax {return Err(ErrorType::MinGreaterThanMax)};
            let (is_image, is_spim) = match scan_parameters {
                None if (xmin==0 && xmax==1024)  => (false, true),
                Some(_) => (false, false),
                _ => (true, false),
            };
            
            let (scanx, scany, spec_bin) = match scan_parameters {
                Some((x, y, bin)) => {
                    if x>spimx || y>spimy {
                        return Err(ErrorType::ScanOutofBounds)
                    };
                    (Some(x), Some(y), Some(bin))
                },
                None => {
                    (None, None, None)
                },
            };

            Ok(Self {
                spectra: Vec::new(),
                ensemble: Vec::new(),
                interval: interval,
                counter: Vec::new(),
                initial_time: None,
                cycle_counter: 0,
                cycle_trigger: true,
                min: xmin,
                max: xmax,
                spimx: spimx,
                spimy: spimy,
                scanx: scanx,
                scany: scany,
                line_offset: lineoffset,
                is_image: is_image,
                is_spim: is_spim,
                spec_bin: spec_bin,
                folder: folder,
                tdc_periodic: None,
                tdc_type: tdc_type,
            })
        }


        
        fn spim_detector(&self, ele_time: usize) -> Option<usize> {
            if let Some(tdc_periodic) = self.tdc_periodic {
                let begin = tdc_periodic.begin_frame;
                let interval = tdc_periodic.low_time;
                let period = tdc_periodic.period;
               
                let dt = ele_time - begin;
                let val = dt % period;
                if val >= interval { return None; }
                let mut r =  dt / period + self.line_offset;
                let rin = val * self.spimx / interval;
                
                if r > (self.spimy - 1) {
                    if r > Pack::electron_reset_time() {return None;}
                    r %= self.spimy;
                }

                let result = r * self.spimx + rin;
                match (self.scanx, self.scany, self.spec_bin) {
                    (None, None, None) => Some(result),
                    (Some(posx), Some(posy), Some(spec_bin)) if (posx as isize-rin as isize).abs()<spec_bin as isize && (posy as isize-r as isize).abs()<spec_bin as isize => Some(result),
                    _ => None,
                }
            } else {None}
        }

        fn expand_data(&mut self) {
            if self.is_spim {
                self.spectra.push(vec![0; self.spimx*self.spimy*1024]);
            } else {
                if self.is_image {
                    self.spectra.push(vec![0; self.spimx*self.spimy]);
                } else {
                    self.spectra.push(vec![0; 1024]);
                }
            }
        }

        fn try_clean_and_append(&mut self) {
            if self.ensemble.len() > 0 {
                self.sort();
                self.remove_clusters();
                for val in &self.ensemble {
                    if self.is_spim {
                        self.spectra[val.data.3][val.data.4*1024+val.data.1] += 1;
                    } else {
                        if self.is_image {
                            self.spectra[val.data.3][val.data.4] += 1;
                        } else {
                            self.spectra[val.data.3][val.data.1] += 1;
                        }
                    }
                }
            }
            self.ensemble = Vec::new();
        }

        fn remove_clusters(&mut self) -> Vec<usize> {
            let nelectrons = self.ensemble.len();
            let mut nelist:Vec<SingleElectron> = Vec::new();
            let mut cs_list: Vec<usize> = Vec::new();

            let mut last: SingleElectron = self.ensemble[0];
            let mut cluster_vec: Vec<SingleElectron> = Vec::new();
            for x in &self.ensemble {
                if SingleElectron::is_new_cluster(x, &last) {
                    let cluster_size: usize = cluster_vec.len();
                    let new_from_cluster = SingleElectron::new_from_cluster(&cluster_vec);
                    nelist.push(new_from_cluster);
                    cs_list.push(cluster_size);
                    cluster_vec = Vec::new();
                }
                last = *x;
                cluster_vec.push(*x);
            }
            self.ensemble = nelist;
            let new_nelectrons = self.ensemble.len();
            println!("Number of electrons: {}. Number of clusters: {}. Electrons per cluster: {}", nelectrons, new_nelectrons, nelectrons as f32/new_nelectrons as f32); 
            cs_list
        }
        
        fn sort(&mut self) {
            self.ensemble.par_sort_unstable_by(|a, b| (a.data).partial_cmp(&b.data).unwrap());
        }
    }

    pub fn analyze_data(file: &str, data: &mut TimeSet) {

        for each in data.set.iter_mut() {
            let mut file = fs::File::open(file).expect("Could not open desired file.");
            each.prepare(&mut file);
        }

        let mut file = fs::File::open(file).expect("Could not open desired file.");
        let mut buffer: Vec<u8> = Vec::new();
        file.read_to_end(&mut buffer).expect("Could not write file on buffer.");

        let mut ci = 0usize;
        let mut packet_chunks = buffer.chunks_exact(8);

        while let Some(pack_oct) = packet_chunks.next() {
            match pack_oct {
                &[84, 80, 88, 51, nci, _, _, _] => {ci = nci as usize},
                _ => {
                    let packet = Pack{chip_index: ci, data: pack_oct};
                    match packet.id() {
                        6 => {
                            for each in data.set.iter_mut() {
                                each.add_tdc(&packet);
                            }
                        },
                        11 => {
                            for each in data.set.iter_mut() {
                                each.add_packet(&packet);
                            }
                        },
                        _ => {},
                    };
                },
            };
        };
    }
}

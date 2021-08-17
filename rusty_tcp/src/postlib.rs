pub mod coincidence {

    use crate::packetlib::{Packet, PacketEELS as Pack};
    use crate::tdclib::TdcType;
    use std::io;
    use std::io::prelude::*;
    use std::fs;
    //use std::time::Instant;

    const TIME_WIDTH: f64 = 50.0e-9;
    const TIME_DELAY: f64 = 165.0e-9;
    const MIN_LEN: usize = 100; // This is the minimal TDC vec size. It reduces over time.
    const EXC: (usize, usize) = (20, 5); //This controls how TDC vec reduces. (20, 5) means if correlation is got in the time index >20, the first 5 items are erased.
    const CLUSTER_DET:f64 = 50.0e-09;

    pub struct ElectronData {
        pub time: Vec<f64>,
        pub rel_time: Vec<f64>,
        pub x: Vec<usize>,
        pub y: Vec<usize>,
        pub tot: Vec<u16>,
        pub cluster_size: Vec<usize>,
        pub spectrum: Vec<usize>,
        pub corr_spectrum: Vec<usize>,
    }

    impl ElectronData {
        fn add_electron(&mut self, val: (f64, usize, usize, u16)) {
            self.spectrum[val.1 + 1024 * val.2] += 1;
        }

        fn add_coincident_electron(&mut self, val: (f64, usize, usize, u16), photon_time: f64) {
            self.corr_spectrum[val.1 + 1024*val.2] += 1;
            self.time.push(val.0);
            self.rel_time.push(val.0-photon_time);
            self.x.push(val.1);
            self.y.push(val.2);
            self.tot.push(val.3);
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
                if let Some(pht) = temp_tdc.check(val.0) {
                    self.add_coincident_electron(val, pht);
                }
            }
        }

        pub fn new() -> Self {
            Self {
                time: Vec::new(),
                rel_time: Vec::new(),
                x: Vec::new(),
                y: Vec::new(),
                tot: Vec::new(),
                cluster_size: Vec::new(),
                spectrum: vec![0; 1024*256],
                corr_spectrum: vec![0; 1024*256],
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
            fs::write("xyH.txt", out).unwrap();
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
            fs::write("xHT.txt", out).unwrap();
        }

        pub fn output_relative_time(&self) {
            let out: String = self.rel_time.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(", ");
            fs::write("tH.txt", out).unwrap();
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
        pub tdc: Vec<f64>,
    }

    impl TempTdcData {
        fn new() -> Self {
            Self {
                tdc: Vec::new(),
            }
        }

        fn add_tdc(&mut self, my_pack: &Pack) {
            self.tdc.push(my_pack.tdc_time_norm() - TIME_DELAY);
        }

        fn sort(&mut self) {
            self.tdc.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        }

        fn check(&mut self, value: f64) -> Option<f64> {
            let veclen = self.tdc.len().min(2*MIN_LEN);
            let result = self.tdc[0..veclen].iter().cloned().enumerate().filter(|(_, x)| (x-value).abs()<TIME_WIDTH).next();
            match result {
                Some((index, pht)) => {
                    if index>EXC.0 && self.tdc.len()>index+MIN_LEN{
                        self.tdc = self.tdc.iter().cloned().skip(index-EXC.1).collect();
                    }
                    Some(pht)
                },
                None => None,
            }
        }
    }



    pub struct TempElectronData {
        pub electron: Vec<(f64, usize, usize, u16)>,
    }

    //impl Iterator for TempElectronData {
        

    impl TempElectronData {
        fn new() -> Self {
            Self {
                electron: Vec::new(),
            }
        }



        fn remove_clusters(&mut self) -> Vec<usize> {
            let mut nelist:Vec<(f64, usize, usize, u16)> = Vec::new();
            let mut cs_list: Vec<usize> = Vec::new();

            let mut last: (f64, usize, usize, u16) = self.electron[0];
            let mut cluster_vec: Vec<(f64, usize, usize, u16)> = Vec::new();
            for x in &self.electron {
                if x.0 > last.0 + CLUSTER_DET || (x.1 as isize - last.1 as isize).abs() > 2 || (x.2 as isize - last.2 as isize).abs() > 2 {
                    let cluster_size: usize = cluster_vec.len();
                    let t_mean:f64 = cluster_vec.iter().map(|&(t, _, _, _)| t).sum::<f64>() / cluster_size as f64;
                    let x_mean:usize = cluster_vec.iter().map(|&(_, x, _, _)| x).sum::<usize>() / cluster_size;
                    let y_mean:usize = cluster_vec.iter().map(|&(_, _, y, _)| y).sum::<usize>() / cluster_size;
                    let tot_mean: u16 = (cluster_vec.iter().map(|&(_, _, _, tot)| tot as usize).sum::<usize>() / cluster_size) as u16;
                    //println!("{:?} and {}", cluster_vec, t_mean);
                    nelist.push((t_mean, x_mean, y_mean, tot_mean));
                    cs_list.push(cluster_size);
                    cluster_vec = Vec::new();
                }
                last = *x;
                cluster_vec.push(*x);
            }
            self.electron = nelist;
            cs_list
        }


        fn add_electron(&mut self, my_pack: &Pack) {
            self.electron.push((my_pack.electron_time(), my_pack.x().unwrap(), my_pack.y().unwrap(), my_pack.tot()));
        }

        fn sort(&mut self) {
            self.electron.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        }
    }
            

    pub fn search_coincidence(file: &str, coinc_data: &mut ElectronData) -> io::Result<()> {
        
        let mut file = fs::File::open(file)?;
        let mut buffer:Vec<u8> = Vec::new();
        file.read_to_end(&mut buffer)?;
        
        let mytdc = TdcType::TdcTwoRisingEdge;
        let mut ci = 0;

        let mut temp_edata = TempElectronData::new();
        let mut temp_tdc = TempTdcData::new();
        
        let mut packet_chunks = buffer.chunks_exact(8);
        while let Some(pack_oct) = packet_chunks.next() {
            match pack_oct {
                &[84, 80, 88, 51, nci, _, _, _] => {ci=nci as usize;},
                _ => {
                    let packet = Pack { chip_index: ci, data: pack_oct };
                    match packet.id() {
                        6 if packet.tdc_type() == mytdc.associate_value() => {
                            temp_tdc.add_tdc(&packet);
                        },
                        11 => {
                            if let (Some(_), Some(_)) = (packet.x(), packet.y()) {
                                temp_edata.add_electron(&packet);
                            }
                        },
                        _ => {},
                    };
                },
            };
        }
        coinc_data.add_events(temp_edata, temp_tdc);
        Ok(())
    }
}

pub mod time_resolved {
    use crate::packetlib::{Packet, PacketEELS as Pack};
    use crate::tdclib::{tdcvec, TdcType};
    use std::io::prelude::*;
    use std::fs;

    #[derive(Debug)]
    pub enum ErrorType {
        OutOfBounds,
        FolderDoesNotExist,
        FolderNotCreated,
    }

    pub trait TimeTypes {
        fn add_packet(&mut self, packet: &Pack);
        fn add_tdc(&mut self, packet: &Pack);
        fn output(&self) -> Result<(), ErrorType>;
        fn display_info(&self) -> Result<(), ErrorType>;
    }

    pub struct TimeSet {
        pub set: Vec<Box<dyn TimeTypes>>,
    }

    /// This enables spectral analysis in a certain spectral window.
    pub struct TimeSpectral {
        pub spectra: Vec<[usize; 1024]>,
        pub initial_time: Option<f64>,
        pub interval: usize,
        pub counter: Vec<usize>,
        pub min: usize,
        pub max: usize,
        pub folder: String,
    }

    impl TimeTypes for TimeSpectral {
        fn add_packet(&mut self, packet: &Pack) {
            self.initial_time = match self.initial_time {
                Some(t) => {Some(t)},
                None => {Some(packet.electron_time())},
            };

            if let Some(offset) = self.initial_time {
                let vec_index = ((packet.electron_time()-offset) * 1.0e9) as usize / self.interval;
                while self.spectra.len() < vec_index+1 {
                    self.spectra.push([0; 1024]);
                    self.counter.push(0);
                }
                match packet.x() {
                    Some(x) if x>self.min && x<self.max => {
                        self.spectra[vec_index][x] += 1;
                        self.counter[vec_index] += 1;
                    },
                    _ => {},
                };
            }
        }

        fn add_tdc(&mut self, _packet: &Pack) {
        }
        
        fn output(&self) -> Result<(), ErrorType> {
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

    /// This enables spatial+spectral analysis in a certain spectral window.
    pub struct TimeSpectralSpatial {
        pub spectra: Vec<Vec<usize>>,
        pub initial_time: Option<f64>,
        pub interval: usize,
        pub counter: Vec<usize>,
        pub min: usize,
        pub max: usize,
        pub folder: String,
        pub spimx: usize,
        pub spimy: usize,
        pub tdc_start_frame: Option<f64>,
        pub tdc_period: Option<f64>,
        pub tdc_low_time: Option<f64>,
        pub tdc_timelist: Vec<(f64, TdcType)>,
        pub tdc_type: TdcType,
    }
    
    impl TimeTypes for TimeSpectralSpatial {
        fn add_packet(&mut self, packet: &Pack) {
            self.initial_time = match self.initial_time {
                Some(t) => {Some(t)},
                None => {Some(packet.electron_time())},
            };

            if let Some(offset) = self.initial_time {
                let vec_index = ((packet.electron_time()-offset) * 1.0e9) as usize / self.interval;
                while self.spectra.len() < vec_index+1 {
                    self.spectra.push(vec![0; self.spimx*self.spimy]);
                    //self.spectra.push(vec![0; self.spimx*self.spimy*1024]);
                    self.counter.push(0);
                }
                match (packet.x(), self.spim_detector(packet.electron_time())) {
                    (Some(x), Some(array_pos)) if x>self.min && x<self.max => {
                        self.spectra[vec_index][array_pos] += 1;
                        self.counter[vec_index] += 1;
                    },
                    _ => {},
                };
            }
        }

        fn add_tdc(&mut self, packet: &Pack) {
            if let ( None, None, None, Some(tdc_found) ) = ( self.tdc_start_frame, self.tdc_period, self.tdc_low_time, TdcType::associate_value_to_enum(packet.tdc_type()) ) {
                self.tdc_timelist.push( (packet.tdc_time_norm(), tdc_found) );
                if tdcvec::check_tdc(&self.tdc_timelist, &self.tdc_type) {
                    self.tdc_start_frame = Some(tdcvec::get_begintime(&self.tdc_timelist, &self.tdc_type));
                    self.tdc_period = Some(tdcvec::find_period(&self.tdc_timelist, &self.tdc_type));
                    let tdc_high_time = tdcvec::find_high_time(&self.tdc_timelist, &self.tdc_type);
                    self.tdc_low_time = Some(self.tdc_period.unwrap() - tdc_high_time);
                    println!("Start frame (us) is {:?}. Period (us) is {:?} and low time (us) is {:?}", self.tdc_start_frame.unwrap()*1e6, self.tdc_period.unwrap()*1e6, self.tdc_low_time.unwrap()*1e6);
                }
            }

            if packet.tdc_type() == self.tdc_type.associate_value() {
                if (packet.tdc_counter() as usize / 2) % self.spimy == 0 {self.tdc_start_frame = Some(packet.tdc_time_norm());};
            }

        }

        fn output(&self) -> Result<(), ErrorType> {
            if let Err(_) = fs::read_dir(&self.folder) {
                if let Err(_) = fs::create_dir(&self.folder) {
                    return Err(ErrorType::FolderNotCreated);
                }
            }
            
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
            println!("Total number of spims are: {}. Total number of electrons are: {}. Electrons / spim are {}. First electron detected at {:?}. TDC period (us) is {}. TDC low time (us) is {}.", self.spectra.len(), number, number / self.spectra.len(), self.initial_time, self.tdc_period.unwrap()*1e6, self.tdc_low_time.unwrap()*1e6);
            Ok(())
        }
    }
    
    impl TimeSpectralSpatial {

        pub fn new(interval: usize, xmin: usize, xmax: usize, spimx: usize, spimy: usize, tdc_type: TdcType, folder: String) -> Result<Self, ErrorType> {
            if xmax>1024 {return Err(ErrorType::OutOfBounds)}
            Ok(Self {
                spectra: Vec::new(),
                interval: interval,
                counter: Vec::new(),
                initial_time: None,
                min: xmin,
                max: xmax,
                spimx: spimx,
                spimy: spimy,
                folder: folder,
                tdc_start_frame: None,
                tdc_period: None,
                tdc_low_time: None,
                tdc_timelist: Vec::new(),
                tdc_type: tdc_type,
            })
        }

        fn spim_detector(&self, ele_time: f64) -> Option<usize> {
            if let (Some(begin), Some(interval), Some(period)) = (self.tdc_start_frame, self.tdc_low_time, self.tdc_period) {
                let ratio = (ele_time - begin) / period;
                let ratio_inline = ratio.fract();
                if ratio_inline > interval / period || ratio_inline.is_sign_negative() {
                    None
                } else {
                    let line = ratio as usize % self.spimy;
                    let xpos = (self.spimx as f64 * ratio_inline / (interval / period)) as usize;
                    let result = (line * self.spimx + xpos);
                    //let result = (line * self.spimx + xpos) * 1024;
                    Some(result)
                }
            } else {None}
        }
    }



    pub fn analyze_data(file: &str, data: &mut TimeSet) {
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

    #[cfg(test)]
    mod tests {
        #[test]
        fn it_works() {
            assert_eq!(2+2, 4);
        }
    }
}

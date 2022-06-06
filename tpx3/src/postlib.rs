pub mod coincidence {

    use crate::spimlib::SPIM_PIXELS;
    use crate::packetlib::{Packet, TimeCorrectedPacketEELS as Pack};
    use crate::tdclib::{TdcControl, TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
    use std::io;
    use std::io::prelude::*;
    use std::fs;
    use std::time::Instant;
    use crate::clusterlib::cluster::{SingleElectron, CollectionElectron};
    use crate::auxiliar::ConfigAcquisition;
    use std::convert::TryInto;
    use std::cmp;
    use crate::auxiliar::value_types::*;

    const TIME_WIDTH: TIME = 16; //Time width to correlate (in units of 640 Mhz, or 1.5625 ns).
    //const TIME_DELAY: usize = 100_000 - 1867; //Time delay to correlate (ps).
    const TIME_DELAY: TIME = 103; // + 50_000; //Time delay to correlate (in units of 640 Mhz, or 1.5625 ns).
    const MIN_LEN: usize = 100; // Sliding time window size.

    pub struct ElectronData {
        pub time: Vec<TIME>,
        pub rel_time: Vec<isize>,
        pub x: Vec<POSITION>,
        pub y: Vec<POSITION>,
        pub tot: Vec<u16>,
        pub cluster_size: Vec<usize>,
        pub spectrum: Vec<usize>,
        pub corr_spectrum: Vec<usize>,
        pub is_spim: bool,
        pub spim_size: (POSITION, POSITION),
        //pub begin_frame: Option<usize>,
        pub spim_index: Vec<u32>,
        pub spim_tdc: Option<PeriodicTdcRef>,
    }

    impl ElectronData {
        fn add_electron(&mut self, val: SingleElectron) {
            self.spectrum[val.image_index()] += 1;
        }

        fn add_spim_line(&mut self, pack: &Pack) {
            //match &mut self.spim_tdc {
            //    Some(spim_tdc) => {
            //        spim_tdc.ticks_to_frame = None;
            //    },
            //    _ => {},
            //};
            
            if let Some(spim_tdc) = &mut self.spim_tdc {
                spim_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
            }
        }

        fn add_coincident_electron(&mut self, val: SingleElectron, photon_time: TIME) {
            self.corr_spectrum[val.image_index()] += 1; //Adding the electron
            self.corr_spectrum[SPIM_PIXELS as usize-1] += 1; //Adding the photon
            self.time.push(val.time());
            self.rel_time.push(val.relative_time(photon_time));
            self.x.push(val.x());
            self.y.push(val.y());
            if let Some(index) = val.get_or_not_spim_index(self.spim_tdc, self.spim_size.0, self.spim_size.1) {
                self.spim_index.push(index);
            }
        }
        
        fn add_events(&mut self, mut temp_edata: TempElectronData, mut temp_tdc: TempTdcData) {
            temp_tdc.sort();
            let nphotons = temp_tdc.tdc.len();
            println!("Supplementary events: {}.", nphotons);
            
            temp_edata.electron.clean();

            self.spectrum[SPIM_PIXELS as usize-1]=nphotons; //Adding photons to the last pixel

            for val in temp_edata.electron.values() {
                self.add_electron(*val);
                if let Some(pht) = temp_tdc.check(*val) {
                    self.add_coincident_electron(*val, pht);
                }
            };

            println!("Number of coincident electrons: {:?}. Last photon real time is {:?}. Last relative time is {:?}.", self.x.len(), self.time.iter().last(), self.rel_time.iter().last());
        }

        fn prepare_spim(&mut self, spim_tdc: PeriodicTdcRef) {
            assert!(self.is_spim);
            self.spim_tdc = Some(spim_tdc);
        }

        pub fn new(my_config: &ConfigAcquisition) -> Self {
            Self {
                time: Vec::new(),
                rel_time: Vec::new(),
                x: Vec::new(),
                y: Vec::new(),
                tot: Vec::new(),
                cluster_size: Vec::new(),
                spectrum: vec![0; SPIM_PIXELS as usize*256],
                corr_spectrum: vec![0; SPIM_PIXELS as usize*256],
                is_spim: my_config.is_spim,
                spim_size: (my_config.xspim, my_config.yspim),
                spim_index: Vec::new(),
                spim_tdc: None,
            }
        }
        
        pub fn output_corr_spectrum(&self, bin: bool) {
            let out: String = match bin {
                true => {
                    let mut spec: Vec<usize> = vec![0; SPIM_PIXELS as usize];
                    for val in self.corr_spectrum.chunks_exact(SPIM_PIXELS as usize) {
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
                    let mut spec: Vec<usize> = vec![0; SPIM_PIXELS as usize];
                    for val in self.spectrum.chunks_exact(SPIM_PIXELS as usize) {
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
        pub tdc: Vec<TIME>,
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

        fn check(&mut self, value: SingleElectron) -> Option<TIME> {

            let array_length = self.tdc.len();
            let max_index = cmp::min(self.min_index + MIN_LEN, array_length);
            
            let result = self.tdc[self.min_index..max_index].iter()
                .enumerate()
                .find(|(_, x)| ((**x as isize - value.time() as isize).abs() as TIME) < TIME_WIDTH);
            
            //Index must be greater than 10% of MIN_LEN, so first photons do not count.
            //Effective size must be greater or equal than MIN_LEN otherwise a smaller array is
            //iterated.
            match result {
                Some((index, pht_value)) => {
                    //if index > MIN_LEN/10 && array_length>self.min_index + MIN_LEN + index {
                    if index > MIN_LEN/10 && (max_index - self.min_index) >= MIN_LEN {
                       self.min_index += index/2;
                    }
                    Some(*pht_value)
                },
                None => None,
            }
        }
    }

    pub struct TempElectronData {
        pub electron: CollectionElectron, //Time, X, Y and ToT and Time difference (for Spim positioning)
        pub min_index: usize,
    }

    impl TempElectronData {
        fn new() -> Self {
            Self {
                electron: CollectionElectron::new(),
                min_index: 0,
            }
        }
    }

            

    pub fn search_coincidence(file: &str, coinc_data: &mut ElectronData) -> io::Result<()> {
        
        let mut file0 = fs::File::open(file)?;
        
        let spim_tdc: Box<dyn TdcControl> = if coinc_data.is_spim {
            if coinc_data.spim_size.0 == 0 || coinc_data.spim_size.1 == 0 {
                panic!("Spim mode is on. X and Y pixels must be greater than 0.");
            }
            let temp = PeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut file0, Some(coinc_data.spim_size.1)).expect("Could not create period TDC reference.");
            coinc_data.prepare_spim(temp);
            Box::new(temp)
        } else {
            Box::new(NonPeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut file0, None).expect("Could not create non periodic TDC reference."))
        };
        let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoRisingEdge, &mut file0, None).expect("Could not create non periodic (photon) TDC reference.");

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
                    [84, 80, 88, 51, nci, _, _, _] => {ci=nci;},
                    _ => {
                        let packet = Pack { chip_index: ci, data: pack_oct.try_into().unwrap() };
                        match packet.id() {
                            6 if packet.tdc_type() == np_tdc.id() => {
                                temp_tdc.add_tdc(&packet);
                            },
                            6 if packet.tdc_type() == spim_tdc.id() => {
                                coinc_data.add_spim_line(&packet);
                            },
                            11 => {
                                let se = SingleElectron::new(&packet, coinc_data.spim_tdc, 0);
                                temp_edata.electron.add_electron(se);
                            },
                            _ => {}, //println!("{}", packet.tdc_type());},
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

pub mod ntime_resolved {
    use crate::spimlib::SPIM_PIXELS;
    use crate::packetlib::{Packet, PacketEELS as Pack};
    use crate::tdclib::{TdcControl, TdcType, PeriodicTdcRef};
    use std::io::prelude::*;
    use crate::clusterlib::cluster::{SingleElectron, CollectionElectron};
    use std::convert::TryInto;
    use std::fs;
    use crate::auxiliar::value_types::*;

    #[derive(Debug)]
    pub enum ErrorType {
        OutOfBounds,
        FolderDoesNotExist,
        FolderNotCreated,
        ScanOutofBounds,
        MinGreaterThanMax,
    }

    pub trait TimeTypes {
        fn prepare(&mut self, file: &mut fs::File);
        fn add_electron(&mut self, packet: &Pack);
        fn add_tdc(&mut self, packet: &Pack);
        fn process(&mut self) -> Result<(), ErrorType>;
        fn output(&mut self, how_many: usize) -> Result<(), ErrorType>;
        fn display_info(&self) -> Result<(), ErrorType>;
    }

    pub struct TimeSet {
        pub set: Vec<Box<dyn TimeTypes>>,
    }

    /// This enables spatial+spectral analysis in a certain spectral window.
    pub struct TimeSpectralSpatial {
        pub spectra: Vec<Vec<usize>>, //Main data,
        pub ensemble: CollectionElectron, //A collection of single electrons,
        pub folder: String, //Folder in which data will be saved,
        pub spimx: POSITION, //The horinzontal axis of the spim,
        pub spimy: POSITION, //The vertical axis of the spim,
        pub tdc_periodic: Option<PeriodicTdcRef>, //The periodic tdc. Can be none if xspim and yspim <= 1,
        pub tdc_type: TdcType, //The tdc type for the spim,
        pub remove_clusters: bool,
        pub frame_int: COUNTER,
        pub slice: COUNTER,
    }
    
    impl TimeTypes for TimeSpectralSpatial {
        fn prepare(&mut self, file: &mut fs::File) {
            self.tdc_periodic = match self.tdc_periodic {
                None if self.spimx>1 && self.spimy>1 => {
                    let val = Some(PeriodicTdcRef::new(self.tdc_type.clone(), file, Some(self.spimy)).expect("Problem in creating periodic tdc ref."));
                    val
                },
                Some(val) => Some(val),
                _ => None,
            };
        }

        fn add_electron(&mut self, packet: &Pack) {
            //Getting Initial Time
            let mut vec_index;
            if let Some(spim_tdc) = self.tdc_periodic {
                vec_index = spim_tdc.frame();
            } else {
                vec_index = 0;
            }
            vec_index = vec_index / self.frame_int;

            //Creating the array using the electron corrected time. Note that you dont need to use it in the 'spim_detector' if you synchronize the clocks.
            while self.spectra.len() < (vec_index - self.slice + 1).try_into().unwrap() {
                self.expand_data();
            }
            
            let se = SingleElectron::new(packet, self.tdc_periodic, vec_index);
            self.ensemble.add_electron(se);
        }

        fn add_tdc(&mut self, packet: &Pack) {
            //Synchronizing clocks using two different approaches. It is always better to use a multiple of 2 and use the FPGA counter.
            match &mut self.tdc_periodic {
                Some(my_tdc_periodic) if packet.tdc_type() == self.tdc_type.associate_value() => {
                    my_tdc_periodic.upt(packet.tdc_time_norm(), packet.tdc_counter());
                },
                _ => {},
            };
        }

        fn process(&mut self) -> Result<(), ErrorType> {
            //self.ensemble.output_data(String::from("entire_data"), 2);
            if self.ensemble.try_clean(0, self.remove_clusters) {
                //self.ensemble.output_data(String::from("entire_data_cluster"), 2);
                let mut max_slice: Option<usize> = None;
                let mut min_slice: Option<usize> = None;
                
                for val in self.ensemble.values() {
                    if let Some(index) = val.get_or_not_spim_index(self.tdc_periodic, self.spimx, self.spimy) {
                        self.spectra[val.spim_slice()-self.slice][index as usize] += 1;
                        
                        max_slice = match max_slice {
                            None => Some(val.spim_slice()),
                            Some(k) if val.spim_slice() >= k => Some(val.spim_slice()),
                            Some(k) => Some(k),
                        };
                        min_slice = match min_slice {
                            None => Some(val.spim_slice()),
                            Some(k) if val.spim_slice() <= k => Some(val.spim_slice()),
                            Some(k) => Some(k),
                        };
                    }
                }
                println!("{:?} and {:?} and {}", max_slice, min_slice, self.spectra.len());
                self.output(max_slice.unwrap() - min_slice.unwrap())?;
                self.ensemble = CollectionElectron::new();
            }
            Ok(())
        }

        fn output(&mut self, how_many: usize) -> Result<(), ErrorType> {

            if let Err(_) = fs::read_dir(&self.folder) {
                if let Err(_) = fs::create_dir(&self.folder) {
                    return Err(ErrorType::FolderNotCreated);
                }
            }

            let mut folder: String = String::from(&self.folder);
            folder.push_str("\\");
            folder.push_str(&(self.spimx).to_string());
            folder.push_str("_");
            folder.push_str(&(self.spimy).to_string());

            folder.push_str("_SparseSpimComplete");

            //println!("{} and {} and {}", self.slice, self.spectra.len(), how_many);
            for _ in 0..how_many {
                let slice_string = String::from(self.slice.to_string());
                let hit_string = String::from("_Hits");
                self.slice += 1;
                let temp_spec = self.spectra.remove(0);
                let out = temp_spec.iter()
                    .enumerate()
                    .filter(|(_index, hits)| **hits != 0)
                    .map(|(index, _hits)| index.to_string())
                    .collect::<Vec<String>>().join(",");
                if let Err(_) = fs::write(folder.clone()+&slice_string, out) {
                    return Err(ErrorType::FolderDoesNotExist);
                }
                let out = temp_spec.iter()
                    .enumerate()
                    .filter(|(_index, hits)| **hits != 0)
                    .map(|(_index, hits)| hits.to_string())
                    .collect::<Vec<String>>().join(",");
                if let Err(_) = fs::write(folder.clone()+&hit_string+&slice_string, out) {
                    return Err(ErrorType::FolderDoesNotExist);
                }
            }
            //println!("{} and {}", self.slice, self.spectra.len());
            Ok(())
        }
        /*
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
            folder.push_str(&(self.spimx).to_string());
            folder.push_str("_");
            folder.push_str(&(self.spimy).to_string());

            
            //Check if sparse or not is better to output;
            let test_slice = self.spectra.len() / 2;

            let out1 = self.spectra[test_slice].iter()
                .map(|x| x.to_string())
                .collect::<Vec<String>>().join(",")
                .len();
            
            let out2_indices = self.spectra[test_slice].iter()
                .enumerate()
                .filter(|(_index, hits)| **hits != 0)
                .map(|(index, _hits)| index.to_string())
                .collect::<Vec<String>>().join(",")
                .len();
            
            let out2_hits = self.spectra[test_slice].iter()
                .enumerate()
                .filter(|(_index, hits)| **hits != 0)
                .map(|(_index, hits)| hits.to_string())
                .collect::<Vec<String>>().join(",")
                .len();

            println!("Estimated size, per slice, for normal output: {}. Estimated size, per slice, for sparse-output: {}.", out1, out2_indices+out2_hits);

            match out1 < out2_indices+out2_hits {
                true => {
                    println!("Normal hyperspectral output automatically selected.");
                    folder.push_str("_SpimComplete");
                    for slice in 0..self.spectra.len() {
                        let slice_string = String::from(slice.to_string());
                        let out = self.spectra[slice].iter()
                            .map(|x| x.to_string())
                            .collect::<Vec<String>>().join(",");
                        if let Err(_) = fs::write(folder.clone()+&slice_string, out) {
                            return Err(ErrorType::FolderDoesNotExist);
                        }
                    }
                },
                false => {
                    println!("Sparse-output hyperspectral output automatically selected.");
                    folder.push_str("_SparseSpimComplete");
                    for slice in 0..self.spectra.len() {
                        let slice_string = String::from(slice.to_string());
                        let hit_string = String::from("_Hits");
                        let out = self.spectra[slice].iter()
                            .enumerate()
                            .filter(|(_index, hits)| **hits != 0)
                            .map(|(index, _hits)| index.to_string())
                            .collect::<Vec<String>>().join(",");
                        if let Err(_) = fs::write(folder.clone()+&slice_string, out) {
                            return Err(ErrorType::FolderDoesNotExist);
                        }
                        let out = self.spectra[slice].iter()
                            .enumerate()
                            .filter(|(_index, hits)| **hits != 0)
                            .map(|(_index, hits)| hits.to_string())
                            .collect::<Vec<String>>().join(",");
                        if let Err(_) = fs::write(folder.clone()+&hit_string+&slice_string, out) {
                            return Err(ErrorType::FolderDoesNotExist);
                        }
                    }
                },
            };
            Ok(())
        }
*/

            
        fn display_info(&self) -> Result<(), ErrorType> {
            println!("Total number of spims are: {}. TDC info is {:?}.", self.spectra.len(), self.tdc_periodic);
            Ok(())
        }
    }
    
    impl TimeSpectralSpatial {
        pub fn new(frame_int: usize, spimx: usize, spimy: usize, remove_clusters: bool, tdc_type: TdcType, folder: String) -> Result<Self, ErrorType> {

            Ok(Self {
                spectra: Vec::new(),
                ensemble: CollectionElectron::new(),
                spimx: spimx,
                spimy: spimy,
                folder: folder,
                tdc_periodic: None,
                tdc_type: tdc_type,
                remove_clusters: remove_clusters,
                frame_int: frame_int,
                slice: 0,
            })
        }
        
        fn expand_data(&mut self) {
            self.spectra.push(vec![0; self.spimx*self.spimy*SPIM_PIXELS]);
        }
    }

    pub fn analyze_data(file: &str, data: &mut TimeSet) {
        for each in data.set.iter_mut() {
            let mut file = fs::File::open(file).expect("Could not open desired file.");
            each.prepare(&mut file);
        }


        let mut my_file = fs::File::open(file).expect("Could not open desired file.");
        let mut buffer: Vec<u8> = vec![0; 128_000_000];

        let mut total_size = 0;
        let mut ci = 0;

        while let Ok(size) = my_file.read(&mut buffer) {
            if size==0 {break;}
            total_size += size;
            buffer[0..size].chunks_exact(8).for_each(|pack_oct| {
                match pack_oct {
                    &[84, 80, 88, 51, nci, _, _, _] => {ci = nci},
                    _ => {
                        let packet = Pack{chip_index: ci, data: pack_oct.try_into().unwrap()};
                        match packet.id() {
                            6 => {
                                for each in data.set.iter_mut() {
                                    each.add_tdc(&packet);
                                }
                            },
                            11 => {
                                for each in data.set.iter_mut() {
                                    each.add_electron(&packet);
                                }
                            },
                            _ => {},
                        };
                    },
                };
            });
            for each in data.set.iter_mut() {
                each.process().expect("Error in processing");
            }
            println!("File: {:?}. Total number of bytes read (MB): ~ {}", file, total_size/1_000_000);
        };
    }
}

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

    const TIME_WIDTH: TIME = 40; //Time width to correlate (in units of 640 Mhz, or 1.5625 ns).
    const TIME_DELAY: TIME = 104; // + 50_000; //Time delay to correlate (in units of 640 Mhz, or 1.5625 ns).
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
        pub spim_index: Vec<POSITION>,
        pub spim_tdc: Option<PeriodicTdcRef>,
        remove_clusters: bool,
    }

    impl ElectronData {
        fn add_electron(&mut self, val: SingleElectron) {
            self.spectrum[val.x() as usize] += 1;
            //self.spectrum[val.image_index() as usize] += 1;
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
            //self.corr_spectrum[val.image_index() as usize] += 1; //Adding the electron
            self.corr_spectrum[val.x() as usize] += 1; //Adding the electron
            self.corr_spectrum[SPIM_PIXELS as usize-1] += 1; //Adding the photon
            self.time.push(val.time());
            self.rel_time.push(val.relative_time_from_abs_tdc(photon_time));
            self.x.push(val.x());
            self.y.push(val.y());
            self.tot.push(val.tot());
            if let Some(index) = val.get_or_not_spim_index(self.spim_tdc, self.spim_size.0, self.spim_size.1) {
                self.spim_index.push(index);
            }
        }
        
        fn add_events(&mut self, mut temp_edata: TempElectronData, mut temp_tdc: TempTdcData) {
            temp_tdc.sort();
            let nphotons = temp_tdc.tdc.len();
            println!("Supplementary events: {}.", nphotons);
            
            //if self.remove_clusters {temp_edata.electron.clean();}
            temp_edata.electron.try_clean(0, self.remove_clusters);

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
                spectrum: vec![0; SPIM_PIXELS as usize*1],
                corr_spectrum: vec![0; SPIM_PIXELS as usize*1],
                is_spim: my_config.is_spim,
                spim_size: (my_config.xspim, my_config.yspim),
                spim_index: Vec::new(),
                spim_tdc: None,
                remove_clusters: my_config.remove_cluster,
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
            self.tdc.push(my_pack.tdc_time_abs_norm() - TIME_DELAY * 6);
        }

        fn sort(&mut self) {
            self.tdc.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        }

        fn check(&mut self, value: SingleElectron) -> Option<TIME> {

            let array_length = self.tdc.len();
            let max_index = cmp::min(self.min_index + MIN_LEN, array_length);
            
            let result = self.tdc[self.min_index..max_index].iter()
                .enumerate()
                .find(|(_, x)| (((**x/6) as isize - value.time() as isize).abs() as TIME) < TIME_WIDTH);
            
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
                                let se = SingleElectron::new(&packet, coinc_data.spim_tdc);
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
    use std::fs::OpenOptions;
    use crate::spimlib::SPIM_PIXELS;
    use crate::packetlib::{Packet, PacketEELS as Pack};
    use crate::tdclib::{TdcControl, TdcType, PeriodicTdcRef};
    use std::io::prelude::*;
    use crate::clusterlib::cluster::{SingleElectron, CollectionElectron};
    use std::convert::TryInto;
    use std::time::Instant;
    use std::fs;
    use crate::auxiliar::{value_types::*, ConfigAcquisition};

    #[derive(Debug)]
    pub enum ErrorType {
        OutOfBounds,
        FolderDoesNotExist,
        FolderNotCreated,
        ScanOutofBounds,
        MinGreaterThanMax,
    }

    /// This enables spatial+spectral analysis in a certain spectral window.
    pub struct TimeSpectralSpatial {
        spectra: Vec<POSITION>, //Main data,
        indices: Vec<u16>,
        ensemble: CollectionElectron, //A collection of single electrons,
        spimx: POSITION, //The horinzontal axis of the spim,
        spimy: POSITION, //The vertical axis of the spim,
        tdc_periodic: Option<PeriodicTdcRef>, //The periodic tdc. Can be none if xspim and yspim <= 1,
        tdc_type: TdcType, //The tdc type for the spim,
        remove_clusters: bool,
    }

    fn as_bytes<T>(v: &[T]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u8,
                v.len() * std::mem::size_of::<T>())
        }
    }
    
    impl TimeSpectralSpatial {
        fn prepare(&mut self, file: &mut fs::File) {
            self.tdc_periodic = match self.tdc_periodic {
                None if self.spimx>1 && self.spimy>1 => {
                    Some(PeriodicTdcRef::new(self.tdc_type.clone(), file, Some(self.spimy)).expect("Problem in creating periodic tdc ref."))
                },
                Some(val) => Some(val),
                _ => None,
            };
        }

        fn add_electron(&mut self, packet: &Pack) {
            //Getting Initial Time
            //let vec_index;
            //if let Some(spim_tdc) = self.tdc_periodic {
            //    vec_index = spim_tdc.frame();
            //} else {
            //    vec_index = 0;
            //}

            let se = SingleElectron::new(packet, self.tdc_periodic);
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
            if self.ensemble.try_clean(0, self.remove_clusters) {
                for val in self.ensemble.values() {
                    if let Some(index) = val.get_or_not_spim_index(self.tdc_periodic, self.spimx, self.spimy) {
                        self.spectra.push(index);
                        self.indices.push((val.spim_slice()).try_into().expect("Exceeded the maximum number of indices"));
                    }
            }
            self.ensemble.clear();
            let mut tfile = OpenOptions::new()
                .append(true)
                .create(true)
                .open("si_complete.txt").expect("Could not output time histogram.");
            tfile.write_all(as_bytes(&self.spectra)).expect("Could not write time to file.");
            let mut tfile2 = OpenOptions::new()
                .append(true)
                .create(true)
                .open("si_complete_indices.txt").expect("Could not output time histogram.");
            tfile2.write_all(as_bytes(&self.indices)).expect("Could not write time to indices file.");
            self.spectra.clear();
            self.indices.clear();
            }
            Ok(())
        }
            
        pub fn new(my_config: &ConfigAcquisition) -> Result<Self, ErrorType> {

            Ok(Self {
                spectra: Vec::new(),
                indices: Vec::new(),
                ensemble: CollectionElectron::new(),
                spimx: my_config.xspim,
                spimy: my_config.yspim,
                tdc_periodic: None,
                tdc_type: TdcType::TdcOneFallingEdge,
                remove_clusters: my_config.remove_cluster,
            })
        }
    }

    pub fn analyze_data(file: &str, data: &mut TimeSpectralSpatial) {
        let mut prepare_file = fs::File::open(file).expect("Could not open desired file.");
        data.prepare(&mut prepare_file);
        
        let start = Instant::now();
        let mut my_file = fs::File::open(file).expect("Could not open desired file.");
        let mut buffer: Vec<u8> = vec![0; 1_000_000_000];

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
                                data.add_tdc(&packet);
                            },
                            11 => {
                                data.add_electron(&packet);
                            },
                            _ => {},
                        };
                    },
                };
            });
            data.process().expect("Error in processing");
            println!("File: {:?}. Total number of bytes read (MB): ~ {}", file, total_size/1_000_000);
            println!("Time elapsed: {:?}", start.elapsed());
        };
    }
}

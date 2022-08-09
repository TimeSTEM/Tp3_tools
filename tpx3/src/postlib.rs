pub mod coincidence {

    use std::fs::OpenOptions;
    use crate::spimlib::SPIM_PIXELS;
    use crate::packetlib::{Packet, TimeCorrectedPacketEELS as Pack};
    use crate::tdclib::{TdcControl, TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
    use crate::postlib::isi_box;
    use std::io;
    use std::io::prelude::*;
    use std::fs;
    use std::time::Instant;
    use crate::clusterlib::cluster::{SingleElectron, CollectionElectron};
    use crate::auxiliar::ConfigAcquisition;
    use std::convert::TryInto;
    use crate::auxiliar::value_types::*;
    use rayon::prelude::*;
    use indicatif::{ProgressBar, ProgressStyle};

    const TIME_WIDTH: TIME = 40; //Time width to correlate (in units of 640 Mhz, or 1.5625 ns).
    const TIME_DELAY: TIME = 104; // + 50_000; //Time delay to correlate (in units of 640 Mhz, or 1.5625 ns).
    const ISI_BUFFER_SIZE: usize = 512_000_000; //Buffer size reading files when using TP3 and IsiBox
    
    fn as_bytes<T>(v: &[T]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u8,
                v.len() * std::mem::size_of::<T>())
        }
    }

    pub struct ElectronData {
        time: Vec<TIME>,
        channel: Vec<COUNTER>,
        rel_time: Vec<i64>,
        g2_time: Vec<Option<i64>>,
        x: Vec<POSITION>,
        y: Vec<POSITION>,
        tot: Vec<u16>,
        cluster_size: Vec<usize>,
        spectrum: Vec<usize>,
        corr_spectrum: Vec<usize>,
        is_spim: bool,
        spim_size: (POSITION, POSITION),
        spim_index: Vec<POSITION>,
        spim_tdc: Option<PeriodicTdcRef>,
        remove_clusters: bool,
        overflow_electrons: COUNTER,
    }

    impl ElectronData {
        fn add_electron(&mut self, val: SingleElectron) {
            self.spectrum[val.x() as usize] += 1;
        }

        fn add_spim_line(&mut self, pack: &Pack) {
            if let Some(spim_tdc) = &mut self.spim_tdc {
                spim_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
            }
        }

        fn estimate_overflow(&self, pack: &Pack) -> Option<TIME> {
            if let Some(spim_tdc) = self.spim_tdc {
                let val = spim_tdc.estimate_time();
                if val > pack.tdc_time() + Pack::electron_overflow() {
                    return Some(val / Pack::tdc_overflow());
                }
                else {return Some(0)}
            }
            None
        }


        fn add_coincident_electron(&mut self, val: SingleElectron, photon: (TIME, COUNTER, Option<i64>)) {
            self.corr_spectrum[val.x() as usize] += 1; //Adding the electron
            self.corr_spectrum[SPIM_PIXELS as usize-1] += 1; //Adding the photon
            self.time.push(val.time());
            self.g2_time.push(photon.2);
            self.tot.push(val.tot());
            self.x.push(val.x());
            self.y.push(val.y());
            self.channel.push(photon.1);
            self.rel_time.push(val.relative_time_from_abs_tdc(photon.0));
            //self.rel_time.push(val.relative_time(photon.0));
            if let Some(index) = val.get_or_not_spim_index(self.spim_tdc, self.spim_size.0, self.spim_size.1) {
                self.spim_index.push(index);
            }
        }
        
        fn add_events(&mut self, mut temp_edata: TempElectronData, temp_tdc: &mut TempTdcData, time_delay: TIME, time_width: TIME) {
            let ntotal = temp_tdc.tdc.len();
            let nphotons = temp_tdc.tdc.iter().
                filter(|(time, channel, dt)| *channel != 16 && *channel != 24).
                count();
            let mut min_index = temp_tdc.min_index;
            //println!("Total supplementary events: {}. Photons: {}. Minimum size of the array: {}.", ntotal, nphotons, min_index);

            let test = match temp_tdc.tdc_type {
                TempTdcDataType::FromTP3 => {
                    temp_tdc.sort();
                    if temp_edata.electron.check_if_overflow() {self.overflow_electrons += 1;}
                },
                TempTdcDataType::FromIsiBox => {
                    if temp_edata.electron.correct_electron_time(self.overflow_electrons) {self.overflow_electrons += 1;}
                },
            };

            //if temp_edata.electron.check_if_overflow() {self.overflow_electrons += 1;}
            //if temp_edata.electron.correct_electron_time(self.overflow_electrons) {self.overflow_electrons += 1;}
            temp_edata.electron.sort();
            temp_edata.electron.try_clean(0, self.remove_clusters);

            self.spectrum[SPIM_PIXELS as usize-1]=nphotons; //Adding photons to the last pixel

            let start = Instant::now();
            let mut photon_vec = temp_tdc.tdc.iter().filter(|ph| ph.1 != 16 && ph.1 != 24).collect::<Vec<_>>();
            println!("{:?}", start.elapsed());

            for val in temp_edata.electron.values() {
                self.add_electron(*val);
                let mut index = 0;
                let mut index_to_increase = None;
                //for (index, ph) in temp_tdc.tdc[min_index..].iter().enumerate().filter(|(index, ph)| ph.1 != 16 && ph.1 != 24) {
                for ph in &photon_vec[min_index..] {
                    //let dt = (ph.0/6) as i64 - val.time() as i64 - time_delay as i64;
                    //if (dt.abs() as TIME) < time_width {
                    if (((ph.0/6) < val.time() + time_delay + time_width) && (val.time() + time_delay < (ph.0/6) + time_width)) {
                        self.add_coincident_electron(*val, **ph);
                        if let None = index_to_increase {
                            index_to_increase = Some(index)
                        }
                    }
                    if ((ph.0/6) > val.time() + time_delay + 1_000) {break;}
                    index += 1;
                }
                if let Some(increase) = index_to_increase {
                    min_index += increase / 3;
                }
            }
            temp_tdc.min_index = min_index;

            //println!("Number of coincident electrons: {:?}. Last photon real time is {:?}. Last relative time is {:?}.", self.x.len(), self.time.iter().last(), self.rel_time.iter().last());
        }

        fn prepare_spim(&mut self, spim_tdc: PeriodicTdcRef) {
            assert!(self.is_spim);
            self.spim_tdc = Some(spim_tdc);
        }

        pub fn new(my_config: &ConfigAcquisition) -> Self {
            Self {
                time: Vec::new(),
                channel: Vec::new(),
                rel_time: Vec::new(),
                g2_time: Vec::new(),
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
                overflow_electrons: 0,
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
            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("tH.txt").expect("Could not output time histogram.");
            tfile.write_all(as_bytes(&self.rel_time)).expect("Could not write time to file.");
            println!("Outputting relative time under tH name. Vector len is {}", self.rel_time.len());
        }
        
        pub fn output_time(&self) {
            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("tabsH.txt").expect("Could not output time histogram.");
            tfile.write_all(as_bytes(&self.time)).expect("Could not write time to file.");
            println!("Outputting relative time under tH name. Vector len is {}", self.time.len());
        }
        
        pub fn output_g2_time(&self) {
            let vec = self.g2_time.iter().map(|x| {
                match x {
                    None => -5_000,
                    Some(x) => *x,
                }
            }).collect::<Vec<i64>>();
            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("g2tH.txt").expect("Could not output g2 time histogram.");
            tfile.write_all(as_bytes(&vec)).expect("Could not write g2 time to file.");
            println!("Outputting relative time under tH name. Vector len is {}", vec.len());
        }
        
        pub fn output_channel(&self) {
            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("channel.txt").expect("Could not output channel histogram.");
            tfile.write_all(as_bytes(&self.channel)).expect("Could not write time to file.");
            println!("Outputting relative time under tH name. Vector len is {}", self.channel.len());
        }
        
        pub fn output_dispersive(&self) {
            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("xH.txt").expect("Could not output X histogram.");
            tfile.write_all(as_bytes(&self.x)).expect("Could not write time to file.");
            println!("Outputting each dispersive value under xH name. Vector len is {}", self.x.len());
        }
        
        pub fn output_non_dispersive(&self) {
            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("yH.txt").expect("Could not output Y histogram.");
            tfile.write_all(as_bytes(&self.y)).expect("Could not write time to file.");
            println!("Outputting each non-dispersive value under yH name. Vector len is {}", self.y.len());
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

        pub fn output_tot(&self) {
            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("tot.txt").expect("Could not output time histogram.");
            tfile.write_all(as_bytes(&self.tot)).expect("Could not write time to file.");
        }
            
    }

    enum TempTdcDataType {
        FromTP3,
        FromIsiBox,
    }

    pub struct TempTdcData {
        tdc: Vec<(TIME, COUNTER, Option<i64>)>, //The absolute time, the channel and the dT
        pub min_index: usize,
        tdc_type: TempTdcDataType,
    }

    impl TempTdcData {
        fn new() -> Self {
            Self {
                tdc: Vec::new(),
                min_index: 0,
                tdc_type: TempTdcDataType::FromTP3,
            }
        }

        fn new_from_isilist(list: isi_box::IsiList) -> Self {
            let vec_list = list.get_timelist_with_tp3_tick();
            Self {
                tdc: vec_list,
                min_index: 0,
                tdc_type: TempTdcDataType::FromIsiBox,
            }
        }

        fn get_vec_len(&self) -> usize {
            self.tdc.len()
        }

        fn correct_tdc(&mut self, val: &mut IsiBoxCorrectVector) {
            self.tdc.iter_mut().zip(val.0.iter_mut()).
                filter(|((time, channel, dt), corr)| corr.is_some()).
                for_each(|((time, channel, dt), corr)| {
                *time += corr.unwrap() as usize;
                //*time = *time - (*time / (Pack::electron_overflow() * 6)) * (Pack::electron_overflow() * 6);
                *corr = Some(0);
            });
            //println!("{:?}", self.tdc.get(0..100));
        }
        
        pub fn get_sync(&self) -> Vec<(usize, TIME)> {
            let first = self.tdc.iter().
                filter(|(_time, channel, _dt)| *channel == 16).
                map(|(time, _channel, dt)| (*time)).
                next().
                unwrap();

            self.tdc.iter().
                enumerate().
                filter(|(index, (_time, channel, _dt))| *channel == 16).
                map(|(index, (time, _channel, dt))| (index, *time)).
                collect::<Vec<_>>()
        }

        fn add_tdc(&mut self, my_pack: &Pack, channel: COUNTER) {
            self.tdc.push((my_pack.tdc_time_abs_norm(), channel, None));
        }

        fn sort(&mut self) {
            self.tdc.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        }
    }

    struct IsiBoxCorrectVector(Vec<Option<TIME>>, usize);

    impl IsiBoxCorrectVector {
        #[inline]
        fn add_offset(&mut self, max_index: usize, value: TIME) {
            //self.0.iter_mut().enumerate().filter(|(index, x)| x.is_none() && *index <= max_index).for_each(|(index, x)| *x = Some(value));
            self.0[self.1..max_index+1].iter_mut().filter(|x| x.is_none()).for_each(|x| *x = Some(value));
            self.1 = max_index
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
        let mut buffer: Vec<u8> = vec![0; 512_000_000];
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
                                temp_tdc.add_tdc(&packet, 0);
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
        coinc_data.add_events(temp_edata, &mut temp_tdc, 104, 40);
        println!("Time elapsed: {:?}", start.elapsed());

        }
        println!("Total number of bytes read {}", total_size);
        Ok(())
    }
    
    pub fn correct_coincidence_isi(file1: &str, file2: &str, coinc_data: &mut ElectronData) -> (TempTdcData, usize) {
    
        //TP3 configurating TDC Ref
        let mut file0 = fs::File::open(file1).unwrap();
        let progress_size = file0.metadata().unwrap().len() as u64;
        let spim_tdc = PeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut file0, Some(coinc_data.spim_size.1)).expect("Could not create period TDC reference.");
        coinc_data.prepare_spim(spim_tdc);
        let begin_tp3_time = spim_tdc.begin_frame;
        let mut tp3_tdc_counter = 0;
    
        //IsiBox loading file & setting up synchronization
        let f = fs::File::open(file2).unwrap();
        let temp_list = isi_box::get_channel_timelist(f, coinc_data.spim_size, 8000);
        let begin_isi_time = temp_list.start_time;
        let mut temp_tdc = TempTdcData::new_from_isilist(temp_list);
        let tdc_vec = temp_tdc.get_sync();
        let isi_tdc_counter = tdc_vec.len();
        let mut tdc_iter = tdc_vec.iter();
        
        
        let bar = ProgressBar::new(progress_size);
        bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:40.white/black} {percent}% {pos:>7}/{len:7} [ETA: {eta}] {Correcting IsiBox values}")
                      .unwrap()
                      .progress_chars("##-"));
        
        let mut correct_vector = IsiBoxCorrectVector(vec![None; temp_tdc.get_vec_len()], 0);
        
        let mut offset = 0;
        let mut ci = 0;
        let mut file = fs::File::open(file1).unwrap();
        let mut buffer: Vec<u8> = vec![0; ISI_BUFFER_SIZE];
        let mut total_size = 0;
        let mut quit = false;
        
        while let Ok(size) = file.read(&mut buffer) {
            if size == 0 {break;}
            if quit {break;}
            total_size += size;
            bar.inc(ISI_BUFFER_SIZE as u64);
            //println!("Reading for correction. MB Read: {}", total_size / 1_000_000 );
            let mut temp_edata = TempElectronData::new();
            buffer[0..size].chunks_exact(8).for_each(|pack_oct| {
                match *pack_oct {
                    [84, 80, 88, 51, nci, _, _, _] => {ci=nci;},
                    _ => {
                        let packet = Pack { chip_index: ci, data: pack_oct.try_into().unwrap() };
                        match packet.id() {
                            6 if packet.tdc_type() == spim_tdc.id() => {
                                tp3_tdc_counter += 1;
                                coinc_data.add_spim_line(&packet);
                                let of = coinc_data.estimate_overflow(&packet).unwrap();
                                let isi_val = tdc_iter.next().unwrap();
                                let tdc_val = packet.tdc_time_abs() + of * Pack::tdc_overflow() * 6;
                                
                                //Sometimes the estimative time does not work, underestimating of.
                                //This tries to recover it out.
                                let t_dif = if isi_val.1 > tdc_val {
                                    let of = of + 1;
                                    let tdc_val = packet.tdc_time_abs() + of * Pack::tdc_overflow() * 6;
                                    tdc_val - isi_val.1
                                } else {
                                    tdc_val - isi_val.1
                                };
                                
                                
                                if (offset != 0) && ((t_dif > offset + 1_000) || (offset > t_dif + 1_000)) {
                                    println!("Possibly problem in acquiring TDC in both TP3 and IsiBox. Values for debug (Time difference, TDC, Isi, Packet_tdc, overflow, current offset) are: {} and {} and {} and {} and {} and {}", t_dif, tdc_val, isi_val.1, packet.tdc_time_abs(), of, offset);
                                    quit = true;
                                } else {
                                    //Note here that a bad one will be skipped but the next one
                                    //will try to fix it because the min_index of
                                    //'IsiBoxCorrectorVector' won't be setted in the bad
                                    //interaction.
                                    correct_vector.add_offset(isi_val.0, t_dif);
                                }

                                offset = t_dif;


                                
                     



                                //let t_dif = packet.tdc_time_abs() + of * Pack::tdc_overflow() * 6 - val.1;
                                //correct_vector.add_offset(val.0, packet.tdc_time_abs() + of * Pack::tdc_overflow() * 6 - val.1);
                                //correct_vector.add_offset(val.0, t_dif);
                                //if (offset != 0) || ((t_dif as i64 - offset as i64).abs() > 1_000) {
                                //    println!("{} and {} and {} and {} and {} and {}", t_dif, offset, packet.tdc_time_abs(), of, packet.tdc_time_abs() + of * Pack::tdc_overflow() * 6, val.1);
                                //}
                                //offset = t_dif;
                            },
                            11 => {},
                            _ => {}, //println!("{}", packet.tdc_type());},
                        };
                    },
                };
            });
        temp_tdc.correct_tdc(&mut correct_vector);
        }
        temp_tdc.sort();
        println!("Total number of bytes read {} and {}", tp3_tdc_counter, isi_tdc_counter);
        (temp_tdc, total_size)
    }

    pub fn search_coincidence_isi(file1: &str, file2: &str, coinc_data: &mut ElectronData) -> io::Result<()> {
    
        //TP3 configurating TDC Ref
        let mut file0 = fs::File::open(file1)?;
        let progress_size = file0.metadata().unwrap().len() as u64;
        let spim_tdc = PeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut file0, Some(coinc_data.spim_size.1)).expect("Could not create period TDC reference.");
        coinc_data.prepare_spim(spim_tdc);
        let begin_tp3_time = spim_tdc.begin_frame;
    
        let (mut temp_tdc, max_total_size) = correct_coincidence_isi(file1, file2, coinc_data);
        println!("total size that will be read {}", max_total_size);
        
        let mut ci = 0;
        let mut file = fs::File::open(file1)?;
        let mut buffer: Vec<u8> = vec![0; ISI_BUFFER_SIZE];
        let mut total_size = 0;
        let start = Instant::now();
        
        let bar = ProgressBar::new(progress_size);
        bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:40.white/black} {percent}% {pos:>7}/{len:7} [ETA: {eta}] {Searching electron photon coincidences}")
                      .unwrap()
                      .progress_chars("##-"));
        
        while let Ok(size) = file.read(&mut buffer) {
            if size == 0 {break;}
            if (total_size >= max_total_size) {break;}
            total_size += size;
            bar.inc(ISI_BUFFER_SIZE as u64);
            //println!("MB Read: {}", total_size / 1_000_000 );
            //if (total_size / 1_000_000) > 10_000 {break;}
            let mut temp_edata = TempElectronData::new();
            buffer[0..size].chunks_exact(8).for_each(|pack_oct| {
                match *pack_oct {
                    [84, 80, 88, 51, nci, _, _, _] => {ci=nci;},
                    _ => {
                        let packet = Pack { chip_index: ci, data: pack_oct.try_into().unwrap() };
                        match packet.id() {
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
        coinc_data.add_events(temp_edata, &mut temp_tdc, 83, 20);
        //println!("Time elapsed: {:?}", start.elapsed());
        }
        println!("Total number of bytes read {}", total_size);
        Ok(())
    }
}

pub mod isi_box {
    //use rand_distr::{Normal, Distribution};
    //use rand::{thread_rng};
    use std::fs::OpenOptions;
    use std::io::{Read, Write};
    use crate::spimlib::{VIDEO_TIME};
    use crate::tdclib::isi_box::CHANNELS;
    use crate::auxiliar::value_types::*;
    use indicatif::{ProgressBar, ProgressStyle};

    fn as_bytes<T>(v: &[T]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u8,
                v.len() * std::mem::size_of::<T>())
        }
    }
    
    fn as_int(v: &[u8]) -> &[u32] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u32,
                //v.len() )
                v.len() * std::mem::size_of::<u8>() / std::mem::size_of::<u32>())
        }
    }
    
    struct IsiListVec(Vec<(TIME, u32, Option<u32>, Option<u32>, Option<i64>)>);
    struct IsiListVecg2(Vec<(i64, u32, Option<u32>, Option<u32>)>);

    pub struct IsiList {
        data: IsiListVec, //Time, channel, spim index, spim frame, dT
        x: u32,
        y: u32,
        pixel_time: u32,
        pub counter: u32,
        pub overflow: u32,
        pub last_time: u32,
        pub start_time: Option<u32>,
        pub line_time: Option<u32>,
    }


    impl IsiList {
        fn increase_counter(&mut self, data: u32) {
  
            if let Some(line_time) = self.line_time {
                let mut time_dif = data as i32 - self.last_time as i32;
                
                if time_dif < 0 {
                    time_dif += 67108864;
                }


                //Modulus of the time_dif relative to the line time
                let fractional = time_dif as u32 - (time_dif as u32 / line_time) * line_time;

                //If modulus > 1% or smaller than 99% of the line time, we have found an issue
                if fractional > line_time / 100 && fractional < (line_time * 99) / 100 {
                }


                if (time_dif - line_time as i32).abs() > 10 {
                    println!("***IsiBox***: Probable line issue. Raw time is {}. Diference relative to the last time is {}. The spim frame is {:?}. Line counter is {}. Line time is {}. Last time is {}. Abs time is {}.", data, time_dif, self.spim_frame(), self.counter, line_time, self.last_time, self.get_abs_time(data));
                }
            }
            
            if data < self.last_time {self.overflow+=1;}
            self.last_time = data;
            self.counter += 1;

            //This happens at the second loop. There is no start_time in the first interaction.
            if let (Some(start_time), None) = (self.start_time, self.line_time) {
                let val = if data > start_time {
                    data - start_time
                } else {
                    data + 67108864 - start_time
                };
                println!("Line time is now: {}.", val * 120 / 1_000_000 );
                self.line_time = Some(val);
            }

            //Setting the start_time
            if let None = self.start_time {
                println!("Start time is now: {}", data );
                self.start_time = Some(data);
            };
        }

        fn get_line_low(&self) -> u32 {
            self.x * self.pixel_time
        }

        fn get_abs_time(&self, data: u32) -> TIME {
            //If data is smaller than the last line, we must add an overflow to the absolute time. However the
            //self.overflow is not controlled here, but only by the scan lines.
            if data > self.last_time {
                self.overflow as TIME * 67108864 + data as TIME
            } else { 
                (self.overflow+1) as TIME * 67108864 + data as TIME
            }
            //self.overflow as u64 * 67108864 + data as u64
            //let time2 = (self.counter-1) as u64 * self.line_time.unwrap() as u64 + self.start_time.unwrap() as u64;
        }

        fn spim_index(&self, data: u32) -> Option<u32> {
            if let Some(_) = self.line_time {

                let line = self.counter % self.y;
                let low = self.get_line_low();

                let time = if data > VIDEO_TIME as u32 * 13 + self.last_time {
                    //data - VIDEO_TIME as u32 * 13 - self.last_time
                    data - self.last_time
                } else {
                    //data + 67108864 - VIDEO_TIME as u32 * 13 - self.last_time
                    data + 67108864 - self.last_time
                };

                if time > low {return None;}
                let column = ((time as u64 * self.x as u64) / low as u64) as u32;

                let index = line * self.x + column;
                Some(index)
            } else {None}
        }

        fn spim_frame(&self) -> Option<u32> {
            if let Some(_) = self.line_time {
                let frame = self.counter / self.y;
                Some(frame)
            } else {None}
        }

        fn add_event(&mut self, channel: u32, data: u32) {
            self.data.0.push((self.get_abs_time(data), channel, self.spim_index(data), self.spim_frame(), None));
        }
        
        pub fn get_timelist_with_tp3_tick(&self) -> Vec<(TIME, COUNTER, Option<i64>)> {
            let first = self.data.0.iter().
                filter(|(_time, channel, _spim_index, _spim_frame, _dt)| *channel == 16).
                map(|(time, _channel, _spim_index, _spim_frame, _dt)| (*time * 1200 * 6) / 15625).
                next().
                unwrap();

            self.data.0.iter().
                map(|(time, channel, _spim_index, _spim_frame, dt)| (((*time * 1200 * 6) / 15625) - first, *channel, *dt)).
                //map(|(time, channel)| (time - (time / 103_079_215_104) * 103_079_215_104, channel)).
                collect::<Vec<_>>()
        }

        fn output_spim(&self) {
            let spim_vec = self.data.0.iter().
                filter(|(_time, channel, _spim_index, _spim_frame, _dt)| *channel != 16 && *channel != 24).
                filter(|(time, channel, spim_index, spim_frame, _dt)| spim_index.is_some() && spim_frame.is_some()).
                map(|(_time, channel, spim_index, _spim_frame, _dt)| spim_index.unwrap() * CHANNELS as u32 + channel).collect::<Vec<u32>>();
 
            let index_vec = self.data.0.iter().
                filter(|(_time, channel, _spim_index, _spim_frame, _dt)| *channel != 16 && *channel != 24).
                filter(|(time, channel, spim_index, spim_frame, _dt)| spim_index.is_some() && spim_frame.is_some()).
                map(|(_time, channel, _spim_index, spim_frame, _dt)| spim_frame.unwrap()).collect::<Vec<u32>>();

            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("isi_si_complete.txt").expect("Could not output SI index.");
            tfile.write_all(as_bytes(&spim_vec)).expect("Could not write time to file.");
            
            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("isi_si_complete_frame.txt").expect("Could not output SI frame.");
            tfile.write_all(as_bytes(&index_vec)).expect("Could not write time to file.");
        }

        fn search_coincidence(&mut self, ch1: u32, ch2: u32) {
            let progress_size = self.data.0.len() as u64;
            let mut vec2 = self.data.0.iter().filter(|(_time, channel, _spim_index, _spim_frame, _dt)| *channel == ch2).cloned().collect::<Vec<_>>();
            let iter1 = self.data.0.iter_mut();
                //filter(|(_time, channel, _spim_index, _spim_frame, is_corr)| *channel == ch1);
            let mut min_index = 0;
            let mut corr = 0;

            let mut new_list = IsiListVecg2(Vec::new());
            let bar = ProgressBar::new(progress_size);
            bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:40.white/black} {percent}% {pos:>7}/{len:7} [ETA: {eta}] {Searching photon coincidences}")
                          .unwrap()
                          .progress_chars("##-"));
        
            for val1 in iter1 {
                bar.inc(1);
                if val1.1 != ch1 {continue;}
                let mut index = 0;
                for val2 in &mut vec2[min_index..] {
                    let dt = val2.0 as i64 - val1.0 as i64;
                    if dt.abs() < 5_000 {

                        val1.4 = Some(dt);
                        val2.4 = Some(dt);

                        corr+=1;
                        new_list.0.push((dt, val2.1, val2.2, val2.3));
                        min_index += index / 10;
                    }
                    if dt > 100_000 {break;}
                    index += 1;
                }
            }

            self.data.0.iter_mut().
                filter(|(_time, channel, _spim_index, _spim_frame, _dt)| *channel == ch2).
                zip(vec2.iter()).
                for_each(|(ph21, ph22)| {
                    ph21.4 = ph22.4;
                    assert_eq!(ph21.1, ph22.1);
                });

            println!("***IsiBox***: Size of the second channel: {}. Number of coincidences: {}", vec2.len(), corr);
            
            let dt_vec = new_list.0.iter().
                filter(|(time, channel, spim_index, spim_frame)| spim_index.is_some() && spim_frame.is_some()).
                map(|(dtime, _channel, _spim_index, _spim_frame)| *dtime).
                collect::<Vec<i64>>();

            let spim_index_vec = new_list.0.iter().
                filter(|(time, channel, spim_index, spim_frame)| spim_index.is_some() && spim_frame.is_some()).
                map(|(_dtime, _channel, spim_index, _spim_frame)| spim_index.unwrap()).
                collect::<Vec<u32>>();

            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("isi_g2.txt").expect("Could not output time histogram.");
            tfile.write_all(as_bytes(&dt_vec)).expect("Could not write time to file.");
            
            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("isi_g2_index.txt").expect("Could not output time histogram.");
            tfile.write_all(as_bytes(&spim_index_vec)).expect("Could not write time to file.");
            
        }

        fn copy_empty(val: &Self) -> Self {
            IsiList {
                data: IsiListVec(Vec::new()),
                x: val.x,
                y: val.y,
                pixel_time: val.pixel_time,
                counter: val.counter,
                overflow: val.overflow,
                last_time: val.last_time,
                start_time: val.start_time,
                line_time: val.line_time,
            }
        }
    }

    //Pixel time must be in nanoseconds.
    pub fn get_channel_timelist<V>(mut data: V, spim_size: (POSITION, POSITION), pixel_time: TIME) -> IsiList 
        where V: Read
        {
            //let zlp = Normal::new(100.0, 25.0).unwrap();
            let mut list = IsiList{data: IsiListVec(Vec::new()), x: spim_size.0, y: spim_size.1, pixel_time: (pixel_time * 83_333 / 10_000) as u32, counter: 0, overflow: 0, last_time: 0, start_time: None, line_time: None};
            let mut buffer = [0; 256_000];
            while let Ok(size) = data.read(&mut buffer) {
                if size == 0 {println!("***IsiBox***: Finished reading file."); break;}
                buffer.chunks_exact(4).for_each( |x| {
                    let channel = (as_int(x)[0] & 0xFC000000) >> 27;
                    let time = as_int(x)[0] & 0x03FFFFFF;
                    list.add_event(channel, time);
                    if channel == 16 {
                        list.increase_counter(time);
                    }
                })
            }
            list.output_spim();
            list.search_coincidence(2, 12);
            list
        }
}

pub mod ntime_resolved {
    use std::fs::OpenOptions;
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
        spim_tdc_type: TdcType, //The tdc type for the spim,
        extra_tdc_type: TdcType, //The tdc type for the external,
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
                    Some(PeriodicTdcRef::new(self.spim_tdc_type.clone(), file, Some(self.spimy)).expect("Problem in creating periodic tdc ref."))
                },
                Some(val) => Some(val),
                _ => None,
            };
        }

        fn add_electron(&mut self, packet: &Pack) {
            let se = SingleElectron::new(packet, self.tdc_periodic);
            self.ensemble.add_electron(se);
        }

        fn add_spim_tdc(&mut self, packet: &Pack) {
            //Synchronizing clocks using two different approaches. It is always better to use a multiple of 2 and use the FPGA counter.
            match &mut self.tdc_periodic {
                //Some(my_tdc_periodic) if packet.tdc_type() == self.tdc_type.associate_value() => {
                Some(my_tdc_periodic) => {
                    my_tdc_periodic.upt(packet.tdc_time_norm(), packet.tdc_counter());
                },
                _ => {},
            };
        }
        
        fn add_extra_tdc(&mut self, _packet: &Pack) {
            //self.spectra.push(SPIM_PIXELS);
            //spimlib::get_spimindex(, dt: TIME, spim_tdc: &PeriodicTdcRef, self.spimx, self.spimy;
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
                spim_tdc_type: TdcType::TdcOneFallingEdge,
                extra_tdc_type: TdcType::TdcTwoRisingEdge,
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
                            6 if packet.tdc_type() == data.spim_tdc_type.associate_value() => {
                                data.add_spim_tdc(&packet);
                            },
                            6 if packet.tdc_type() == data.extra_tdc_type.associate_value() => {
                                data.add_extra_tdc(&packet);
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

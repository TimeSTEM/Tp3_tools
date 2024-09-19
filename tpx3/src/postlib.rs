pub mod coincidence {
    use crate::packetlib::Packet;
    use crate::tdclib::{TdcRef};
    use crate::errorlib::Tp3ErrorKind;
    use crate::clusterlib::cluster::{ClusterCorrection, CoincidenceSearcher};
    use std::io::prelude::*;
    use std::fs;
    use std::convert::TryInto;
    use crate::clusterlib::cluster::{SingleElectron, CollectionElectron, SinglePhoton, CollectionPhoton};
    use crate::auxiliar::{Settings, value_types::*, misc::{output_data, packet_change}, FileManager};
    use crate::constlib::*;
    use indicatif::{ProgressBar, ProgressStyle};

    //When we would like to have large E-PH timeoffsets, such as skipping entire line periods, the
    //difference between E-PH could not fit in i16. We fold these big numbers to fit in a i16
    //vector, and thus reducing the size of the output data
    trait FoldNumber<T>: Sized {
        fn fold(self) -> T;
    }

    impl FoldNumber<i16> for i64 {
        fn fold(self) -> i16 {
            (self % i16::MAX as i64) as i16
        }
    }

    //Non-standard data types 
    pub struct ElectronData<T> {
        reduced_raw_data: Vec<u64>,
        index_to_add_in_raw: Vec<usize>,
        coinc_electrons: CollectionElectron,
        channel: Vec<u8>,
        rel_time: Vec<i16>,
        spectrum: Vec<u32>,
        corr_spectrum: Vec<u32>,
        spim_frame: Vec<u32>,
        is_spim: bool,
        spim_size: (POSITION, POSITION),
        spim_tdc: Option<TdcRef>,
        remove_clusters: T,
        overflow_electrons: COUNTER,
        file: String,
        my_settings: Settings,
    }

    impl<T: ClusterCorrection> ElectronData<T> {
        
        //Called for all the electrons (not only coincident)
        fn add_electron(&mut self, val: SingleElectron) {
            self.spectrum[val.x() as usize] += 1;
            if self.is_spim {
                if let Some(index) = val.get_or_not_spim_index(self.spim_tdc, self.spim_size.0, self.spim_size.1) {
                    self.spim_frame[index as usize] += 1;
                }
            }
        }
        
        //Called for all the photons (not only coincident)
        fn add_photon(&mut self, val: SinglePhoton) {
            self.spectrum[PIXELS_X as usize - 1] += 1;
            if self.is_spim {
                if let Some(index) = val.get_or_not_spim_index(self.spim_tdc, self.spim_size.0, self.spim_size.1) {
                    self.spim_frame[index as usize] += 1;
                }
            }
        }

        //This adds the index of the 64-bit packet that will be afterwards added to the reduced
        //raw. We should not do on the fly as the order of the packets will not be preserved for
        //photons and electrons, for example (we would add one photon but then check later if there
        //is a correspondent electron). So we should run once and then run again for the
        //recorded indexes.
        fn add_packet_to_raw_index(&mut self, index: usize) {
            self.index_to_add_in_raw.push(index);
        }
        
        //This adds the packet to the reduced raw value and clear the index list afterwards
        fn add_packets_to_reduced_data(&mut self, buffer: &[u8]) {
            //Now we must add the concerned data to the reduced raw. We should first sort the indexes
            //that we have saved
            self.index_to_add_in_raw.sort();
            //Then we should iterate and see matching indexes to add.
            for index in self.index_to_add_in_raw.iter() {
                let value = packet_change(&buffer[index * 8..(index + 1) * 8])[0];
                self.reduced_raw_data.push(value);
            }
            self.index_to_add_in_raw.clear();
        }

        fn add_spim_line(&mut self, pack: &Packet) {
            //This must be called only if "self.spim" is Some(TdcRef). Otherwise this channel is
            //another photon
            self.spim_tdc.as_mut().expect("Inconsistence in TdcRef regarding spectral imaging.")
                .upt(pack.tdc_time_norm(), pack.tdc_counter());
        }

        fn add_coincident_electron(&mut self, val: SingleElectron, photon: SinglePhoton) {
            self.corr_spectrum[val.x() as usize] += 1; //Adding the electron
            self.corr_spectrum[PIXELS_X as usize-1] += 1; //Adding the photon
            self.channel.push(photon.channel().try_into().unwrap());
            self.rel_time.push(val.relative_time_from_abs_tdc(photon.time()).fold());
            self.coinc_electrons.add_electron(val);
        }
        
        fn add_events(&mut self, mut temp_edata: TempElectronData, temp_tdc: &mut TempTdcData, time_delay: TIME, time_width: TIME, _line_offset: i64) {
            
            let mut min_index = temp_tdc.min_index;

            //Sorting and removing clusters (if need) for electrons.
            temp_tdc.sort();
            if temp_edata.electron.check_if_overflow() {self.overflow_electrons += 1;}
            temp_edata.electron.sort();
            temp_edata.electron.try_clean(0, &self.remove_clusters);

            //Adding photons to the last pixel. We also add the photons in the spectra image.
            temp_tdc.event_list.iter().for_each(|photon| self.add_photon(*photon));

            //Adding electrons to the spectra image
            temp_edata.electron.iter().for_each(|electron| self.add_electron(*electron));

            //This effectivelly searches for coincidence.
            let (coinc_electron, coinc_photon) = temp_edata.electron.search_coincidence(&temp_tdc.event_list, &mut self.index_to_add_in_raw, &mut min_index, time_delay, time_width);
            coinc_electron.iter().zip(coinc_photon.iter()).for_each(|(ele, pho)| self.add_coincident_electron(*ele, *pho));

            /*
            //Second trial to search for coincidence. This seems to be faster but need to make sure of the result. 
            let searcher = CoincidenceSearcher::new(&mut temp_edata.electron, &mut temp_tdc.event_list, time_delay, time_width);
            for (ele, pho) in searcher {
                self.add_coincident_electron(ele, pho);
                self.add_packet_to_raw_index(ele.raw_packet_index());
            }
            */

            //Setting the new min_index in the case the photon list does not start from the
            //beginning in the next interaction.
            temp_tdc.min_index = min_index;
        }

        fn prepare_spim(&mut self, spim_tdc: TdcRef) {
            assert!(self.is_spim);
            self.spim_tdc = Some(spim_tdc);
        }

        pub fn new(file_path: String, correction_type: T, my_settings: Settings) -> Self {
            Self {
                reduced_raw_data: Vec::new(),
                index_to_add_in_raw: Vec::new(),
                coinc_electrons: CollectionElectron::new(),
                channel: Vec::new(),
                rel_time: Vec::new(),
                spim_frame: vec![0; (PIXELS_X * my_settings.xspim_size * my_settings.yspim_size) as usize],
                spectrum: vec![0; PIXELS_X as usize],
                corr_spectrum: vec![0; PIXELS_X as usize],
                is_spim: my_settings.mode == 2,
                spim_size: (my_settings.xspim_size, my_settings.yspim_size),
                spim_tdc: None,
                remove_clusters: correction_type,
                overflow_electrons: 0,
                file: file_path,
                my_settings,
            }
        }

        fn try_create_folder(&self) -> Result<(), Tp3ErrorKind> {
            let path_length = &self.file.len();
            match fs::create_dir(&self.file[..path_length - 5]) {
                Ok(_) => {Ok(())},
                Err(_) => { Err(Tp3ErrorKind::CoincidenceFolderAlreadyCreated) }
            }
        }
        
        fn output_data(&self) {
            self.output_spectrum();
            self.output_hyperspec();
            self.output_corr_spectrum();
        }
        
        fn output_corr_spectrum(&self) {
            output_data(&self.corr_spectrum, self.file.clone(), "cspec.txt");
        }
        
        fn output_spectrum(&self) {
            output_data(&self.spectrum, self.file.clone(), "spec.txt");
        }

        fn output_hyperspec(&self) {
            output_data(&self.spim_frame, self.file.clone(), "spim_frame.txt");
        }

        fn early_output_data(&mut self) {
            let packet_index: Vec<usize> = self.coinc_electrons.iter().map(|se| se.raw_packet_index()).collect::<Vec<_>>();
            
            let x: Vec<u16> = self.coinc_electrons.iter().map(|se| se.x() as u16).collect::<Vec<_>>();
            let y: Vec<u16> = self.coinc_electrons.iter().map(|se| se.y() as u16).collect::<Vec<_>>();
            let tot: Vec<u16> = self.coinc_electrons.iter().map(|se| se.tot()).collect::<Vec<_>>();
            let time: Vec<TIME> = self.coinc_electrons.iter().map(|se| se.time()).collect::<Vec<_>>();
            let cs: Vec<u16> = self.coinc_electrons.iter().map(|se| se.cluster_size()).collect::<Vec<_>>();
            let spim_index: Vec<INDEXHYPERSPEC> = self.coinc_electrons.
                iter().
                map(|se| 
                    se.get_or_not_spim_index(self.spim_tdc, self.spim_size.0, self.spim_size.1).unwrap_or(POSITION::MAX)
            ).collect::<Vec<_>>();
            
            output_data(&self.coinc_electrons, self.file.clone(), "coinc_elec.txt");
            output_data(&x, self.file.clone(), "xH.txt");
            output_data(&y, self.file.clone(), "yH.txt");
            output_data(&tot, self.file.clone(), "tot.txt");
            output_data(&time, self.file.clone(), "tabsH.txt");
            output_data(&cs, self.file.clone(), "cs.txt");
            output_data(&spim_index, self.file.clone(), "si.txt");
            self.coinc_electrons.clear();
            
            //The photon channel
            output_data(&self.channel, self.file.clone(), "channel.txt");
            self.channel.clear();
            
            //The relative time
            output_data(&self.rel_time, self.file.clone(), "tH.txt");
            self.rel_time.clear();
            
            //Unrelated with the ordered ones above
            output_data(&self.reduced_raw_data, self.file.clone(), "reduced_raw.tpx3");
            self.reduced_raw_data.clear();
        }
            
    }

    //the absolute time, the channel, the g2_dT, and the spim index;
    pub type TdcStructureData = (TIME, COUNTER, Option<i16>, Option<INDEXHYPERSPEC>);
    pub struct TempTdcData {
        event_list: CollectionPhoton,
        //tdc: Vec<TdcStructureData>,
        //clean_tdc: Vec<TdcStructureData>,
        min_index: usize,
    }

    impl TempTdcData {
        fn new() -> Self {
            Self {
                event_list: CollectionPhoton::new(),
                //tdc: Vec::new(),
                //clean_tdc: Vec::new(),
                min_index: 0,
            }
        }
        
        fn add_photon(&mut self, photon: SinglePhoton) {
            self.event_list.add_photon(photon)
        }

        fn sort(&mut self) {
            self.event_list.sort();
            //self.tdc.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
            //self.clean_tdc = self.tdc.iter().filter(|ph| ph.1 != 16 && ph.1 != 24).cloned().collect::<Vec<_>>();
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
        fn add_electron(&mut self, se: SingleElectron) {
            self.electron.add_electron(se);
        }
    }

    pub fn search_coincidence<T: ClusterCorrection>(coinc_data: &mut ElectronData<T>) -> Result<(), Tp3ErrorKind> {
        //If folder exists, the procedure does not continue.
        coinc_data.try_create_folder()?;
        
        //Opening the raw data file.
        let mut file0 = match fs::File::open(&coinc_data.file) {
            Ok(val) => val,
            Err(_) => return Err(Tp3ErrorKind::CoincidenceCantReadFile),
        };

        let progress_size = file0.metadata().unwrap().len();
        let main_tdc: TdcRef = if coinc_data.is_spim {
            if coinc_data.spim_size.0 == 0 || coinc_data.spim_size.1 == 0 {
                panic!("***Coincidence***: Spim mode is on. X and Y pixels must be greater than 0.");
            }
            let mut empty_filemanager = FileManager::new_empty();
            let temp = TdcRef::new_periodic(MAIN_TDC, &mut file0, &coinc_data.my_settings, &mut empty_filemanager).expect("Could not create period TDC reference.");
            coinc_data.prepare_spim(temp);
            temp
        } else {
            TdcRef::new_no_read(MAIN_TDC).expect("Could not create non periodic TDC reference.")
        };
        let np_tdc = TdcRef::new_no_read(SECONDARY_TDC).expect("Could not create non periodic (photon) TDC reference.");

 
        let mut ci = 0;
        let mut file = match fs::File::open(&coinc_data.file) {
            Ok(val) => val,
            Err(_) => return Err(Tp3ErrorKind::CoincidenceCantReadFile),
        };

        let mut buffer: Vec<u8> = vec![0; TP3_BUFFER_SIZE];
        let mut total_size = 0;
        
        let bar = ProgressBar::new(progress_size);
        bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:40.white/black} {percent}% {pos:>7}/{len:7} [ETA: {eta}] Searching electron photon coincidences")
                      .unwrap()
                      .progress_chars("=>-"));
        
        while let Ok(size) = file.read(&mut buffer) {
            if size == 0 {println!("Finished Reading."); break;}
            total_size += size;
            if LIMIT_READ && total_size >= LIMIT_READ_SIZE {break;}
            bar.inc(TP3_BUFFER_SIZE as u64);
            let mut temp_edata = TempElectronData::new();
            let mut temp_tdc = TempTdcData::new();
            buffer[0..size].chunks_exact(8).enumerate().for_each(|(current_raw_index, pack_oct)| {
                let packet = Packet { chip_index: ci, data: packet_change(pack_oct)[0] };
                match *pack_oct {
                    [84, 80, 88, 51, nci, _, _, _] => {
                        ci=nci;
                        coinc_data.add_packet_to_raw_index(current_raw_index);
                    },
                    _ => {
                        match packet.id() {
                            6 if packet.tdc_type() == np_tdc.id() => {
                                let photon = SinglePhoton::new(&packet, 0, coinc_data.spim_tdc, current_raw_index);
                                temp_tdc.add_photon(photon);
                                //temp_tdc.add_tdc(&packet, 0, coinc_data.spim_tdc, coinc_data.spim_size.0, coinc_data.spim_size.1);
                                coinc_data.add_packet_to_raw_index(current_raw_index);
                            },
                            6 if packet.tdc_type() == main_tdc.id() => {
                                if coinc_data.is_spim {
                                    coinc_data.add_spim_line(&packet);
                                } else { //if its not synchronized measurement, this tdc is used as a event-channel.
                                    let photon = SinglePhoton::new(&packet, 1, coinc_data.spim_tdc, current_raw_index);
                                    temp_tdc.add_photon(photon);
                                }
                                coinc_data.add_packet_to_raw_index(current_raw_index);
                            },
                            11 => {
                                let se = SingleElectron::new(&packet, coinc_data.spim_tdc, current_raw_index);
                                temp_edata.add_electron(se);
                            },
                            _ => {
                                coinc_data.add_packet_to_raw_index(current_raw_index);
                            },
                        };
                    },
                };
            });
            coinc_data.add_events(temp_edata, &mut temp_tdc, coinc_data.my_settings.time_delay, coinc_data.my_settings.time_width, 0);
            coinc_data.add_packets_to_reduced_data(&buffer);
            coinc_data.early_output_data();
        }
        println!("Total number of bytes read {}", total_size);
        coinc_data.output_data();
        Ok(())
    }
}

pub mod isi_box {
    use std::fs::OpenOptions;
    use std::io::{Read, Write};
    use crate::auxiliar::{misc::{as_int, as_bytes}, value_types::*};
    use indicatif::{ProgressBar, ProgressStyle};
    use crate::constlib::*;
    use crate::postlib::coincidence::TdcStructureData;
    
    const ISI_CHANNEL_SHIFT: [u32; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];

    /*
    fn add_overflow(data: u64, value: u64) -> u64
    {
        (data + value) % 67108864
    }
    */
    
    fn subtract_overflow(data: u64, value: u64) -> u64
    {
        if data > value {
            (data - value) % 67108864
        } else {
            (data + 67108864 - value) % 67108864
        }
    }
 
    type IsiListType = (TIME, u32, Option<u32>, Option<u32>, Option<i16>);
    struct IsiListVec(Vec<IsiListType>);
    struct IsiListVecg2(Vec<(i64, u32, Option<u32>, Option<u32>)>);

    pub struct IsiList {
        data_raw: IsiListVec, //Time, channel, spim index, spim frame, dT
        x: POSITION,
        y: POSITION,
        pixel_time: TIME,
        pub counter: u32,
        pub overflow: u32,
        last_time: u32,
        pub start_time: Option<u32>,
        line_time: Option<u32>,
        line_offset: u32,
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
                    //println!("***IsiBox***: Probable line issue. Raw time is {}. Diference relative to the last time is {}. The spim frame is {:?}. Line counter is {}. Line time is {}. Last time is {}. Abs time is {}.", data, time_dif, self.spim_frame(), self.counter, line_time, self.last_time, self.get_abs_time(data));
                }
            }
            
            if data < self.last_time {self.overflow+=1;}
            self.last_time = data;
            self.counter += 1;
        }

        fn add_event(&mut self, channel: u32, data: u32) {
            //self.data.0.push((self.get_abs_time(data), channel, self.spim_index(data), self.spim_frame(), None));
            let data = if channel < 16 {
                ISI_CHANNEL_SHIFT[channel as usize] + data
            } else {
                data
            };
            if self.counter >= self.line_offset {
                self.data_raw.0.push((data as u64, channel, None, None, None));
            }
        }

        fn determine_line_time(&mut self) {
            let iter = self.scan_iterator().
                filter_map(|(val1, val2)| {
                    if val2.1.0 > val1.1.0 {
                        Some((val2.1.0 - val1.1.0) as u32)
                    } else {None}
                });

            let mut line = u32::MAX;
            for val in iter {
                if line == val {
                    break;
                }
                line = val;
            };
            println!("***IsiBox***: Line time is (units of 120 ps): {}", line);
            self.line_time = Some(line);
        }

        fn check_for_issues(&mut self) {
            //Check if there is an issue in the first scan. This is very rare but can happens sometimes.
            let iter = self.scan_iterator().
                filter(|(val1, val2)| ((subtract_overflow(val2.1.0, val1.1.0) > self.line_time.unwrap() as u64 + 1_000) || (subtract_overflow(val2.1.0, val1.1.0) < self.line_time.unwrap() as u64 - 1_000))).
                collect::<Vec<_>>();
            for val in iter {
                if val.0.0 == 0 { //First value is literally a bad scan line
                    //Removing the bad vector
                    self.data_raw.0.remove(0);
                    break;
                }
            }
            
            let progress_size = ISI_NB_CORRECTION_ITERACTION;
            let bar = ProgressBar::new(progress_size);
            bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:40.white/black} {percent}% {pos:>7}/{len:7} [ETA: {eta}] Checking for issues in the IsiBox data")
                          .unwrap()
                          .progress_chars("=>-"));

            for _ in 0..progress_size {
                bar.inc(1);
                let iter = self.scan_iterator().
                    filter(|(val1, val2)| ((subtract_overflow(val2.1.0, val1.1.0) > self.line_time.unwrap() as u64 + ISI_CORRECTION_MAX_DIF) || (subtract_overflow(val2.1.0, val1.1.0) < self.line_time.unwrap() as u64 - ISI_CORRECTION_MAX_DIF))).
                    collect::<Vec<_>>();
                //println!("***IsiBox***: Start of a correction cycle. The size is {}.", iter.len());
                let mut number_of_insertions = 0;
                if iter.is_empty() {
                    //println!("***IsiBox***: values successfully corrected."); 
                    break;}
                for val in iter {
                    //println!("{:?}", val);
                    self.data_raw.0.insert(val.1.0+number_of_insertions, (subtract_overflow(val.1.1.0, self.line_time.unwrap() as u64), val.1.1.1, val.1.1.2, val.1.1.3, val.1.1.4));
                    number_of_insertions += 1;
                }
            }
            println!("***IsiBox***: reference values corrected.");
        }

        fn correct_data(&mut self, isi_overflow_correction: u32) {
            let mut counter = 0;
            let mut last_time = 0;
            let mut overflow = 0;
            let low = self.x as u64 * self.pixel_time;
            let y = self.y;
            let x = self.x;

            
            let spim_index = |data: u64, ct: u32, lt: u64| -> Option<u32> {
                let line = ct % y;
                let time = if data > VIDEO_TIME * 13 + lt {
                    data - lt
                } else {
                    data + 67108864 - lt
                };

                if time > low {return None;}
                let column = ((time * x as u64) / low) as u32;
                let index = line * x + column;
                
                Some(index)
            };
            

            self.data_raw.0.iter_mut().for_each(|x| {
                //Correction time
                let raw_time = x.0;
                if x.0 > last_time {
                    x.0 += (overflow + counter * isi_overflow_correction) as TIME * 67108864;
                } else {
                    x.0 += (overflow + counter * isi_overflow_correction + 1) as TIME  * 67108864;
                };
                //Correcting spim index
                x.2 = spim_index(raw_time, counter, last_time);
                //Correcting spim frame
                x.3 = Some(counter / y);

                //If it is a scan signal
                if x.1 == 16 {
                    if raw_time < last_time {
                        overflow+=1;
                    }
                    counter += 1;
                    last_time = raw_time;
                }


            });
        }

        
        fn scan_iterator(&self) -> impl Iterator<Item=((usize, IsiListType), (usize, IsiListType))> + '_ {
            let iter1 = self.data_raw.0.iter().
                cloned().
                enumerate().
                filter(|(_index, (_time, channel, _spim_index, _spim_frame, _dt))| *channel == 16);
            
            let mut iter2 = self.data_raw.0.iter().
                cloned().
                enumerate().
                filter(|(_index, (_time, channel, _spim_index, _spim_frame, _dt))| *channel == 16);

            let _advance = iter2.next();

            iter1.zip(iter2)
        }
        
        pub fn get_timelist_with_tp3_tick(&self) -> Vec<TdcStructureData> {
            let first = self.data_raw.0.iter().
                filter(|(_time, channel, _spim_index, _spim_frame, _dt)| *channel == 16).
                map(|(time, _channel, _spim_index, _spim_frame, _dt)| (*time * 1200 * 6) / 15625).
                next().
                unwrap();
            
            self.data_raw.0.iter().
                map(|(time, channel, _spim_index, _spim_frame, dt)| (((*time * 1200 * 6) / 15625) - first, *channel, *dt, None)).
                collect::<Vec<_>>()
            
        }

        fn output_spim(&self) {
            let spim_vec = self.data_raw.0.iter().
                filter(|(_time, channel, _spim_index, _spim_frame, _dt)| *channel != 16 && *channel != 24).
                filter(|(_time, _channel, spim_index, spim_frame, _dt)| spim_index.is_some() && spim_frame.is_some()).
                map(|(_time, channel, spim_index, _spim_frame, _dt)| spim_index.unwrap() * CHANNELS as u32 + channel).collect::<Vec<u32>>();
 
            let index_vec = self.data_raw.0.iter().
                filter(|(_time, channel, _spim_index, _spim_frame, _dt)| *channel != 16 && *channel != 24).
                filter(|(_time, _channel, spim_index, spim_frame, _dt)| spim_index.is_some() && spim_frame.is_some()).
                map(|(_time, _channel, _spim_index, spim_frame, _dt)| spim_frame.unwrap()).collect::<Vec<u32>>();

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
            let progress_size = self.data_raw.0.len() as u64;
            let vec1_len = self.data_raw.0.iter().filter(|(_time, channel, _spim_index, _spim_frame, _dt)| *channel == ch1).count();
            let mut vec2 = self.data_raw.0.iter().filter(|(_time, channel, _spim_index, _spim_frame, _dt)| *channel == ch2).cloned().collect::<Vec<_>>();
            let iter1 = self.data_raw.0.iter_mut();
            let mut min_index = 0;
            let mut corr = 0;


            let mut new_list = IsiListVecg2(Vec::new());
            let bar = ProgressBar::new(progress_size);
            bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:40.white/black} {percent}% {pos:>7}/{len:7} [ETA: {eta}] Searching photon coincidences")
                          .unwrap()
                          .progress_chars("=>-"));
        
            for val1 in iter1 {
                bar.inc(1);
                if val1.1 != ch1 {continue;}
                for (index, val2) in vec2[min_index..].iter_mut().enumerate() {
                    let dt = val2.0 as i64 - val1.0 as i64;
                    if dt.abs() < 5_000 {

                        val1.4 = Some(dt as i16);
                        val2.4 = Some(dt as i16);

                        corr+=1;
                        new_list.0.push((dt, val2.1, val2.2, val2.3));
                        min_index += index / 10;
                    }
                    if dt > 100_000 {break;}
                }
            }

            self.data_raw.0.iter_mut().
                filter(|(_time, channel, _spim_index, _spim_frame, _dt)| *channel == ch2).
                zip(vec2.iter()).
                for_each(|(ph21, ph22)| {
                    ph21.4 = ph22.4;
                    assert_eq!(ph21.1, ph22.1);
                });

            
            let dt_vec = new_list.0.iter().
                filter(|(_time, _channel, spim_index, spim_frame)| spim_index.is_some() && spim_frame.is_some()).
                map(|(dtime, _channel, _spim_index, _spim_frame)| *dtime).
                collect::<Vec<i64>>();
            
            println!("***IsiBox***: Size of the (first/second) channel: ({} / {}). Number of coincidences: {}. Number of output coincidences: {}. Ratio: {} %.", vec1_len, vec2.len(), corr, dt_vec.len(), dt_vec.len() as f32 * 100.0 / vec1_len as f32);
            
            let spim_index_vec = new_list.0.iter().
                filter(|(_time, _channel, spim_index, spim_frame)| spim_index.is_some() && spim_frame.is_some()).
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
    }

    //Pixel time must be in nanoseconds.
    pub fn get_channel_timelist<V>(mut data: V, spim_size: (POSITION, POSITION), pixel_time: TIME, line_offset: u32, isi_overflow_correction: u32) -> IsiList 
        where V: Read
        {
            let mut list = IsiList{data_raw: IsiListVec(Vec::new()), x: spim_size.0, y: spim_size.1, pixel_time: (pixel_time * 83_333 / 10_000), counter: 0, overflow: 0, last_time: 0, start_time: None, line_time: None, line_offset};
            let mut buffer = [0; 256_000];
            while let Ok(size) = data.read(&mut buffer) {
                if size == 0 {println!("***IsiBox***: Finished reading file."); break;}
                buffer.chunks_exact(4).for_each( |x| {
                    let channel = (as_int(x)[0] & 0xFC000000) >> 27;
                    let _overflow = (as_int(x)[0] & 0x04000000) >> 26;
                    let time = as_int(x)[0] & 0x03FFFFFF;
                    list.add_event(channel, time);
                    if channel == 16 {
                        list.increase_counter(time);
                    }
                })
            }
            list.determine_line_time();
            list.check_for_issues();
            list.correct_data(isi_overflow_correction);
            list.output_spim();
            list.search_coincidence(0, 12);
            list
        }
}

pub mod ntime_resolved {
    use crate::packetlib::Packet;
    use crate::tdclib::{TdcType, TdcRef};
    use crate::errorlib::Tp3ErrorKind;
    use crate::clusterlib::cluster::{SingleElectron, CollectionElectron, ClusterCorrection};
    use crate::auxiliar::{misc::{packet_change, output_data}, value_types::*, ConfigAcquisition, Settings, FileManager};
    use crate::constlib::*;
    use std::io::prelude::*;
    use std::convert::TryInto;
    use std::fs;
    use indicatif::{ProgressBar, ProgressStyle};

    /// This enables spatial+spectral analysis in a certain spectral window.
    pub struct TimeSpectralSpatial<T> {
        hyperspec_index: Vec<INDEXHYPERSPEC>, //Main data,
        hyperspec_return_index: Vec<INDEXHYPERSPEC>, //Main data from flyback,
        fourd_index: Vec<INDEX4D>,
        fourd_return_index: Vec<INDEX4D>,
        frame_indices: Vec<u16>, //indexes from main scan
        frame_return_indices: Vec<u16>, //indexes from flyback
        ensemble: CollectionElectron, //A collection of single electrons,
        spimx: POSITION, //The horinzontal axis of the spim,
        spimy: POSITION, //The vertical axis of the spim,
        tdc_periodic: Option<TdcRef>, //The periodic tdc. Can be none if xspim and yspim <= 1,
        spim_tdc_type: TdcType, //The tdc type for the spim,
        extra_tdc_type: TdcType, //The tdc type for the external,
        remove_clusters: T,
        file: String,
        fourd_data: bool,
        my_settings: Settings,
    }

    impl<T: ClusterCorrection> TimeSpectralSpatial<T> {
        fn prepare(&mut self, file: &mut fs::File) -> Result<(), Tp3ErrorKind> {
            let mut empty_filemanager = FileManager::new_empty();
            self.tdc_periodic = match self.tdc_periodic {
                None if self.spimx>1 && self.spimy>1 => {
                    Some(TdcRef::new_periodic(self.spim_tdc_type.clone(), file, &self.my_settings, &mut empty_filemanager).expect("Problem in creating periodic tdc ref."))
                },
                Some(val) => Some(val),
                _ => None,
            };
            Ok(())
        }
    
        fn try_create_folder(&self) -> Result<(), Tp3ErrorKind> {
            let path_length = &self.file.len();
            match fs::create_dir(&self.file[..path_length - 5]) {
                Ok(_) => {Ok(())},
                Err(_) => { Err(Tp3ErrorKind::CoincidenceFolderAlreadyCreated) }
            }
        }

        fn add_electron(&mut self, packet: &Packet, packet_index: usize) {
            let se = SingleElectron::new(packet, self.tdc_periodic, packet_index);
            self.ensemble.add_electron(se);
        }

        fn add_spim_tdc(&mut self, packet: &Packet) {
            //Synchronizing clocks using two different approaches. It is always better to use a multiple of 2 and use the FPGA counter.
            if let Some(my_tdc_periodic) = &mut self.tdc_periodic {
                my_tdc_periodic.upt(packet.tdc_time_norm(), packet.tdc_counter());
            }
        }
        
        fn add_extra_tdc(&mut self, _packet: &Packet) {
            //self.spectra.push(SPIM_PIXELS);
            //spimlib::get_spimindex(, dt: TIME, spim_tdc: &PeriodicTdcRef, self.spimx, self.spimy;
        }

        fn process(&mut self) -> Result<(), Tp3ErrorKind> {
            if self.fourd_data {
                Ok(self.process_fourd()?)
            } else {
                Ok(self.process_hyperspec()?)
            }
        }
        
        fn process_hyperspec(&mut self) -> Result<(), Tp3ErrorKind> {
            if self.ensemble.try_clean(0, &self.remove_clusters) {
                for val in self.ensemble.iter() {
                    if let Some(index) = val.get_or_not_spim_index(self.tdc_periodic, self.spimx, self.spimy) {
                        self.hyperspec_index.push(index);
                        self.frame_indices.push((val.spim_slice()).try_into().expect("Exceeded the maximum number of indices"));
                    }
                    
                    if let Some(index) = val.get_or_not_return_spim_index(self.tdc_periodic, self.spimx, self.spimy) {
                        self.hyperspec_return_index.push(index);
                        self.frame_return_indices.push((val.spim_slice()).try_into().expect("Exceeded the maximum number of indices"));
                    }
            }
            self.ensemble.clear();

            output_data(&self.hyperspec_index, self.file.clone(), "si_complete.txt");
            output_data(&self.hyperspec_return_index, self.file.clone(), "si_return_complete.txt");
            output_data(&self.frame_indices, self.file.clone(), "si_complete_indices.txt");
            output_data(&self.frame_return_indices, self.file.clone(), "si_complete_return_indices.txt");

            self.hyperspec_index.clear();
            self.hyperspec_return_index.clear();
            self.frame_indices.clear();
            self.frame_return_indices.clear();
            }
            Ok(())
        }
        
        fn process_fourd(&mut self) -> Result<(), Tp3ErrorKind> {
            if self.ensemble.try_clean(0, &self.remove_clusters) {
                for val in self.ensemble.iter() {
                    if let Some(index) = val.get_or_not_4d_index(self.tdc_periodic, self.spimx, self.spimy) {
                        self.fourd_index.push(index);
                        self.frame_indices.push((val.spim_slice()).try_into().expect("Exceeded the maximum number of indices"));
                    }
                    
                    if let Some(index) = val.get_or_not_return_4d_index(self.tdc_periodic, self.spimx, self.spimy) {
                        self.fourd_return_index.push(index);
                        self.frame_return_indices.push((val.spim_slice()).try_into().expect("Exceeded the maximum number of indices"));
                    }
            }
            self.ensemble.clear();

            output_data(&self.fourd_index, self.file.clone(), "fourd_complete.txt");
            output_data(&self.fourd_return_index, self.file.clone(), "fourd_return_complete.txt");
            output_data(&self.frame_indices, self.file.clone(), "fourd_complete_indices.txt");
            output_data(&self.frame_return_indices, self.file.clone(), "fourd_complete_return_indices.txt");

            self.fourd_index.clear();
            self.fourd_return_index.clear();
            self.frame_indices.clear();
            self.frame_return_indices.clear();
            }
            Ok(())
        }
        
        pub fn new(my_config: ConfigAcquisition<T>, my_settings: Settings) -> Result<Self, Tp3ErrorKind> {

            Ok(Self {
                hyperspec_index: Vec::new(),
                hyperspec_return_index: Vec::new(),
                fourd_index: Vec::new(),
                fourd_return_index: Vec::new(),
                frame_indices: Vec::new(),
                frame_return_indices: Vec::new(),
                ensemble: CollectionElectron::new(),
                spimx: my_config.xspim,
                spimy: my_config.yspim,
                tdc_periodic: None,
                spim_tdc_type: MAIN_TDC,
                extra_tdc_type: SECONDARY_TDC,
                remove_clusters: my_config.correction_type,
                file: my_config.file,
                fourd_data: my_settings.mode != 2,
                my_settings,
                
            })
        }
    }

    pub fn analyze_data<T: ClusterCorrection>(data: &mut TimeSpectralSpatial<T>) -> Result<(), Tp3ErrorKind> {
        
        data.try_create_folder()?;
        
        let mut prepare_file = fs::File::open(&data.file).expect("Could not open desired file.");
        let progress_size = prepare_file.metadata().unwrap().len();
        data.prepare(&mut prepare_file)?;
        
        let mut my_file = fs::File::open(&data.file).expect("Could not open desired file.");
        let mut buffer: Vec<u8> = vec![0; 512_000_000];
        
        let mut ci = 0;
            
        let bar = ProgressBar::new(progress_size);
        bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:40.white/black} {percent}% {pos:>7}/{len:7} [ETA: {eta}] Reconstructing hyperspectral image.")
                      .unwrap()
                      .progress_chars("=>-"));

        while let Ok(size) = my_file.read(&mut buffer) {
            if size==0 {break;}
            bar.inc(512_000_000_u64);
            buffer[0..size].chunks_exact(8).enumerate().for_each(|(current_raw_index, pack_oct)| {
                match pack_oct {
                    &[84, 80, 88, 51, nci, _, _, _] => {ci = nci},
                    _ => {
                        let packet = Packet{chip_index: ci, data: packet_change(pack_oct)[0]};
                        match packet.id() {
                            6 if packet.tdc_type() == data.spim_tdc_type.associate_value() => {
                                data.add_spim_tdc(&packet);
                            },
                            6 if packet.tdc_type() == data.extra_tdc_type.associate_value() => {
                                data.add_extra_tdc(&packet);
                            },
                            11 => {
                                data.add_electron(&packet, current_raw_index);
                            },
                            _ => {},
                        };
                    },
                }
            });
            data.process()?
        };
        println!("File has been succesfully read.");
        Ok(())
    }
}

pub mod calibration {

    use std::fs::OpenOptions;
    use crate::packetlib::Packet;
    use crate::auxiliar::misc::{as_bytes, packet_change};
    use std::io;
    use std::io::prelude::*;
    use std::fs;
    use std::convert::TryInto;
    use crate::clusterlib::cluster::{SingleElectron, CollectionElectron, ClusterCorrection};
    use indicatif::{ProgressBar, ProgressStyle};
    
    fn output_data<T>(data: &[T], name: &str) {
        let mut tfile = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(name).unwrap();
        tfile.write_all(as_bytes(data)).unwrap();
        println!("Outputting data under {:?} name. Vector len is {}", name, data.len());
    }
    
    pub struct CalibrationData {
        rel_time: Vec<i8>,
        x: Vec<u16>,
        y: Vec<u8>,
        tot: Vec<u16>,
        cluster_size: Vec<u16>,
    }

    impl CalibrationData {
        fn new() -> Self {
            CalibrationData {rel_time: Vec::new(), x: Vec::new(), y: Vec::new(), tot: Vec::new(), cluster_size: Vec::new()}
        }
        fn append_from_collection(&mut self, val: CollectionElectron) {
            for electron in val.iter() {
                self.x.push(electron.x().try_into().unwrap());
                self.y.push(electron.y().try_into().unwrap());
                self.tot.push(electron.tot());
                let electron_time = electron.time() as i64;
                let electron_tot_reference = electron.frame_dt() as i64;
                let time_diference = (electron_time - electron_tot_reference) as i8;
                self.rel_time.push(time_diference);
                self.cluster_size.push(electron.cluster_size().try_into().unwrap());
            }
        }
        pub fn output_relative_calibration_time(&self) {
            output_data(&self.rel_time, "relative_calibration_time.txt");
        }
        pub fn output_x(&self) {
            output_data(&self.x, "relative_calibration_x.txt");
        }
        pub fn output_y(&self) {
            output_data(&self.y, "relative_calibration_y.txt");
        }
        pub fn output_tot(&self) {
            output_data(&self.tot, "relative_calibration_tot.txt");
        }
        pub fn output_cluster_size(&self) {
            output_data(&self.cluster_size, "relative_calibration_cluster_size.txt");
        }
    }

    pub fn calibrate<T: ClusterCorrection>(file: &str, correction_type: &T) -> io::Result<()> {

        let mut ci = 0;
        let mut file = fs::File::open(file)?;
        let progress_size = file.metadata().unwrap().len();
        let mut buffer: Vec<u8> = vec![0; 512_000_000];
        let mut total_size = 0;
        
        let bar = ProgressBar::new(progress_size);
        bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:40.white/black} {percent}% {pos:>7}/{len:7} [ETA: {eta}] Searching for clusters and calibrating data")
                      .unwrap()
                      .progress_chars("=>-"));
        
        let mut calibration_data = CalibrationData::new();
        while let Ok(size) = file.read(&mut buffer) {
            let mut temp_electrons = CollectionElectron::new();
            if size == 0 {println!("Finished Reading."); break;}
            total_size += size;
            //if total_size / 1_000_000_000 > 2 {break;}
            bar.inc(512_000_000_u64);
            buffer[0..size].chunks_exact(8).enumerate().for_each(|(current_raw_index, pack_oct)| {
                match *pack_oct {
                    [84, 80, 88, 51, nci, _, _, _] => {ci=nci;},
                    _ => {
                        let packet = Packet { chip_index: ci, data: packet_change(pack_oct)[0] };
                        if packet.id() == 11 {
                            let se = SingleElectron::new(&packet, None, current_raw_index);
                            temp_electrons.add_electron(se);
                            //temp_edata.electron.add_electron(se);
                        }
                    },
                };
            });
            temp_electrons.sort();
            temp_electrons.try_clean(0, correction_type);
            calibration_data.append_from_collection(temp_electrons);
        }
        calibration_data.output_relative_calibration_time();
        calibration_data.output_x();
        calibration_data.output_y();
        calibration_data.output_tot();
        calibration_data.output_cluster_size();
        println!("Total number of bytes read {}", total_size);
        Ok(())
    }
}

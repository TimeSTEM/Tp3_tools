




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

    const TIME_WIDTH: TIME = 40; //Time width to correlate (in units of 640 Mhz, or 1.5625 ns).
    const TIME_DELAY: TIME = 104; // + 50_000; //Time delay to correlate (in units of 640 Mhz, or 1.5625 ns).
    
    fn as_bytes<T>(v: &[T]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u8,
                v.len() * std::mem::size_of::<T>())
        }
    }

    pub struct ElectronData {
        time: Vec<TIME>,
        rel_time: Vec<i64>,
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
                //println!("TPX3: {}", pack.tdc_time_norm());
                spim_tdc.upt(pack.tdc_time_norm(), pack.tdc_counter());
            }
        }

        fn add_coincident_electron(&mut self, val: SingleElectron, photon_time: TIME) {
            //self.corr_spectrum[val.image_index() as usize] += 1; //Adding the electron
            self.corr_spectrum[val.x() as usize] += 1; //Adding the electron
            self.corr_spectrum[SPIM_PIXELS as usize-1] += 1; //Adding the photon
            self.time.push(val.time());
            self.tot.push(val.tot());
            self.x.push(val.x());
            self.y.push(val.y());
            self.rel_time.push(val.relative_time_from_abs_tdc(photon_time));
            if let Some(index) = val.get_or_not_spim_index(self.spim_tdc, self.spim_size.0, self.spim_size.1) {
                self.spim_index.push(index);
            }
        }
        
        fn add_events(&mut self, mut temp_edata: TempElectronData, temp_tdc: &mut TempTdcData, time_delay: TIME, time_width: TIME) {
            temp_tdc.sort();
            let nphotons = temp_tdc.tdc.len();
            println!("Supplementary events: {}.", nphotons);

            //if temp_edata.electron.check_if_overflow() {self.overflow_electrons += 1;}
            //if temp_edata.electron.correct_electron_time(self.overflow_electrons) {self.overflow_electrons += 1};
            //let first = temp_edata.electron.values().next();
            //let last = temp_edata.electron.values().last();
            //println!("{:?} and {:?}", first, last);
            
            if temp_edata.electron.check_if_overflow() {self.overflow_electrons += 1;}
            temp_edata.electron.sort();
            temp_edata.electron.try_clean(0, self.remove_clusters);

            self.spectrum[SPIM_PIXELS as usize-1]=nphotons; //Adding photons to the last pixel

            let mut min_index = 0;
            for val in temp_edata.electron.values() {
                let mut index = 0;
                self.add_electron(*val);
                for ph in temp_tdc.tdc[min_index..].iter() {
                    let dt = (ph/6) as i64 - val.time() as i64 - time_delay as i64;
                    if (dt.abs() as TIME) < time_width {
                        self.add_coincident_electron(*val, *ph);
                        min_index += index/2;
                    }
                    if dt > 100_000 {break;}
                    index += 1;
                }
            }

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
        
        pub fn output_dispersive(&self) {
            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("xH.txt").expect("Could not output X histogram.");
            tfile.write_all(as_bytes(&self.x)).expect("Could not write time to file.");
            println!("Outputting each dispersive value under xH name. Vector len is {}", self.rel_time.len());
        }
        
        pub fn output_non_dispersive(&self) {
            let mut tfile = OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open("yH.txt").expect("Could not output Y histogram.");
            tfile.write_all(as_bytes(&self.y)).expect("Could not write time to file.");
            println!("Outputting each non-dispersive value under yH name. Vector len is {}", self.rel_time.len());
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

        fn new_from_isilist(list: isi_box::IsiList) -> Self {
            let vec_list = list.get_timelist_with_tp3_tick();
            Self {
                tdc: vec_list,
                min_index: 0,
            }
        }

        fn add_tdc(&mut self, my_pack: &Pack) {
            self.tdc.push(my_pack.tdc_time_abs_norm());
        }

        fn sort(&mut self) {
            self.tdc.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
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
        coinc_data.add_events(temp_edata, &mut temp_tdc, 104, 40);
        println!("Time elapsed: {:?}", start.elapsed());

        }
        println!("Total number of bytes read {}", total_size);
        Ok(())
    }
    
    pub fn search_coincidence_isi(file: &str, coinc_data: &mut ElectronData) -> io::Result<()> {
    
        //TP3 configurating TDC Ref
        let mut file0 = fs::File::open(file)?;
        let spim_tdc = PeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut file0, Some(coinc_data.spim_size.1)).expect("Could not create period TDC reference.");
        coinc_data.prepare_spim(spim_tdc);
        let begin_tp3_time = spim_tdc.begin_frame;
    
        //IsiBox loading file
        let f = fs::File::open("isi_raw205.isi").unwrap();
        let temp_list = isi_box::get_channel_timelist(f);
        let begin_isi_time = temp_list.start_time;
        let mut temp_tdc = TempTdcData::new_from_isilist(temp_list);

        println!("{:?} and {:?} and {:?}", begin_tp3_time, (begin_isi_time.unwrap() as TIME * 1200) / 15625, begin_isi_time.unwrap());

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
            buffer[0..size].chunks_exact(8).for_each(|pack_oct| {
                match *pack_oct {
                    [84, 80, 88, 51, nci, _, _, _] => {ci=nci;},
                    _ => {
                        let packet = Pack { chip_index: ci, data: pack_oct.try_into().unwrap() };
                        match packet.id() {
                            //6 if packet.tdc_type() == np_tdc.id() => {
                            //    temp_tdc.add_tdc(&packet);
                            //},
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
        //coinc_data.add_events(temp_edata, &mut temp_tdc, begin_tp3_time - (begin_isi_time.unwrap() as TIME * 1200) / 15625, 400);
        coinc_data.add_events(temp_edata, &mut temp_tdc, 0, 4000);
        println!("Time elapsed: {:?}", start.elapsed());
        break;

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
    
    struct IsiListVec(Vec<(TIME, u32, u32, u32)>);
    struct IsiListVecg2(Vec<(i64, u32, u32, u32)>);
    
    pub struct IsiList {
        data: IsiListVec, //Time, channel, spim index, spim frame
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
                let mut val = data as i32 - self.last_time as i32;
                if val < 0 {
                    val += 67108864;
                }
                if (val - line_time as i32).abs() > 10 {
                    println!("{} and {:?} and {} and {}", val, self.spim_frame(), self.counter, line_time);
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
                    data - VIDEO_TIME as u32 * 13 - self.last_time
                } else {
                    data + 67108864 - VIDEO_TIME as u32 * 13 - self.last_time
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
            if let (Some(spim_index), Some(spim_frame), Some(_)) = (self.spim_index(data), self.spim_frame(), self.line_time) {
                self.data.0.push((self.get_abs_time(data), channel, spim_index, spim_frame));
            };
        }
        
        pub fn get_timelist_with_tp3_tick(&self) -> Vec<TIME> {
            self.data.0.iter().
                filter(|(_time, channel, _spim_index, _spim_frame)| *channel != 16 || *channel != 24).
                map(|(time, _channel, _spim_index, _spim_frame)| (*time * 1200) / 15625).
                collect::<Vec<TIME>>()
        }

        /*
        pub fn get_interator_sync(&self) -> TIME {
            self.data.0.iter().
                filter(|(_time, channel, _spim_index, _spim_frame)| *channel == 16).
                map(|(time, _channel, _spim_index, _spim_frame)| (*time * 1200) / 15625)
        }
        */

        fn output_spim(&self) {
            let spim_vec = self.data.0.iter().map(|(_time, channel, spim_index, _spim_frame)| *spim_index * CHANNELS as u32 + channel).collect::<Vec<u32>>();
            let index_vec = self.data.0.iter().map(|(_time, channel, _spim_index, spim_frame)| *spim_frame).collect::<Vec<u32>>();

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

        fn search_coincidence(&self, ch1: u32, ch2: u32) {
            let iter1 = self.data.0.iter().filter(|(_time, channel, _spim_index, _spim_frame)| *channel == ch1);
            let size = self.data.0.iter().filter(|(_time, channel, _spim_index, _spim_frame)| *channel == ch1).count();
            let vec2 = self.data.0.iter().filter(|(_time, channel, _spim_index, _spim_frame)| *channel == ch2).cloned().collect::<Vec<_>>();
            let mut count = 0;
            let mut min_index = 0;
            let mut corr = 0;


            let mut new_list = IsiListVecg2(Vec::new());
            
            for val1 in iter1 {
                let mut index = 0;
                if count % 200000 == 0 {
                    println!("Complete: {}%. Current photon is is: {:?}", count*100/size, val1);
                }
                count+=1;
                for val2 in &vec2[min_index..] {
                    let dt = val2.0 as i64 - val1.0 as i64;
                    if dt.abs() < 500 {
                        corr+=1;
                        new_list.0.push((dt, val2.1, val2.2, val2.3));
                        min_index += index / 5;
                    }
                    if dt > 10_000 {break;}
                    index += 1;
                }
            }

            println!("{}", vec2.len());
            println!("{}", corr);
            
            let dt_vec = new_list.0.iter().map(|(dtime, _channel, _spim_index, _spim_frame)| *dtime).collect::<Vec<i64>>();
            let spim_index_vec = new_list.0.iter().map(|(_dtime, _channel, spim_index, _spim_frame)| *spim_index).collect::<Vec<u32>>();

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

    pub fn get_channel_timelist<V>(mut data: V) -> IsiList 
        where V: Read
        {
            //let zlp = Normal::new(100.0, 25.0).unwrap();
            let mut list = IsiList{data: IsiListVec(Vec::new()), x: 256, y: 256, pixel_time: 66_667, counter: 0, overflow: 0, last_time: 0, start_time: None, line_time: None};
            let mut buffer = [0; 256_000];
            while let Ok(size) = data.read(&mut buffer) {
                if size == 0 {println!("Finished Reading."); break;}
                buffer.chunks_exact(4).for_each( |x| {
                    let channel = (as_int(x)[0] & 0xFC000000) >> 27;
                    let time = as_int(x)[0] & 0x03FFFFFF;
                    if channel == 16 {
                        list.increase_counter(time);
                    }
                    else if channel == 24 {}
                    else {
                        list.add_event(channel, time);
                    }

                        //println!("{} and {} and {}", channel, list.get_abs_time(time), ((list.get_abs_time(time)) * 1200) / 15625 + 577797779);
                    //} else if channel == 24 {
                    //} else {
                    //    list.add_event(channel, time);
                        //let val = zlp.sample(&mut thread_rng());
                        //let val_pos = (val as i32).abs() as u32;
                        //if val as i32 >= 0 {
                        //    list.add_event(0, time+val_pos);
                        //} else {
                        //    if time>val_pos {
                        //        list.add_event(0, time-val_pos);
                        //    }
                        //}
                

                })
            }
            list.output_spim();
            list.search_coincidence(2, 12);
            println!("{:?} and {:?} and {} and {} and {:?}", list.start_time, list.line_time, list.counter, list.overflow, list.last_time);
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

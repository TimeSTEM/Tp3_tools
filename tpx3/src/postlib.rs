//!`postlib` is a collection of tools to treat acquisition data.
pub mod coincidence {
    //!Used for temporally correlation between electrons and external events, by
    //!means of the TDCs in the SPIDR readout or not.
    use crate::packetlib::Packet;
    use crate::tdclib::TdcRef;
    use crate::errorlib::Tp3ErrorKind;
    use crate::clusterlib::cluster::ClusterCorrectionTypes;
    use std::io::prelude::*;
    use std::fs;
    use crate::clusterlib::cluster::{SingleElectron, CollectionElectron, SinglePhoton, CollectionPhoton};
    use crate::auxiliar::{Settings, value_types::*, misc::{output_data, packet_change}, FileManager};
    use crate::constlib::*;
    use indicatif::{ProgressBar, ProgressStyle};
    use std::sync::{mpsc, Arc, Mutex, Condvar};
    use std::thread;

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

    //This is the struct that is sent over the channels for parallelization. Basically we send data
    //here and process in another thread.
    struct ChannelSender {
        temp_electron: CollectionElectron,
        temp_photon: CollectionPhoton,
        raw_index: Vec<usize>
    }

    impl ChannelSender {
        fn new() -> Self {
            Self {
                temp_electron: CollectionElectron::new(),
                temp_photon: CollectionPhoton::new(),
                raw_index: Vec::new(),
            }
        }
        
        fn add_electron(&mut self, val: SingleElectron) {
            self.temp_electron.add_electron(val);
        }
        fn add_photon(&mut self, val: SinglePhoton) {
            self.temp_photon.add_photon(val);
        }
        //This adds the index of the 64-bit packet that will be afterwards added to the electron
        //data and then to the reduced raw. We should not do on the fly as the order of the packets will not be preserved for
        //photons and electrons, for example (we would add one photon but then check later if there
        //is a correspondent electron). So we should run once and then run again for the
        //recorded indexes.
        pub fn add_packet_index(&mut self, val: usize) {
            self.raw_index.push(val);
        }
        fn sort_all(&mut self) {
            //Sorting photons.
            self.temp_photon.sort();
            self.temp_photon.dedup_by(|a, b| a.raw_packet_data() == b.raw_packet_data());

            //Sorting and removing clusters (if need) for electrons.
            self.temp_electron.sort();
            self.temp_electron.dedup_by(|a, b| a.raw_packet_data().data() == b.raw_packet_data().data());
        }

    }
 
    //The settings to be used to create electron data. Used for taking actions in the data to be
    //treated.
    #[derive(Clone)]
    pub struct ElectronDataSettings {
        remove_clusters: ClusterCorrectionTypes,
        file: String,
        my_settings: Settings,
        save_locally: bool,
        tdc1: TdcRef, //If its a Hyperspectral Image, the Reference TDC should be here.
        tdc2: TdcRef, //If its a Fast Oscillator experiment, the Reference TDC should be here.
    }

    impl ElectronDataSettings {

        //Condition to be a SpectralImage
        fn is_spim(&self) -> bool {
            self.my_settings.mode == 2
        }

        //Condition of the FastOscillator
        fn is_fast_oscillator(&self) -> bool {
            self.my_settings.mode == 99 //TODO: this is incorrect and for debug purposes only
        }

        //Get spectralImage TDC
        fn try_get_spim_tdc(&self) -> Option<&TdcRef> {
            if self.is_spim() {
                Some(&self.tdc1)
            } else {
                None
            }
        }
 
        //Get spectralImage TDC as mut
        fn try_get_spim_tdc_mut(&mut self) -> Option<&mut TdcRef> {
            if self.is_spim() {
                Some(&mut self.tdc1)
            } else {
                None
            }
        }

        //Get Oscillator TDC
        fn try_get_oscillator_tdc(&self) -> Option<&TdcRef> {
            if self.is_fast_oscillator() {
                Some(&self.tdc2)
            } else {
                None
            }
        }

        //Get Oscillator TDC as mut
        fn try_get_oscillator_tdc_mut(&mut self) -> Option<&mut TdcRef> {
            if self.is_fast_oscillator() {
                Some(&mut self.tdc2)
            } else {
                None
            }
        }
 
        fn create_tdcs(&mut self) {
            //Opening the raw data file. We have already checked if the file opens so no worries here.
            let mut file0 = fs::File::open(&self.file).unwrap();

            if self.is_spim() {
                if self.my_settings.xspim_size == 0 || self.my_settings.yspim_size == 0 {
                    panic!("***Coincidence***: Spim mode is on. X and Y pixels must be greater than 0.");
                }
                let mut empty_filemanager = FileManager::new_empty();
                let temp = TdcRef::new_periodic(MAIN_TDC, &mut file0, &self.my_settings, &mut empty_filemanager).expect("Could not create period TDC reference.");
                self.tdc1 = temp;
            };
            

            if self.is_fast_oscillator() {
                let mut empty_filemanager = FileManager::new_empty();
                let temp = TdcRef::new_periodic(SECONDARY_TDC, &mut file0, &self.my_settings, &mut empty_filemanager).expect("Could not create period TDC reference.");
                self.tdc2 = temp;
            };
        }

        fn try_create_folder(&self) -> Result<(), Tp3ErrorKind> {
            let path_length = &self.file.len();
            match fs::create_dir(&self.file[..path_length - 5]) {
                Ok(_) => {Ok(())},
                Err(_) => { Err(Tp3ErrorKind::FolderAlreadyCreated) }
            }
        }
        
        fn is_file_readable(&self) -> Result<(), Tp3ErrorKind> {
            match fs::File::open(&self.file) {
                Ok(_) => {Ok(())},
                Err(_) => { Err(Tp3ErrorKind::CoincidenceCantReadFile) }
            }
        }
		
		fn copy_json(&self) -> Result<(), Tp3ErrorKind> {
			let tpx3_file = std::path::Path::new(&self.file);
			
			let stem_file = tpx3_file.file_stem().unwrap();
			let stem_file = std::path::Path::new(stem_file);
			let json_filename = format!("{}.json", stem_file.to_string_lossy());
			
			let parent_folder = tpx3_file.parent().unwrap();
			let destination_folder = parent_folder.join(stem_file);
			
			let json_source = parent_folder.join(json_filename);
			let json_destination = destination_folder.join("reduced_raw.json");
			
            std::fs::copy(json_source, json_destination)?;
			Ok(())
		}

        pub fn prepare_to_search(&mut self) -> Result<(), Tp3ErrorKind> {
            if self.save_locally {self.try_create_folder()?;};
            self.is_file_readable()?;
			self.copy_json()?;
            Ok(())
        }

        pub fn new(file_path: String, correction_type: ClusterCorrectionTypes, my_settings: Settings, save_locally: bool) -> Self {
            Self {
                remove_clusters: correction_type,
                file: file_path,
                my_settings,
                save_locally,
                tdc1: TdcRef::new_no_read(MAIN_TDC).expect("Could not create non-periodic TDC reference."),
                tdc2: TdcRef::new_no_read(SECONDARY_TDC).expect("Could not create non-periodic TDC reference.")
            }
        } 
    }


    //Non-standard data types 
    pub struct ElectronData {
        reduced_raw_data: Vec<u64>,
        index_to_add_in_raw: Vec<usize>,
        coinc_electrons: CollectionElectron,
        spectrum: Vec<u32>,
        corr_spectrum: Vec<u32>,
        spim_frame: Vec<u32>,
        spim_size: (POSITION, POSITION),
        edata_settings: ElectronDataSettings,
    }

    impl ElectronData {

        //Called for all the electrons (not only coincident).
        fn add_electron(&mut self, val: &SingleElectron) {
            self.spectrum[val.x() as usize] += 1;
            if let Some(index) = val.get_or_not_spim_index(self.edata_settings.try_get_spim_tdc(), self.spim_size.0, self.spim_size.1) {
                self.spim_frame[index as usize] += 1;
            }
        }
        
        //Called for all the photons (not only coincident).
        fn add_photon(&mut self, val: &SinglePhoton) {
            self.spectrum[PIXELS_X as usize - 1] += 1;
            if let Some(index) = val.get_or_not_spim_index(self.edata_settings.try_get_spim_tdc(), self.spim_size.0, self.spim_size.1) {
                self.spim_frame[index as usize] += 1;
            }
        }

        //This adds the indexes collected with the ChannelSender (all the TDCs, for example, are
        //automatically save to the raw reduced data)
        fn add_packet_to_raw_index_from_channel_sender(&mut self, channel_sender: &mut ChannelSender) {
            self.index_to_add_in_raw.append(&mut channel_sender.raw_index);
        }
        
        //This adds the packet to the reduced raw value and clear the index list afterwards
        fn add_packets_to_reduced_data(&mut self, buffer: &[u8]) {
            //Now we must add the concerned data to the reduced raw. We should first sort the indexes
            //that we have saved, ensuring that the data is saved in the same order as the raw
            //data.
            self.index_to_add_in_raw.sort();
            //Then we should iterate and see matching indexes to add.
            for index in self.index_to_add_in_raw.iter() {
                let value = packet_change(&buffer[index * 8..(index + 1) * 8])[0];
                self.reduced_raw_data.push(value);
            }
            self.index_to_add_in_raw.clear();
        }

        fn add_coincident_electron(&mut self, val: SingleElectron) {
            self.corr_spectrum[val.x() as usize] += 1; //Adding the electron
            self.corr_spectrum[PIXELS_X as usize-1] += 1; //Adding the photon
            self.coinc_electrons.add_electron(val);
        }
        
        fn add_events(&mut self, channel_sender: &mut ChannelSender, time_delay: TIME, time_width: TIME, _line_offset: i64) {
            //Removing clusters (if need) for electrons.
            channel_sender.temp_electron.try_clean(0, &self.edata_settings.remove_clusters);

            //Adding photons to the last pixel. We also add the photons in the spectra image.
            channel_sender.temp_photon.iter().for_each(|photon| self.add_photon(photon));

            //Adding electrons to the spectra image
            channel_sender.temp_electron.iter().for_each(|electron| self.add_electron(electron));

            //This effectivelly searches for coincidence. It also adds electrons in self.index_to_add_in_raw.
            let coinc_electron = channel_sender.temp_electron.search_coincidence(&channel_sender.temp_photon, &mut self.index_to_add_in_raw, time_delay, time_width);

            //Adding electron in the coincidence action
            coinc_electron.into_iter().for_each(|electron| self.add_coincident_electron(electron));
        }

        pub fn new_from_settings(eds: &ElectronDataSettings) -> Self {
            Self {
                reduced_raw_data: Vec::new(),
                index_to_add_in_raw: Vec::new(),
                coinc_electrons: CollectionElectron::new(),
                spim_frame: vec![0; (PIXELS_X * eds.my_settings.xspim_size * eds.my_settings.yspim_size) as usize],
                spectrum: vec![0; PIXELS_X as usize],
                corr_spectrum: vec![0; PIXELS_X as usize],
                spim_size: (eds.my_settings.xspim_size, eds.my_settings.yspim_size),
                edata_settings: eds.clone(),
            }
        }
              
        fn output_hyperspec(&self) {
            if !self.edata_settings.save_locally { return; };
            output_data(&self.spim_frame, self.edata_settings.file.clone(), "spim_frame.txt");
        }

        pub fn get_electron_collection(&self) -> &CollectionElectron {
            &self.coinc_electrons
        }
        pub fn create_x(&self) -> Vec<u16> {
            self.coinc_electrons.iter().map(|se| se.x() as u16).collect()
        }
        pub fn create_y(&self) -> Vec<u16> {
            self.coinc_electrons.iter().map(|se| se.y() as u16).collect()
        }
        pub fn create_channel(&self) -> Vec<u8> {
            self.coinc_electrons.iter().map(|se| se.coincident_photon().unwrap().channel() as u8).collect()
        }
        pub fn create_tot(&self) -> Vec<u16> {
            self.coinc_electrons.iter().map(|se| se.tot()).collect()
        }
        pub fn create_abs_time(&self) -> Vec<TIME> {
            self.coinc_electrons.iter().map(|se| se.time()).collect()
        }
        pub fn create_rel_time(&self) -> Vec<i16> {
            self.coinc_electrons.iter()
                .filter_map(|se| se.relative_time_from_coincident_photon()
                                            .map(|value| value.fold())).collect()
        }
        pub fn create_rel_corrected_time(&self) -> Vec<i16> {
            self.coinc_electrons.iter()
                .filter_map(|se| se.relative_corrected_time_from_coincident_photon()
                            .map(|value| value.fold())).collect()
        }
        pub fn create_condensed_packet(&self) -> Vec<u64> {
            self.coinc_electrons.iter().map(|se| se.raw_packet_data().modified_packet_data()).collect()
        }
        pub fn create_reduced_raw(&self) -> &[u64] {
            &self.reduced_raw_data
        }
        pub fn create_spim_index(&self) -> Vec<INDEXHYPERSPEC> {
            self.coinc_electrons.iter().map(|se| se.get_or_not_spim_index(self.edata_settings.try_get_spim_tdc(), self.spim_size.0, self.spim_size.1).unwrap_or(POSITION::MAX)).collect()
        }

        fn early_output_data(&mut self) {
            //This reorders the packet based on how they have arrived. If you are in the middle of
            //a time overflow and sort the data, output data would be strange without this.
            self.coinc_electrons.reorder_by_packet_index();

            if !self.edata_settings.save_locally { return; };
            
            let relative_corrected_time: Vec<i16> = self.create_rel_corrected_time();
            let channel: Vec<u8> = self.create_channel();
            let relative_time: Vec<i16> = self.create_rel_time();
            let x: Vec<u16> = self.create_x();
            let y: Vec<u16> = self.create_y();
            let tot: Vec<u16> = self.create_tot();
            let time: Vec<TIME> = self.create_abs_time();
            let condensed_packet: Vec<u64> = self.create_condensed_packet();
            let spim_index: Vec<INDEXHYPERSPEC> = self.create_spim_index();

            output_data(&channel, self.edata_settings.file.clone(), "channel.txt");
            output_data(&relative_time, self.edata_settings.file.clone(), "tH.txt");
            output_data(&relative_corrected_time, self.edata_settings.file.clone(), "tcorH.txt");
            output_data(&x, self.edata_settings.file.clone(), "xH.txt");
            output_data(&y, self.edata_settings.file.clone(), "yH.txt");
            output_data(&tot, self.edata_settings.file.clone(), "tot.txt");
            output_data(&time, self.edata_settings.file.clone(), "tabsH.txt");
            output_data(&condensed_packet, self.edata_settings.file.clone(), "condensed_packet.txt");
            output_data(&spim_index, self.edata_settings.file.clone(), "si.txt");
            self.coinc_electrons.clear();

            //Output corr EELS spectrum
            output_data(&self.corr_spectrum, self.edata_settings.file.clone(), "cspec.txt");
            self.corr_spectrum.iter_mut().for_each(|x| *x = 0);
            
            //Output total EELS spectrum
            output_data(&self.spectrum, self.edata_settings.file.clone(), "spec.txt");
            self.spectrum.iter_mut().for_each(|x| *x = 0);
                
            //Output reduced raw
            output_data(&self.reduced_raw_data, self.edata_settings.file.clone(), "reduced_raw.tpx3");
            self.reduced_raw_data.clear();
            
        }
            
    }
    
    pub fn search_coincidence(mut coinc_data_set: ElectronDataSettings, limit_read_size: u32){

        //Consumer & Producer 
        let (tx, rx) = mpsc::channel();

        //Creating the appropriate TDCs
        coinc_data_set.create_tdcs();
        let mut coinc_data = ElectronData::new_from_settings(&coinc_data_set);

        //Opening the raw data file. We have already checked if the file opens so no worries here.
        let mut ci = 0;
        let mut file = fs::File::open(&coinc_data_set.file).unwrap();
        let mut total_size = 0;
        
        //Setting the progress bar
        let progress_size = file.metadata().unwrap().len();
        let bar = ProgressBar::new(progress_size);
        bar.set_style(ProgressStyle::with_template("[{elapsed_precise}] {bar:40.white/black} {percent}% {pos:>7}/{len:7} [ETA: {eta}] Searching electron photon coincidences")
                      .unwrap()
                      .progress_chars("=>-"));

        //This memory-bounds the problem. It means we cannot have the producer TOO fast.
        let counter = Arc::new((Mutex::new(0), Condvar::new()));

        //Producer. It is a single thread but in principle if the consumer is too slow this could
        //explose in memory. The MEMORY_BOUND_QUEUE_SIZE limits the value this thread can be ahead
        //of the consumer.
        let counter_tx = Arc::clone(&counter);
        thread::spawn( move || {
            let mut buffer: Vec<u8> = vec![0; TP3_BUFFER_SIZE];
            while let Ok(size) = file.read(&mut buffer) {
                
                //Memory-bound the thread using Condvar.
                let (lock, cvar) = &*counter_tx;
                let mut count = lock.lock().unwrap();
                while *count >= MEMORY_BOUND_QUEUE_SIZE {
                    count = cvar.wait(count).unwrap();
                }
                *count += 1;

                if size == 0 {println!("Finished Reading."); break;}
                total_size += size;
                if limit_read_size != 0 && total_size as u32 >= limit_read_size {break;}
                bar.inc(TP3_BUFFER_SIZE as u64);
                let mut channel_sender = ChannelSender::new();
                buffer[0..size].chunks_exact(8).enumerate().for_each(|(current_raw_index, pack_oct)| {
                    let packet = Packet::new(ci, packet_change(pack_oct)[0]);
                    match *pack_oct {
                        [84, 80, 88, 51, nci, _, _, _] => {
                            ci=nci;
                            channel_sender.add_packet_index(current_raw_index);
                        },
                        _ => {
                            match packet.id() {
                                6 if packet.tdc_type() == SECONDARY_TDC.associate_value() => { //Oscillator or Normal Event
                                    if let Some(fast_oscillator_tdc) = coinc_data_set.try_get_oscillator_tdc_mut() {
                                        fast_oscillator_tdc.upt(&packet);
                                    } else { //if its not synchronized measurement, this tdc is used as a event-channel.
                                        let photon = SinglePhoton::new(packet, 0, coinc_data_set.try_get_spim_tdc(), current_raw_index);
                                        channel_sender.add_photon(photon);
                                    }
                                    channel_sender.add_packet_index(current_raw_index);
                                },
                                6 if packet.tdc_type() == MAIN_TDC.associate_value() => { //Hyperspec or Normal Event
                                    if let Some(spim_tdc) = coinc_data_set.try_get_spim_tdc_mut() {
                                        spim_tdc.upt(&packet);
                                    } else { //if its not synchronized measurement, this tdc is used as a event-channel.
                                        let photon = SinglePhoton::new(packet, 1, coinc_data_set.try_get_spim_tdc(), current_raw_index);
                                        channel_sender.add_photon(photon);
                                    }
                                    channel_sender.add_packet_index(current_raw_index);
                                },
                                11 => {
                                    if let Some(oscillator_tdc) = coinc_data_set.try_get_oscillator_tdc() { //Oscillator is present
                                        if let Some(electron_time) = oscillator_tdc.tr_electron_correct_by_blanking(&packet) { //The electron time can be corrected
                                            let se = SingleElectron::new(packet, coinc_data_set.try_get_spim_tdc(), current_raw_index, Some(electron_time));
                                            channel_sender.add_electron(se);
                                        }
                                    } else {
                                        let se = SingleElectron::new(packet, coinc_data_set.try_get_spim_tdc(), current_raw_index, None);
                                        channel_sender.add_electron(se);
                                    }
                                },
                                12 => { //In some versions, the id can be a modified one, based on the CI.
                                    let packet = Packet::new(0, packet_change(pack_oct)[0]);
                                    let se = SingleElectron::new(packet, coinc_data_set.try_get_spim_tdc(), current_raw_index, None);
                                    channel_sender.add_electron(se);
                                },
                                13 => { //In some versions, the id can be a modified one, based on the CI.
                                    let packet = Packet::new(1, packet_change(pack_oct)[0]);
                                    let se = SingleElectron::new(packet, coinc_data_set.try_get_spim_tdc(), current_raw_index, None);
                                    channel_sender.add_electron(se);
                                },
                                14 => { //In some versions, the id can be a modified one, based on the CI.
                                    let packet = Packet::new(2, packet_change(pack_oct)[0]);
                                    let se = SingleElectron::new(packet, coinc_data_set.try_get_spim_tdc(), current_raw_index, None);
                                    channel_sender.add_electron(se);
                                },
                                15 => { //In some versions, the id can be a modified one, based on the CI.
                                    let packet = Packet::new(3, packet_change(pack_oct)[0]);
                                    let se = SingleElectron::new(packet, coinc_data_set.try_get_spim_tdc(), current_raw_index, None);
                                    channel_sender.add_electron(se);
                                },
                                _ => {
                                    channel_sender.add_packet_index(current_raw_index);
                                },
                            };
                        },
                    };
                });
                tx.send((channel_sender, buffer.clone())).unwrap();
            }
        });

        //Consumer
        let counter_rx = Arc::clone(&counter);
        for received in rx {
            let (mut channel_sender, buffer): (ChannelSender, Vec<u8>) = received;
            channel_sender.sort_all();
            coinc_data.add_packet_to_raw_index_from_channel_sender(&mut channel_sender); //Add standard packets
            coinc_data.add_events(&mut channel_sender, coinc_data.edata_settings.my_settings.time_delay, coinc_data.edata_settings.my_settings.time_width, 0); //Ad coincidence packets
            coinc_data.add_packets_to_reduced_data(&buffer); //Sort and exports the packets to raw_reduced_data
            coinc_data.early_output_data();
            
            let (lock, cvar) = &*counter_rx;
            let mut count = lock.lock().unwrap();
            *count -= 1;
            cvar.notify_one();
        }
        coinc_data.output_hyperspec();

    }
}

pub mod ntime_resolved {
    use crate::packetlib::Packet;
    use crate::tdclib::{TdcType, TdcRef};
    use crate::errorlib::Tp3ErrorKind;
    use crate::clusterlib::cluster::{SingleElectron, CollectionElectron, ClusterCorrectionTypes};
    use crate::auxiliar::{misc::{packet_change, output_data}, value_types::*, ConfigAcquisition, Settings, FileManager};
    use crate::constlib::*;
    use std::io::prelude::*;
    use std::convert::TryInto;
    use std::fs;
    use indicatif::{ProgressBar, ProgressStyle};

    /// This enables spatial+spectral analysis in a certain spectral window.
    pub struct TimeSpectralSpatial {
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
        remove_clusters: ClusterCorrectionTypes,
        file: String,
        fourd_data: bool,
        my_settings: Settings,
    }

    impl TimeSpectralSpatial {
        fn prepare(&mut self, file: &mut fs::File) -> Result<(), Tp3ErrorKind> {
            let mut empty_filemanager = FileManager::new_empty();
            
            if self.tdc_periodic.is_none() && self.spimx>1 && self.spimy>1 {
                self.tdc_periodic = Some(TdcRef::new_periodic(self.spim_tdc_type.clone(), file, &self.my_settings, &mut empty_filemanager).expect("Problem in creating periodic tdc ref."))
            }
            Ok(())
        }
    
        fn try_create_folder(&self) -> Result<(), Tp3ErrorKind> {
            let path_length = &self.file.len();
            match fs::create_dir(&self.file[..path_length - 5]) {
                Ok(_) => {Ok(())},
                Err(_) => { Err(Tp3ErrorKind::FolderAlreadyCreated) }
            }
        }

        fn add_electron(&mut self, packet: Packet, packet_index: usize) {
            let se = SingleElectron::new(packet, self.tdc_periodic.as_ref(), packet_index, None);
            self.ensemble.add_electron(se);
        }

        fn add_spim_tdc(&mut self, packet: Packet) {
            //Synchronizing clocks using two different approaches. It is always better to use a multiple of 2 and use the FPGA counter.
            if let Some(my_tdc_periodic) = &mut self.tdc_periodic {
                my_tdc_periodic.upt(&packet);
            }
        }
        
        fn add_extra_tdc(&mut self, _packet: Packet) {
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
                    if let Some(index) = val.get_or_not_spim_index(self.tdc_periodic.as_ref(), self.spimx, self.spimy) {
                        self.hyperspec_index.push(index);
                        self.frame_indices.push((val.spim_slice()).try_into().expect("Exceeded the maximum number of indices"));
                    }
                    
                    if let Some(index) = val.get_or_not_return_spim_index(self.tdc_periodic.as_ref(), self.spimx, self.spimy) {
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
                    if let Some(index) = val.get_or_not_4d_index(self.tdc_periodic.as_ref(), self.spimx, self.spimy) {
                        self.fourd_index.push(index);
                        self.frame_indices.push((val.spim_slice()).try_into().expect("Exceeded the maximum number of indices"));
                    }
                    
                    if let Some(index) = val.get_or_not_return_4d_index(self.tdc_periodic.as_ref(), self.spimx, self.spimy) {
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
        
        pub fn new(my_config: ConfigAcquisition, my_settings: Settings) -> Result<Self, Tp3ErrorKind> {

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

    pub fn analyze_data(data: &mut TimeSpectralSpatial) -> Result<(), Tp3ErrorKind> {
        
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
                        let packet = Packet::new(ci, packet_change(pack_oct)[0]);
                        match packet.id() {
                            6 if packet.tdc_type() == data.spim_tdc_type.associate_value() => {
                                data.add_spim_tdc(packet);
                            },
                            6 if packet.tdc_type() == data.extra_tdc_type.associate_value() => {
                                data.add_extra_tdc(packet);
                            },
                            11 => {
                                data.add_electron(packet, current_raw_index);
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
    use crate::clusterlib::cluster::{SingleElectron, CollectionElectron, ClusterCorrectionTypes};
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
                //self.cluster_size.push(electron.cluster_size());
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

    pub fn calibrate(file: &str, correction_type: &ClusterCorrectionTypes) -> io::Result<()> {

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
                        let packet = Packet::new(ci, packet_change(pack_oct)[0]);
                        if packet.id() == 11 {
                            let se = SingleElectron::new(packet, None, current_raw_index, None);
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

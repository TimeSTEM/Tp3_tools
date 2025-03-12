//!`clusterlib` is a collection of tools to identify and manipulate TPX3 cluster.

pub mod cluster {
    use crate::packetlib::Packet;
    use crate::spimlib;
    use crate::tdclib::TdcRef;
    use std::ops::{Deref, DerefMut};
    use crate::constlib::*;
    use rayon::prelude::*;
    use std::cmp::Ordering;
    use crate::auxiliar::value_types::*;
    
    fn transform_energy_calibration(v: &[u8]) -> &[f32] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const f32,
                v.len() * std::mem::size_of::<u8>() / std::mem::size_of::<f32>() )
        }
    }

    pub struct CollectionElectron {
        data: Vec<SingleElectron>,
    }
    impl Deref for CollectionElectron {
        type Target = Vec<SingleElectron>;
        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }
    impl DerefMut for CollectionElectron {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.data
        }
    }

    impl Default for CollectionElectron {
        fn default() -> Self {
            Self::new()
        }
    }

    impl CollectionElectron {
        pub fn new() -> Self {
            CollectionElectron {
                data: Vec::new(),
            }
        }
        pub fn add_electron(&mut self, electron: SingleElectron) {
            self.data.push(electron);
        }
        fn first_value(&self) -> SingleElectron {
            *self.data.iter().find(|x| x.cluster_size() == 1).unwrap()
        }
        fn remove_clusters(&mut self, correction_type: &ClusterCorrectionTypes) {
            let mut new_elist: CollectionElectron = CollectionElectron::new();
            let mut last: SingleElectron = self.first_value();
            let mut cluster_vec: Vec<SingleElectron> = Vec::new();
            for x in self.iter() {
                    //if x.cluster_size() == 1 {
                        if x.is_new_cluster(&last) {
                            if let Some(new_from_cluster) = correction_type.new_from_cluster(&cluster_vec) {
                                for electrons_in_cluster in new_from_cluster.iter() {
                                    new_elist.add_electron(*electrons_in_cluster);
                                }
                            }
                            cluster_vec.clear();
                        }
                        last = *x;
                        cluster_vec.push(*x);
                    //} else {
                    //    new_elist.add_electron(*x);
                    //}
            }
            self.data = new_elist.data;
        }

        pub fn check_if_overflow(&self) -> bool {
            let first = self.data.get(0).expect("No first value detected.");
            let last = self.data.iter().last().expect("No last value detected.");
            first.time() > last.time()
        }
        pub fn sort(&mut self) {
            self.data.par_sort_unstable();
        }
        fn clean(&mut self, correction_type: &ClusterCorrectionTypes) {
            self.remove_clusters(correction_type);
        }
        pub fn try_clean(&mut self, min_size: usize, correction_type: &ClusterCorrectionTypes) -> bool {
            if self.data.len() > min_size && correction_type.must_correct() {
                let nelectrons = self.data.len();
                self.clean(correction_type);
                let new_nelectrons = self.data.len();
                println!("Number of electrons: {}. Number of clusters: {}. Electrons per cluster: {}", nelectrons, new_nelectrons, nelectrons as f32/new_nelectrons as f32); 
                return true
            }
            !correction_type.must_correct()
        }
        pub fn clear(&mut self) {
            self.data.clear();
        }
        //This should return an Iterator so there is no need of allocating two vectors.
        pub fn search_coincidence(&self, photon_list: &CollectionPhoton, raw_packet_index: &mut Vec<usize>, min_index: &mut usize, time_delay: TIME, time_width: TIME) -> (Self, CollectionPhoton) {
            let mut corr_array = Self::new();
            let mut corr_photons = CollectionPhoton::new();
            for electron in self.iter() {
                let mut index_to_increase = None;
                let mut photons_per_electron = 0;
                for (index, photon) in photon_list.iter().skip(*min_index).enumerate() {
                    if (photon.time() / 6 < electron.time() + time_delay + time_width) && (electron.time() + time_delay < photon.time() / 6 + time_width) {
                        corr_array.add_electron(*electron);
                        corr_photons.add_photon(*photon);
                        if photons_per_electron == 0 {
                            raw_packet_index.push(electron.raw_packet_index());
                        }
                        photons_per_electron += 1;
                        if index_to_increase.is_none() { index_to_increase = Some(index); }
                    }
                    if photon.time() / 6 > electron.time() + time_delay + time_width {break;}
                }
                if let Some(increase) = index_to_increase {
                    *min_index += increase / PHOTON_LIST_STEP;
                }
            }
            (corr_array, corr_photons)
        }
        pub fn reorder_by_packet_index(&mut self) {
            self.data.par_sort_unstable_by_key(|&i| i.raw_packet_index());
        }
    }

    ///ToA, X, Y, ToT, Spim dT, Spim Slice, Cluster Size, raw packet, packet index, Spim Line
    #[derive(Copy, Clone, Eq)]
    pub struct SingleElectron {
        data: (Option<TIME>, Option<POSITION>, Option<POSITION>, Option<u16>, TIME, COUNTER, u16, Packet, usize, POSITION),
    }
    
    ///Important for sorting
    impl PartialEq for SingleElectron {
        fn eq(&self, other: &Self) -> bool {
            self.time() == other.time()
        }
    }
    impl PartialOrd for SingleElectron {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for SingleElectron {
        fn cmp(&self, other: &Self) -> Ordering {
            (self.time()).cmp(&other.time())
        }
    }

    impl SingleElectron {
        pub fn new(pack: &Packet, begin_frame: Option<TdcRef>, raw_index: usize) -> Self {
            match begin_frame {
                Some(spim_tdc) => {
                    let ele_time = spim_tdc.correct_or_not_etime(pack.electron_time()).unwrap();
                    let frame = spim_tdc.frame().unwrap_or(0);
                    SingleElectron {
                        data: (None, None, None, None, ele_time, frame, 1, *pack, raw_index, spim_tdc.current_line().unwrap())
                    }
                },
                None => {
                    SingleElectron {
                        data: (None, None, None, None, 0, 0, 1, *pack, raw_index, 0),
                    }
                },
            }
        }

        pub fn time(&self) -> TIME {
            self.data.0.unwrap_or_else(|| self.raw_packet_data().electron_time())
        }
        pub fn x(&self) -> POSITION {
            self.data.1.unwrap_or_else(|| self.raw_packet_data().x())
        }
        pub fn y(&self) -> POSITION {
            self.data.2.unwrap_or_else(|| self.raw_packet_data().y())
        }
        pub fn tot(&self) -> u16 {
            self.data.3.unwrap_or_else(|| self.raw_packet_data().tot())
        }
        pub fn frame_dt(&self) -> TIME {
            self.data.4
        } 
        pub fn spim_slice(&self) -> COUNTER {
            self.data.5
        }
        pub fn cluster_size(&self) -> u16 {
            self.data.6
        }
        pub fn raw_packet_data(&self) -> Packet {
            self.data.7
        }
        pub fn raw_packet_index(&self) -> usize {
            self.data.8
        }
        pub fn spim_line(&self) -> POSITION {
            self.data.9
        }
        pub fn image_index(&self) -> POSITION {
            self.x() + PIXELS_X*self.y()
        }
        pub fn relative_time(&self, reference_time: TIME) -> i64 {
            self.time() as i64 - reference_time as i64
        }
        //This is multiplied by six to simulate the time bin of the TDC so in this case the
        //electron bin is ~0.260 ps
        pub fn relative_time_from_abs_tdc(&self, reference_time: TIME) -> i64 {
            (self.time()*6) as i64 - reference_time as i64
        }
        fn tot_to_energy(&self) -> u16 {
            let index = self.x() as usize + 1024 * self.y() as usize;
            let temp = (self.tot() as f32 - transform_energy_calibration(BTOT)[index]) / transform_energy_calibration(ATOT)[index];
            if temp < 0.0 {
                0_u16
            } else {
                temp.round() as u16
            }
        }
        fn is_new_cluster(&self, s: &SingleElectron) -> bool {
            self.time() > s.time() + CLUSTER_DET || (self.x() as isize - s.x() as isize).abs() > CLUSTER_SPATIAL || (self.y() as isize - s.y() as isize).abs() > CLUSTER_SPATIAL
        }
        pub fn get_or_not_spim_index(&self, spim_tdc: Option<TdcRef>, xspim: POSITION, yspim: POSITION) -> Option<INDEXHYPERSPEC> {
            spimlib::get_spimindex(self.x(), self.frame_dt(), &spim_tdc?, xspim, yspim, None)
        }
        pub fn get_or_not_return_spim_index(&self, spim_tdc: Option<TdcRef>, xspim: POSITION, yspim: POSITION) -> Option<INDEXHYPERSPEC> {
            spimlib::get_return_spimindex(self.x(), self.frame_dt(), &spim_tdc?, xspim, yspim, None)
        }
        pub fn get_or_not_4d_index(&self, spim_tdc: Option<TdcRef>, xspim: POSITION, yspim: POSITION) -> Option<INDEX4D> {
            spimlib::get_4dindex(self.x(), self.y(), self.frame_dt(), &spim_tdc?, xspim, yspim, None)
        }
        pub fn get_or_not_return_4d_index(&self, spim_tdc: Option<TdcRef>, xspim: POSITION, yspim: POSITION) -> Option<INDEX4D> {
            spimlib::get_return_4dindex(self.x(), self.y(), self.frame_dt(), &spim_tdc?, xspim, yspim, None)
        }
    }

    pub struct CollectionPhoton {
        data: Vec<SinglePhoton>
    }
    impl CollectionPhoton {
        pub fn new() -> Self {
            CollectionPhoton {
                data: Vec::new(),
            }
        }
        pub fn add_photon(&mut self, photon: SinglePhoton) {
            self.data.push(photon);
        }
        pub fn len(&self) -> usize {
            self.data.len()
        }
        pub fn sort(&mut self) {
            self.data.sort_unstable();
        }
        pub fn clear(&mut self) {
            self.data.clear();
        }
    }

    //Implementing Deref means that when struct<CollectionPhoton>.iter() is called, the struc will
    //be dereferenced into self.data directly
    impl Deref for CollectionPhoton {
        type Target = Vec<SinglePhoton>;
        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }
    impl DerefMut for CollectionPhoton {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.data
        }
    }

    ///The absolute time (units of 260 ps), the channel, the g2_dT, Spim dT, raw packet, packet_index
    #[derive(Copy, Clone, Eq)]
    pub struct SinglePhoton {
        data: (TIME, COUNTER, Option<i16>, TIME, Packet, usize),
    }

    impl PartialEq for SinglePhoton {
        fn eq(&self, other: &Self) -> bool {
            self.time() == other.time()
        }
    }
    impl PartialOrd for SinglePhoton {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }
    impl Ord for SinglePhoton {
        fn cmp(&self, other: &Self) -> Ordering {
            (self.time()).cmp(&other.time())
        }
    }

    impl SinglePhoton {
        pub fn new(pack: &Packet, channel: COUNTER, begin_frame: Option<TdcRef>, raw_index: usize) -> Self {
            match begin_frame {
                Some(spim_tdc) => {
                    let tdc_time = spim_tdc.correct_or_not_etime(pack.tdc_time_norm()).unwrap();
                    SinglePhoton{
                        data: (pack.tdc_time_abs_norm(), channel, None, tdc_time, *pack, raw_index)
                    }
                }
                None => {
                    SinglePhoton{
                        data: (pack.tdc_time_abs_norm(), channel, None, 0, *pack, raw_index)
                    }
                }
            }
        }
        pub fn channel(&self) -> COUNTER {
            self.data.1
        }
        pub fn get_or_not_spim_index(&self, spim_tdc: Option<TdcRef>, xspim: POSITION, yspim: POSITION) -> Option<INDEXHYPERSPEC> {
            spimlib::get_spimindex(PIXELS_X-1, self.frame_dt(), &spim_tdc?, xspim, yspim, None)
        }
        pub fn frame_dt(&self) -> TIME {
            self.data.3
        }
        pub fn time(&self) -> TIME {
            self.data.0
        }
        pub fn g2_time(&self) -> Option<i16> {
            self.data.2
        }
        pub fn raw_packet_data(&self) -> Packet {
            self.data.4
        }
    }

    pub fn grab_cluster_correction(val: &str) -> ClusterCorrectionTypes {
        match val {
            "0" => ClusterCorrectionTypes::NoCorrection,
            "1" => ClusterCorrectionTypes::AverageCorrection,
            "2" => ClusterCorrectionTypes::LargestToT,
            "3" => ClusterCorrectionTypes::LargestToTWithThreshold(20, 100),
            "4" => ClusterCorrectionTypes::ClosestToTWithThreshold(50, 20, 100),
            //"5" => {Box::new(FixedToT(10))},
            //"6" => {Box::new(FixedToTCalibration(30, 60))},
            //"7" => {Box::new(NoCorrectionVerbose)},
            //"8" => {Box::new(SingleClusterToTCalibration)},
            _ => ClusterCorrectionTypes::NoCorrection,
        }
    }


    ///This is used for searching coincidence as a iterator, but it does not seem
    ///to be faster than normal double loop approach. This is done from backwards, by poping
    ///elements from the vectors. That's why the photon is reversed at the beginning of the struct.
    pub struct CoincidenceSearcher<'a> {
        electron: &'a mut CollectionElectron,
        photon: &'a mut CollectionPhoton,
        photon_counter: usize,
        time_delay: TIME,
        time_width: TIME,
    }

    impl<'a> CoincidenceSearcher<'a> {
        pub fn new(electron: &'a mut CollectionElectron, photon: &'a mut CollectionPhoton, time_delay: TIME, time_width: TIME) -> Self {
            photon.reverse();
            CoincidenceSearcher {
                electron,
                photon,
                photon_counter: 0,
                time_delay,
                time_width,
            }
        }
    }

    impl<'a> Iterator for CoincidenceSearcher<'a> {
        type Item = (SingleElectron, SinglePhoton);
        fn next(&mut self) -> Option<Self::Item> {
            let electron = self.electron.pop()?;
            let mut index = 0;
            for photon in self.photon.iter().skip(self.photon_counter) {
                if (photon.time() / 6 < electron.time() + self.time_delay + self.time_width) && (electron.time() + self.time_delay < photon.time() / 6 + self.time_width) {
                    self.photon_counter += index / PHOTON_LIST_STEP;
                    return Some((electron, *photon));
                }
                if photon.time() / 6 > electron.time() + self.time_delay + self.time_width {break;}
                index += 1;
            }
            self.next()
        }
    }

    #[derive(Debug)]
    pub enum ClusterCorrectionTypes {
        AverageCorrection,
        LargestToT,
        LargestToTWithThreshold(u16, u16), //Threshold min and max
        ClosestToTWithThreshold(u16, u16, u16), //Reference, Threshold min and max
        FixedToT(u16), //Reference
        FixedToTCalibration(u16, u16), //Reference
        MuonTrack,
        NoCorrection,
        NoCorrectionVerbose,
        SingleClusterToTCalibration,
    }

    impl ClusterCorrectionTypes {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {

            match &self {
                ClusterCorrectionTypes::AverageCorrection => {
                    let cluster_size = cluster.len() as u16;
                    let t_mean:TIME = cluster.iter().
                        map(|se| se.time()).
                        sum::<TIME>() / cluster_size as TIME;
                    
                    let x_mean:POSITION = cluster.iter().
                        map(|se| se.x()).
                        sum::<POSITION>() / cluster_size as POSITION;
                    
                    let y_mean:POSITION = cluster.iter().
                        map(|se| se.y()).
                        sum::<POSITION>() / cluster_size as POSITION;
                    
                    let time_dif: TIME = cluster.iter().
                        map(|se| se.frame_dt()).
                        next().
                        unwrap();
                    
                    let slice: COUNTER = cluster.iter().
                        map(|se| se.spim_slice()).
                        next().
                        unwrap();
                    
                    let tot_sum: u16 = cluster.iter().
                        map(|se| se.tot() as usize).
                        sum::<usize>() as u16;
                    
                    let raw_data = cluster.iter().
                        map(|se| se.raw_packet_data()).
                        next().
                        unwrap();
                    
                    let raw_index = cluster.iter().
                        map(|se| se.raw_packet_index()).
                        next().
                        unwrap();
                    
                    let mut val = CollectionElectron::new();
                    val.add_electron(SingleElectron {
                        data: (Some(t_mean), Some(x_mean), Some(y_mean), Some(tot_sum), time_dif, slice, cluster_size, raw_data, raw_index, 0),
                    });
                    Some(val)
                },
                ClusterCorrectionTypes::LargestToT => {
                    let cluster_size = cluster.len() as u16;

                    let electron = cluster.iter().
                        reduce(|accum, item| if accum.tot() > item.tot() {accum} else {item}).
                        unwrap();

                    let mut val = CollectionElectron::new();
                    val.add_electron(SingleElectron {
                        data: (None, None, None, None, electron.frame_dt(), electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
                    });
                    Some(val)
                }
                ClusterCorrectionTypes::LargestToTWithThreshold(min, max) => {
                    let cluster_size = cluster.len() as u16;

                    let electron = cluster.iter().
                        reduce(|accum, item| if accum.tot() > item.tot() {accum} else {item}).
                        unwrap();

                    if electron.tot() < *min {return None;}
                    if electron.tot() > *max {return None;}

                    let mut val = CollectionElectron::new();
                    val.add_electron(SingleElectron {
                        data: (None, None, None, None, electron.frame_dt(), electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
                    });
                    Some(val)
                },
                ClusterCorrectionTypes::ClosestToTWithThreshold(reference, min_th, max_th) => {
                    let cluster_size = cluster.len() as u16;

                    let electron = cluster.iter().
                        reduce(|accum, item| if (accum.tot() as i16 - *reference as i16).abs() < (item.tot() as i16 - *reference as i16).abs() {accum} else {item}).
                        unwrap();

                    if electron.tot() < *min_th {return None;}
                    if electron.tot() > *max_th {return None;}

                    let mut val = CollectionElectron::new();
                    val.add_electron(SingleElectron {
                        data: (None, None, None, None, electron.frame_dt(), electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
                    });
                    Some(val)
                },
                ClusterCorrectionTypes::FixedToT(reference) => {
                    let cluster_size = cluster.len() as u16;
                    
                    let cluster_filter_size = cluster.iter().
                        filter(|se| se.tot() == *reference).
                        count();

                    if cluster_filter_size == 0 {return None};

                    let t_mean:TIME = cluster.iter().
                        filter(|se| se.tot() == *reference).
                        map(|se| se.time()).sum::<TIME>() / cluster_filter_size as TIME;
                    
                    let x_mean:POSITION = cluster.iter().
                        map(|se| se.x()).
                        sum::<POSITION>() / cluster_size as POSITION;
                    
                    let y_mean:POSITION = cluster.iter().
                        map(|se| se.y()).
                        sum::<POSITION>() / cluster_size as POSITION;
                    
                    let time_dif: TIME = cluster.iter().
                        map(|se| se.frame_dt()).
                        next().
                        unwrap();
                    
                    let slice: COUNTER = cluster.iter().
                        map(|se| se.spim_slice()).
                        next().
                        unwrap();
                    
                    let tot_sum: u16 = cluster.iter().
                        map(|se| se.tot() as usize).
                        sum::<usize>() as u16;
                    
                    let raw_packet_data: Packet = cluster.iter().
                        map(|se| se.raw_packet_data()).
                        next().
                        unwrap();
                    
                    let raw_packet_index: usize = cluster.iter().
                        map(|se| se.raw_packet_index()).
                        next().
                        unwrap();

                    let mut val = CollectionElectron::new();
                    val.add_electron(SingleElectron{
                        data: (Some(t_mean), Some(x_mean), Some(y_mean), Some(tot_sum), time_dif, slice, cluster_size, raw_packet_data, raw_packet_index, 0),
                    });
                    Some(val)
                },
                ClusterCorrectionTypes::FixedToTCalibration(energy_ref, energy_sum_ref) => {
                    let cluster_size = cluster.len() as u16;

                    if (cluster_size < 3) || (cluster_size > 4) {return None;} //3 to 4 objects in the cluster
                    
                    let cluster_filter_size = cluster.iter().
                        filter(|se| se.tot_to_energy() == *energy_ref).
                        count(); //Number of elements in the reference energy value
                    
                    if cluster_filter_size != 1 {return None}; //It must be one for complete control
                    
                    let energy_sum: u16 = cluster.iter().
                        map(|se| se.tot_to_energy() as usize).
                        sum::<usize>() as u16;

                    if energy_sum < energy_sum_ref - 15 {return None;} //The energy sum must be close to the electron energy value
                    if energy_sum > energy_sum_ref + 15 {return None;}

                    let time_reference = cluster.iter().
                        filter(|se| se.tot_to_energy() == *energy_ref).
                        map(|se| se.time()).
                        next().
                        unwrap();

                    let mut val = CollectionElectron::new();
                    for electron in cluster {
                        //if electron.tot_to_energy() == self.0 {continue;} //ToT reference not need to be output
                        let time_diference = electron.time() as i64 - time_reference as i64;
                        if time_diference.abs() > 100 {continue;} //must not output far-away data from tot==reference value
                        val.add_electron(SingleElectron{
                            data: (None, None, None, Some(electron.tot_to_energy()), time_reference, electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
                        });
                    }
                    Some(val)
                },
                ClusterCorrectionTypes::MuonTrack => {
                    let cluster_size = cluster.len() as u16;
                    if cluster_size == 1 {return None;}
                    let time_reference = cluster.iter().
                        map(|se| se.time()).
                        next().
                        unwrap();

                    let mut val = CollectionElectron::new();
                    for electron in cluster {
                        //let time_diference = electron.time() as i64 - time_reference as i64;
                        val.add_electron(SingleElectron{
                            data: (None, None, None, None, time_reference, electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
                        });
                    }
                    Some(val)
                },
                ClusterCorrectionTypes::NoCorrection => {
                    let mut val = CollectionElectron::new();
                    for electron in cluster {
                        val.add_electron(SingleElectron{
                            data: (None, None, None, None, electron.frame_dt(), electron.spim_slice(), 1, electron.raw_packet_data(), electron.raw_packet_index(), 0),
                        });
                    }
                    Some(val)
                },
                ClusterCorrectionTypes::NoCorrectionVerbose => {
                    let cluster_size = cluster.len() as u16;
                    let mut val = CollectionElectron::new();
                    for electron in cluster {
                        val.add_electron(SingleElectron{
                            data: (None, None, None, None, electron.frame_dt(), electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
                    });
                    }
                    Some(val)
                },
                ClusterCorrectionTypes::SingleClusterToTCalibration => {
                    let cluster_size = cluster.len() as u16;
                    if cluster_size != 1 {return None}; //It must be single cluster
                    let mut val = CollectionElectron::new();
                    for electron in cluster {
                        val.add_electron(SingleElectron {
                            data: (None, None, None, None, electron.frame_dt(), electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
                        });
                    }
                    Some(val)
                },
            }

        }
        fn must_correct(&self) -> bool {
            match &self {
                ClusterCorrectionTypes::NoCorrection => {false},
                _ => true,
            }
        }
        pub fn set_thresholds(&mut self, min_value: u16, max_value: u16) {
            match self {
                ClusterCorrectionTypes::LargestToTWithThreshold(min, max) => {*min = min_value; *max=max_value;},
                ClusterCorrectionTypes::ClosestToTWithThreshold(_, min, max) => {*min = min_value; *max = max_value},
                _ => {},
            }
        }
        pub fn set_reference(&mut self, reference: u16) {
            match self {
                ClusterCorrectionTypes::FixedToT(ref_value) => {*ref_value = reference}
                ClusterCorrectionTypes::ClosestToTWithThreshold(ref_value, _, _) => {*ref_value = reference}
                ClusterCorrectionTypes::FixedToTCalibration(ref_value, _) => {*ref_value = reference}
                _ => {},
            }
        }
    }



    /*
    pub struct AverageCorrection; //
    pub struct LargestToT; //
    pub struct LargestToTWithThreshold(pub u16, pub u16); //Threshold min and max
    pub struct ClosestToTWithThreshold(pub u16, pub u16, pub u16); //Reference, Threshold min and max
    pub struct FixedToT(pub u16); //Reference
    pub struct FixedToTCalibration(pub u16, pub u16); //Reference
    pub struct MuonTrack; //
    pub struct NoCorrection; //
    pub struct NoCorrectionVerbose; //
    pub struct SingleClusterToTCalibration; //

    pub trait ClusterCorrection: {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron>;
        fn set_reference(&mut self, _reference: u16) {}
        fn set_thresholds(&mut self, _min_val: u16, _max_val: u16) {}
        fn must_correct(&self) -> bool {true}
    }

    
    impl<S: ClusterCorrection + ?Sized> ClusterCorrection for Box<S> {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            (**self).new_from_cluster(cluster)
        }
        fn set_reference(&mut self, reference: u16) {
            (**self).set_reference(reference);
        }
        fn set_thresholds(&mut self, min_val: u16, max_val: u16) {
            (**self).set_thresholds(min_val, max_val);
        }
        fn must_correct(&self) -> bool {
            (**self).must_correct()
        }
    }
    

    impl ClusterCorrection for AverageCorrection {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as u16;

            let t_mean:TIME = cluster.iter().
                map(|se| se.time()).
                sum::<TIME>() / cluster_size as TIME;
            
            let x_mean:POSITION = cluster.iter().
                map(|se| se.x()).
                sum::<POSITION>() / cluster_size as POSITION;
            
            let y_mean:POSITION = cluster.iter().
                map(|se| se.y()).
                sum::<POSITION>() / cluster_size as POSITION;
            
            let time_dif: TIME = cluster.iter().
                map(|se| se.frame_dt()).
                next().
                unwrap();
            
            let slice: COUNTER = cluster.iter().
                map(|se| se.spim_slice()).
                next().
                unwrap();
            
            let tot_sum: u16 = cluster.iter().
                map(|se| se.tot() as usize).
                sum::<usize>() as u16;
            
            let raw_data = cluster.iter().
                map(|se| se.raw_packet_data()).
                next().
                unwrap();
            
            let raw_index = cluster.iter().
                map(|se| se.raw_packet_index()).
                next().
                unwrap();
            
            let mut val = CollectionElectron::new();
            val.add_electron(SingleElectron {
                data: (Some(t_mean), Some(x_mean), Some(y_mean), Some(tot_sum), time_dif, slice, cluster_size, raw_data, raw_index, 0),
            });
            Some(val)
        }


    }
    
    impl ClusterCorrection for LargestToT {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as u16;

            let electron = cluster.iter().
                reduce(|accum, item| if accum.tot() > item.tot() {accum} else {item}).
                unwrap();

            let mut val = CollectionElectron::new();
            val.add_electron(SingleElectron {
                data: (None, None, None, None, electron.frame_dt(), electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
            });
            Some(val)
        }
    }
    
    impl ClusterCorrection for LargestToTWithThreshold {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as u16;

            let electron = cluster.iter().
                reduce(|accum, item| if accum.tot() > item.tot() {accum} else {item}).
                unwrap();

            if electron.tot() < self.0 {return None;}
            if electron.tot() > self.1 {return None;}

            let mut val = CollectionElectron::new();
            val.add_electron(SingleElectron {
                data: (None, None, None, None, electron.frame_dt(), electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
            });
            Some(val)
        }

        fn set_thresholds(&mut self, min_val: u16, max_val: u16) {
            self.0 = min_val;
            self.1 = max_val;
        }
    }
    
    impl ClusterCorrection for ClosestToTWithThreshold {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as u16;

            let electron = cluster.iter().
                reduce(|accum, item| if (accum.tot() as i16 - self.0 as i16).abs() < (item.tot() as i16 - self.0 as i16).abs() {accum} else {item}).
                unwrap();

            if electron.tot() < self.1 {return None;}
            if electron.tot() > self.2 {return None;}

            let mut val = CollectionElectron::new();
            val.add_electron(SingleElectron {
                data: (None, None, None, None, electron.frame_dt(), electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
            });
            Some(val)
        }

        fn set_reference(&mut self, reference: u16) {
            self.0 = reference;
        }
        
        fn set_thresholds(&mut self, min_val: u16, max_val: u16) {
            self.1 = min_val;
            self.2 = max_val;
        }
    }

    impl ClusterCorrection for FixedToT {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as u16;
            
            let cluster_filter_size = cluster.iter().
                filter(|se| se.tot() == self.0).
                count();

            if cluster_filter_size == 0 {return None};

            let t_mean:TIME = cluster.iter().
                filter(|se| se.tot() == self.0).
                map(|se| se.time()).sum::<TIME>() / cluster_filter_size as TIME;
            
            let x_mean:POSITION = cluster.iter().
                map(|se| se.x()).
                sum::<POSITION>() / cluster_size as POSITION;
            
            let y_mean:POSITION = cluster.iter().
                map(|se| se.y()).
                sum::<POSITION>() / cluster_size as POSITION;
            
            let time_dif: TIME = cluster.iter().
                map(|se| se.frame_dt()).
                next().
                unwrap();
            
            let slice: COUNTER = cluster.iter().
                map(|se| se.spim_slice()).
                next().
                unwrap();
            
            let tot_sum: u16 = cluster.iter().
                map(|se| se.tot() as usize).
                sum::<usize>() as u16;
            
            let raw_packet_data: Packet = cluster.iter().
                map(|se| se.raw_packet_data()).
                next().
                unwrap();
            
            let raw_packet_index: usize = cluster.iter().
                map(|se| se.raw_packet_index()).
                next().
                unwrap();

            let mut val = CollectionElectron::new();
            val.add_electron(SingleElectron{
                data: (Some(t_mean), Some(x_mean), Some(y_mean), Some(tot_sum), time_dif, slice, cluster_size, raw_packet_data, raw_packet_index, 0),
            });
            Some(val)
        }
        
        fn set_reference(&mut self, reference: u16) {
            self.0 = reference;
        }
    }
    impl ClusterCorrection for FixedToTCalibration {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as u16;

            if (cluster_size < 3) || (cluster_size > 4) {return None;} //3 to 4 objects in the cluster
            
            let cluster_filter_size = cluster.iter().
                filter(|se| se.tot_to_energy() == self.0).
                count(); //Number of elements in the reference energy value
            
            if cluster_filter_size != 1 {return None}; //It must be one for complete control
            
            let energy_sum: u16 = cluster.iter().
                map(|se| se.tot_to_energy() as usize).
                sum::<usize>() as u16;

            if energy_sum < self.1 - 15 {return None;} //The energy sum must be close to the electron energy value
            if energy_sum > self.1 + 15 {return None;}

            let time_reference = cluster.iter().
                filter(|se| se.tot_to_energy() == self.0).
                map(|se| se.time()).
                next().
                unwrap();

            let mut val = CollectionElectron::new();
            for electron in cluster {
                //if electron.tot_to_energy() == self.0 {continue;} //ToT reference not need to be output
                let time_diference = electron.time() as i64 - time_reference as i64;
                if time_diference.abs() > 100 {continue;} //must not output far-away data from tot==reference value
                val.add_electron(SingleElectron{
                    data: (None, None, None, Some(electron.tot_to_energy()), time_reference, electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
                });
            }
            Some(val)
        }
        fn set_reference(&mut self, reference: u16) {
            self.0 = reference;
        }
    }
    impl ClusterCorrection for MuonTrack {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as u16;
            
            if cluster_size == 1 {return None;}

            let time_reference = cluster.iter().
                map(|se| se.time()).
                next().
                unwrap();

            let mut val = CollectionElectron::new();
            for electron in cluster {
                //let time_diference = electron.time() as i64 - time_reference as i64;
                val.add_electron(SingleElectron{
                    data: (None, None, None, None, time_reference, electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
                });
            }
            Some(val)
        }
    }

    impl ClusterCorrection for NoCorrection {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let mut val = CollectionElectron::new();
            for electron in cluster {
                val.add_electron(SingleElectron{
                    data: (None, None, None, None, electron.frame_dt(), electron.spim_slice(), 1, electron.raw_packet_data(), electron.raw_packet_index(), 0),
            });
            }
            Some(val)
        }
        fn must_correct(&self) -> bool {false}
    }
    impl ClusterCorrection for NoCorrectionVerbose {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as u16;
            let mut val = CollectionElectron::new();
            for electron in cluster {
                val.add_electron(SingleElectron{
                    data: (None, None, None, None, electron.frame_dt(), electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
            });
            }
            Some(val)
        }
    }
    impl ClusterCorrection for SingleClusterToTCalibration {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as u16;
            if cluster_size != 1 {return None}; //It must be single cluster
            let mut val = CollectionElectron::new();
            for electron in cluster {
                val.add_electron(SingleElectron {
                    data: (None, None, None, None, electron.frame_dt(), electron.spim_slice(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
                });
            }
            Some(val)
        }
    }
    */
}

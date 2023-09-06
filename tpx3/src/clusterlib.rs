//!`clusterlib` is a collection of tools to identify and manipulate TPX3 cluster.

pub mod cluster {
    use crate::packetlib::Packet;
    use crate::spimlib;
    use crate::tdclib::PeriodicTdcRef;
    use std::fs::OpenOptions;
    use std::io::Write;
    use std::ops::Deref;
    use crate::constlib::*;
    use rayon::prelude::*;
    use crate::auxiliar::value_types::*;
    
    const CLUSTER_DET: TIME = 128; //Cluster time window (in 640 Mhz or 1.5625).
    const CLUSTER_SPATIAL: isize = 256; // If electron hit position in both X or Y > CLUSTER_SPATIAL, then we have a new cluster.
    
    static ATOT: &[u8; 1024 * 256 * 4] = include_bytes!("atot_v2.dat");
    static BTOT: &[u8; 1024 * 256 * 4] = include_bytes!("btot_v2.dat");
    
    /*
    fn as_bytes<T>(v: &[T]) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const u8,
                v.len() * std::mem::size_of::<T>())
        }
    }
    
    fn transform_time_shift(v: &[u8]) -> &[i16] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const i16,
                v.len() * std::mem::size_of::<u8>() / std::mem::size_of::<i16>() )
        }
    }
    
    fn transform_time_walk(v: &[u8]) -> &[i16] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const i16,
                v.len() * std::mem::size_of::<u8>() / std::mem::size_of::<i16>() )
        }
    }
    */
    
    fn transform_energy_calibration(v: &[u8]) -> &[f32] {
        unsafe {
            std::slice::from_raw_parts(
                v.as_ptr() as *const f32,
                v.len() * std::mem::size_of::<u8>() / std::mem::size_of::<f32>() )
        }
    }


    
    /*
    fn read_time_shift() -> Vec<u8> {
        let mut shift_array: Vec<u8> = vec![0; 1024 * 256 * 2];
        let mut tfile = OpenOptions::new()
            .read(true)
            .open("time_shift.dat")
            .unwrap();
        tfile.read(&mut shift_array).unwrap();
        //println!("{:?}", shift_array);
        shift_array
    }
    */

    #[derive(Debug)]
    pub struct CollectionElectron {
        data: Vec<SingleElectron>,
    }
    impl CollectionElectron {

        pub fn values(&self) -> Inspector {
            Inspector{iter: self.data.iter()}
        }

        pub fn first_value(&self) -> SingleElectron {
            //*self.data.iter().filter(|x| x.cluster_size() == 1).next().unwrap()
            *self.data.iter().find(|x| x.cluster_size() == 1).unwrap()
        }
    }

    
    impl Deref for CollectionElectron {
        type Target = Vec<SingleElectron>;

        fn deref(&self) -> &Self::Target {
            &self.data
        }
    }

    pub struct Inspector<'a> {
        iter: std::slice::Iter<'a, SingleElectron>,
    }

    impl<'a> Iterator for Inspector<'a> {
        type Item = &'a SingleElectron;

        fn next(&mut self) -> Option<Self::Item> {
            self.iter.next()
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
        
        fn remove_clusters<T: ClusterCorrection>(&mut self, correction_type: &T) {
            
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

        pub fn correct_electron_time(&mut self, overflow: COUNTER) -> bool {
            if self.check_if_overflow() {
                let overflow_index = self.data.iter().
                    enumerate().
                    min_by_key(|x| x.1.time()).unwrap().0;
                self.data[0..overflow_index].iter_mut().for_each(|se| se.correct_time_overflow(overflow));
                self.data[overflow_index..].iter_mut().for_each(|se| se.correct_time_overflow(overflow+1));
                true
            } else {
                self.data.iter_mut().for_each(|se| se.correct_time_overflow(overflow));
                false
            }
        }

        pub fn sort(&mut self) {
            self.data.par_sort_unstable_by(|a, b| (a.data).partial_cmp(&b.data).unwrap());
        }

        fn clean<T: ClusterCorrection>(&mut self, correction_type: &T) {
            //self.sort();
            self.remove_clusters(correction_type);
            //for _x in 0..10 {
            //    self.remove_clusters(correction_type);
            //}
        }

        pub fn try_clean<T: ClusterCorrection>(&mut self, min_size: usize, correction_type: &T) -> bool {
            if self.data.len() > min_size && correction_type.must_correct() {
                let _nelectrons = self.data.len();
                self.clean(correction_type);
                let _new_nelectrons = self.data.len();
                //println!("Number of electrons: {}. Number of clusters: {}. Electrons per cluster: {}", nelectrons, new_nelectrons, nelectrons as f32/new_nelectrons as f32); 
                return true
            }
            !correction_type.must_correct()
        }
        
        pub fn clear(&mut self) {
            self.data.clear();
        }

        pub fn output_data(&self, filename: String, slice: COUNTER) {
            let mut tfile = OpenOptions::new()
                .append(true)
                .create(true)
                .open(&filename).expect("Could not output time histogram.");
            let out: Vec<String> = self.data.iter().filter(|se| se.spim_slice()==slice && se.tot() > 60 && se.tot() < 220).map(|x| x.to_string()).collect::<Vec<String>>();
            if !out.is_empty() {
                println!("Outputting data for slice {}. Number of electrons: {}", slice, out.len());
                let out_str: String = out.join("");
                tfile.write_all(out_str.as_ref()).expect("Could not write time to file.");
            }
        }
    }

    ///ToA, X, Y, Spim dT, Spim Slice, ToT, Cluster Size, Copied raw header, Copied raw packet, raw
    ///packet index, Spim Line
    #[derive(Copy, Clone, Debug)]
    pub struct SingleElectron {
        data: (TIME, POSITION, POSITION, TIME, COUNTER, u16, COUNTER, u64, usize, POSITION),
    }

    impl ToString for SingleElectron {
        fn to_string(&self) -> String {

            let mut val = self.time().to_string();
            val.push(',');
            val.push_str(&self.x().to_string());
            val.push(',');
            val.push_str(&self.y().to_string());
            val.push(',');
            val.push_str(&self.frame_dt().to_string());
            val.push(',');
            val.push_str(&self.spim_slice().to_string());
            val.push(',');
            val.push_str(&self.tot().to_string());
            val.push(',');
            val.push_str(&self.cluster_size().to_string());
            val.push(',');
            
            val
        }
    }

    impl SingleElectron {
        pub fn new<T: Packet + ?Sized>(pack: &T, begin_frame: Option<PeriodicTdcRef>, raw_index: usize) -> Self {
            match begin_frame {
                Some(spim_tdc) => {
                    let ele_time = spimlib::correct_or_not_etime(pack.electron_time(), &spim_tdc);
                    SingleElectron {
                        data: (pack.electron_time(), pack.x(), pack.y(), ele_time-spim_tdc.begin_frame-VIDEO_TIME, spim_tdc.frame(), pack.tot(), 1, pack.data(), raw_index, spim_tdc.current_line().unwrap())
                        //data: (pack.electron_time(), pack.x(), pack.y(), ele_time-spim_tdc.time()-VIDEO_TIME, spim_tdc.frame(), pack.tot(), 1, pack.create_header(), pack.data(), raw_index, spim_tdc.current_line().unwrap())
                    }
                },
                None => {
                    SingleElectron {
                        data: (pack.electron_time(), pack.x(), pack.y(), 0, 0, pack.tot(), 1, pack.data(), raw_index, 0),
                    }
                },
            }
        }

        pub fn correct_time_overflow(&mut self, overflow: COUNTER) {
            self.data.0 += overflow as TIME * ELECTRON_OVERFLOW;
        }
        pub fn x(&self) -> POSITION {
            self.data.1
        }
        pub fn y(&self) -> POSITION {
            self.data.2
        }
        pub fn time(&self) -> TIME {
            self.data.0
        }
        pub fn tot(&self) -> u16 {
            self.data.5
        }
        pub fn frame_dt(&self) -> TIME {
            self.data.3
        }
        pub fn image_index(&self) -> POSITION {
            self.data.1 + SPIM_PIXELS*self.data.2
        }
        pub fn relative_time(&self, reference_time: TIME) -> i64 {
            self.data.0 as i64 - reference_time as i64
        }
        //This is multiplied by six to simulate the time bin of the TDC so in this case the
        //electron bin is ~0.260 ps
        pub fn relative_time_from_abs_tdc(&self, reference_time: TIME) -> i64 {
            (self.data.0*6) as i64 - reference_time as i64
        }
        fn tot_to_energy(&self) -> u16 {
            let index = self.x() as usize + 1024 * self.y() as usize;
            let temp = (self.tot() as f32 - transform_energy_calibration(BTOT)[index]) / transform_energy_calibration(ATOT)[index];
            if temp < 0.0 {
                0 as u16
            } else {
                temp.round() as u16
            }
        }

        pub fn spim_slice(&self) -> COUNTER {
            self.data.4
        }
        pub fn cluster_size(&self) -> COUNTER {
            self.data.6
        }
        
        pub fn raw_packet_data(&self) -> u64 {
            self.data.7
        }

        pub fn raw_packet_index(&self) -> usize {
            self.data.8
        }
        
        pub fn spim_line(&self) -> POSITION {
            self.data.9
        }

        fn is_new_cluster(&self, s: &SingleElectron) -> bool {
            self.time() > s.time() + CLUSTER_DET || (self.x() as isize - s.x() as isize).abs() > CLUSTER_SPATIAL || (self.y() as isize - s.y() as isize).abs() > CLUSTER_SPATIAL
        }
        
        pub fn get_or_not_spim_index(&self, spim_tdc: Option<PeriodicTdcRef>, xspim: POSITION, yspim: POSITION) -> Option<POSITION> {
            spimlib::get_spimindex(self.x(), self.frame_dt(), &spim_tdc?, xspim, yspim)
        }
        
        pub fn get_or_not_spim_index_using_line(&self, spim_tdc: Option<PeriodicTdcRef>, line: POSITION, xspim: POSITION, yspim: POSITION) -> Option<POSITION> {
            spimlib::get_spimindex_using_line(self.x(), self.frame_dt(), line, &spim_tdc?, xspim, yspim)
        }
        
        pub fn get_or_not_return_spim_index(&self, spim_tdc: Option<PeriodicTdcRef>, xspim: POSITION, yspim: POSITION) -> Option<POSITION> {
            spimlib::get_return_spimindex(self.x(), self.frame_dt(), &spim_tdc?, xspim, yspim)
        }
        pub fn get_or_not_4d_index(&self, spim_tdc: Option<PeriodicTdcRef>, xspim: POSITION, yspim: POSITION) -> Option<u64> {
            spimlib::get_4dindex(self.x(), self.y(), self.frame_dt(), &spim_tdc?, xspim, yspim)
        }
        pub fn get_or_not_return_4d_index(&self, spim_tdc: Option<PeriodicTdcRef>, xspim: POSITION, yspim: POSITION) -> Option<u64> {
            spimlib::get_return_4dindex(self.x(), self.y(), self.frame_dt(), &spim_tdc?, xspim, yspim)
        }
    }

    pub fn grab_cluster_correction(val: &str) -> Box<dyn ClusterCorrection> {
        match val {
            "0" => {Box::new(NoCorrection)},
            "1" => {Box::new(AverageCorrection)},
            "2" => {Box::new(LargestToT)},
            "3" => {Box::new(LargestToTWithThreshold(20, 100))},
            "4" => {Box::new(ClosestToTWithThreshold(50, 20, 100))},
            "5" => {Box::new(FixedToT(10))},
            "6" => {Box::new(FixedToTCalibration(30, 60))},
            "7" => {Box::new(NoCorrectionVerbose)},
            "8" => {Box::new(SingleClusterToTCalibration)},
            _ => {Box::new(NoCorrection)},
        }
    }
    

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
            let cluster_size = cluster.len() as COUNTER;

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
                data: (t_mean, x_mean, y_mean, time_dif, slice, tot_sum, cluster_size, raw_data, raw_index, 0),
            });
            Some(val)
        }


    }
    
    impl ClusterCorrection for LargestToT {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as COUNTER;

            let electron = cluster.iter().
                reduce(|accum, item| if accum.tot() > item.tot() {accum} else {item}).
                unwrap();

            let mut val = CollectionElectron::new();
            val.add_electron(SingleElectron {
                data: (electron.time(), electron.x(), electron.y(), electron.frame_dt(), electron.spim_slice(), electron.tot(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
            });
            Some(val)
        }
    }
    
    impl ClusterCorrection for LargestToTWithThreshold {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as COUNTER;

            let electron = cluster.iter().
                reduce(|accum, item| if accum.tot() > item.tot() {accum} else {item}).
                unwrap();

            if electron.tot() < self.0 {return None;}
            if electron.tot() > self.1 {return None;}

            let mut val = CollectionElectron::new();
            val.add_electron(SingleElectron {
                data: (electron.time(), electron.x(), electron.y(), electron.frame_dt(), electron.spim_slice(), electron.tot(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
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
            let cluster_size = cluster.len() as COUNTER;

            let electron = cluster.iter().
                reduce(|accum, item| if (accum.tot() as i16 - self.0 as i16).abs() < (item.tot() as i16 - self.0 as i16).abs() {accum} else {item}).
                unwrap();

            if electron.tot() < self.1 {return None;}
            if electron.tot() > self.2 {return None;}

            let mut val = CollectionElectron::new();
            val.add_electron(SingleElectron {
                data: (electron.time(), electron.x(), electron.y(), electron.frame_dt(), electron.spim_slice(), electron.tot(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
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
            let cluster_size = cluster.len() as COUNTER;
            
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
            
            let raw_packet_data: u64 = cluster.iter().
                map(|se| se.raw_packet_data()).
                next().
                unwrap();
            
            let raw_packet_index: usize = cluster.iter().
                map(|se| se.raw_packet_index()).
                next().
                unwrap();

            let mut val = CollectionElectron::new();
            val.add_electron(SingleElectron{
                data: (t_mean, x_mean, y_mean, time_dif, slice, tot_sum, cluster_size, raw_packet_data, raw_packet_index, 0),
            });
            Some(val)
        }
        
        fn set_reference(&mut self, reference: u16) {
            self.0 = reference;
        }
    }
    impl ClusterCorrection for FixedToTCalibration {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as COUNTER;

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
                    data: (electron.time(), electron.x(), electron.y(), time_reference, electron.spim_slice(), electron.tot_to_energy(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
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
            let cluster_size = cluster.len() as COUNTER;
            
            if cluster_size == 1 {return None;}

            let time_reference = cluster.iter().
                map(|se| se.time()).
                next().
                unwrap();

            let mut val = CollectionElectron::new();
            for electron in cluster {
                //let time_diference = electron.time() as i64 - time_reference as i64;
                val.add_electron(SingleElectron{
                    data: (electron.time(), electron.x(), electron.y(), time_reference, electron.spim_slice(), electron.tot(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
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
                    data: (electron.time(), electron.x(), electron.y(), electron.frame_dt(), electron.spim_slice(), electron.tot(), 1, electron.raw_packet_data(), electron.raw_packet_index(), 0),
            });
            }
            Some(val)
        }
        fn must_correct(&self) -> bool {false}
    }
    impl ClusterCorrection for NoCorrectionVerbose {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as COUNTER;
            let mut val = CollectionElectron::new();
            for electron in cluster {
                val.add_electron(SingleElectron{
                    data: (electron.time(), electron.x(), electron.y(), electron.frame_dt(), electron.spim_slice(), electron.tot(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
            });
            }
            Some(val)
        }
    }
    impl ClusterCorrection for SingleClusterToTCalibration {
        fn new_from_cluster(&self, cluster: &[SingleElectron]) -> Option<CollectionElectron> {
            let cluster_size = cluster.len() as COUNTER;
            if cluster_size != 1 {return None}; //It must be single cluster
            let mut val = CollectionElectron::new();
            for electron in cluster {
                val.add_electron(SingleElectron {
                    data: (electron.time(), electron.x(), electron.y(), electron.frame_dt(), electron.spim_slice(), electron.tot(), cluster_size, electron.raw_packet_data(), electron.raw_packet_index(), 0),
                });
            }
            Some(val)
        }
    }
}

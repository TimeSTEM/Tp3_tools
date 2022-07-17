//!`clusterlib` is a collection of tools to identify and manipulate TPX3 cluster.

pub mod cluster {
    use crate::spimlib::{SPIM_PIXELS, VIDEO_TIME};
    use crate::packetlib::{Packet, PacketEELS as Pack};
    use crate::spimlib;
    use crate::tdclib::PeriodicTdcRef;
    use std::fs::OpenOptions;
    use std::io::Write;
    use rayon::prelude::*;
    use crate::auxiliar::value_types::*;
    
    const CLUSTER_DET: TIME = 128; //Cluster time window (in 640 Mhz or 1.5625).
    const CLUSTER_SPATIAL: isize = 2; // If electron hit position in both X or Y > CLUSTER_SPATIAL, then we have a new cluster.

    #[derive(Debug)]
    pub struct CollectionElectron {
        data: Vec<SingleElectron>,
    }
    impl CollectionElectron {

        pub fn values(&self) -> Inspector {
            Inspector{iter: self.data.iter()}
        }

        pub fn first_value(&self) -> SingleElectron {
            *self.data.iter().filter(|x| x.cluster_size() == 1).next().unwrap()
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

    impl CollectionElectron {
        pub fn new() -> Self {
            CollectionElectron {
                data: Vec::new(),
            }
        }
        pub fn add_electron(&mut self, electron: SingleElectron) {
            self.data.push(electron);
        }
        
        fn remove_clusters(&mut self) {
            
            let mut nelist: Vec<SingleElectron> = Vec::new();
            let mut last: SingleElectron = self.first_value();
            let mut cluster_vec: Vec<SingleElectron> = Vec::new();
            
            for x in self.values() {
                    //if x.cluster_size() == 1 {
                        if x.is_new_cluster(&last) {
                            //if let Some(new_from_cluster) = SingleElectron::new_from_cluster_fixed_tot(&cluster_vec, 40) {
                            if let Some(new_from_cluster) = SingleElectron::new_from_cluster(&cluster_vec) {
                                nelist.push(new_from_cluster);
                            }
                            cluster_vec.clear();
                        }
                        last = *x;
                        cluster_vec.push(*x);
                    //} else {
                    //    nelist.push(*x);
                    //}
            }
            self.data = nelist;
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

        fn clean(&mut self) {
            //self.sort();
            self.remove_clusters();
            //for _x in 0..2 {
            //    self.remove_clusters();
            //}
        }

        pub fn try_clean(&mut self, min_size: usize, remove: bool) -> bool {
            if self.data.len() > min_size && remove {
                let nelectrons = self.data.len();
                self.clean();
                let new_nelectrons = self.data.len();
                println!("Number of electrons: {}. Number of clusters: {}. Electrons per cluster: {}", nelectrons, new_nelectrons, nelectrons as f32/new_nelectrons as f32); 
                return true
            }
            !remove
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
            if out.len() > 0 {
                println!("Outputting data for slice {}. Number of electrons: {}", slice, out.len());
                let out_str: String = out.join("");
                tfile.write_all(out_str.as_ref()).expect("Could not write time to file.");
            }
        }

        /*
        pub fn output_time(&self, mut filename: String, code: usize) {
            filename.push_str(&code.to_string());
            let mut tfile = OpenOptions::new()
                .append(false)
                .write(true)
                .truncate(true)
                .create(true)
                .open(&filename).expect("Could not output time histogram.");
            let out: String = self.data.iter().map(|x| x.time().to_string()).collect::<Vec<String>>().join(",");
            tfile.write_all(out.as_ref()).expect("Could not write time to file.");
        }
        pub fn output_x(&self, mut filename: String, code: usize) {
            filename.push_str(&code.to_string());
            let mut tfile = OpenOptions::new()
                .append(false)
                .write(true)
                .truncate(true)
                .create(true)
                .open(&filename).expect("Could not output x histogram.");
            let out: String = self.data.iter().map(|x| x.x().to_string()).collect::<Vec<String>>().join(",");
            tfile.write_all(out.as_ref()).expect("Could not write time to file.");
        }
        pub fn output_y(&self, mut filename: String, code: usize) {
            filename.push_str(&code.to_string());
            let mut tfile = OpenOptions::new()
                .append(false)
                .write(true)
                .truncate(true)
                .create(true)
                .open(&filename).expect("Could not output x histogram.");
            let out: String = self.data.iter().map(|x| x.y().to_string()).collect::<Vec<String>>().join(",");
            tfile.write_all(out.as_ref()).expect("Could not write time to file.");
        }
        pub fn output_tot(&self, mut filename: String, code: usize) {
            filename.push_str(&code.to_string());
            let mut tfile = OpenOptions::new()
                .append(false)
                .write(true)
                .truncate(true)
                .create(true)
                .open(&filename).expect("Could not output x histogram.");
            let out: String = self.data.iter().map(|x| x.tot().to_string()).collect::<Vec<String>>().join(",");
            tfile.write_all(out.as_ref()).expect("Could not write time to file.");
        }
        pub fn output_cluster_size(&self, mut filename: String, code: usize) {
            filename.push_str(&code.to_string());
            let mut tfile = OpenOptions::new()
                .append(false)
                .write(true)
                .truncate(true)
                .create(true)
                .open(&filename).expect("Could not output x histogram.");
            let out: String = self.data.iter().map(|x| x.cluster_size().to_string()).collect::<Vec<String>>().join(",");
            tfile.write_all(out.as_ref()).expect("Could not write time to file.");
        }
        */
    }

    ///ToA, X, Y, Spim dT, Spim Slice, ToT, Cluster Size
    #[derive(Copy, Clone, Debug)]
    pub struct SingleElectron {
        data: (TIME, POSITION, POSITION, TIME, COUNTER, u16, usize),
    }

    impl ToString for SingleElectron {
        fn to_string(&self) -> String {

            let mut val = String::from(self.time().to_string());
            val.push_str(",");
            val.push_str(&self.x().to_string());
            val.push_str(",");
            val.push_str(&self.y().to_string());
            val.push_str(",");
            val.push_str(&self.frame_dt().to_string());
            val.push_str(",");
            val.push_str(&self.spim_slice().to_string());
            val.push_str(",");
            val.push_str(&self.tot().to_string());
            val.push_str(",");
            val.push_str(&self.cluster_size().to_string());
            val.push_str(",");
            
            val
        }
    }

    impl SingleElectron {
        pub fn new<T: Packet>(pack: &T, begin_frame: Option<PeriodicTdcRef>) -> Self {
            match begin_frame {
                Some(spim_tdc) => {
                    let ele_time = spimlib::correct_or_not_etime(pack.electron_time(), &spim_tdc);
                    SingleElectron {
                        data: (pack.electron_time(), pack.x(), pack.y(), ele_time-spim_tdc.begin_frame-VIDEO_TIME, spim_tdc.frame(), pack.tot(), 1),
                    }
                },
                None => {
                    SingleElectron {
                        data: (pack.electron_time(), pack.x(), pack.y(), 0, 0, pack.tot(), 1),
                    }
                },
            }
        }

        pub fn correct_time_overflow(&mut self, overflow: COUNTER) {
            self.data.0 += overflow as TIME * Pack::electron_overflow();
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
        pub fn relative_time(&self, reference_time: TIME) -> isize {
            self.data.0 as isize - reference_time as isize
        }
        pub fn relative_time_from_abs_tdc(&self, reference_time: TIME) -> i64 {
            (self.data.0*6) as i64 - reference_time as i64
        }
        pub fn spim_slice(&self) -> COUNTER {
            self.data.4
        }
        pub fn cluster_size(&self) -> usize {
            self.data.6
        }

        fn is_new_cluster(&self, s: &SingleElectron) -> bool {
            if self.time() > s.time() + CLUSTER_DET || (self.x() as isize - s.x() as isize).abs() > CLUSTER_SPATIAL || (self.y() as isize - s.y() as isize).abs() > CLUSTER_SPATIAL {
                true
            } else {
                false
            }
        }
        
        fn new_from_cluster_fixed_tot(cluster: &[SingleElectron], tot_threshold: u16) -> Option<SingleElectron> {

            let cluster_size = cluster.iter().
                count();
            
            let cluster_filter_size = cluster.iter().
                filter(|se| se.tot() > tot_threshold).
                count();

            if cluster_filter_size == 0 {return None};

            let t_mean:TIME = cluster.iter().
                filter(|se| se.tot() > tot_threshold).
                map(|se| se.time()).sum::<TIME>() / cluster_filter_size as TIME;
            
            let x_mean:POSITION = cluster.iter().
                map(|se| se.x()).
                sum::<POSITION>() / cluster_size as POSITION;
            
            let y_mean:POSITION = cluster.iter().
                map(|se| se.y()).
                sum::<POSITION>() / cluster_size as POSITION;
            
            let time_dif: TIME = cluster.iter().
                filter(|se| se.tot() > tot_threshold).
                map(|se| se.frame_dt()).
                next().
                unwrap();
            
            let slice: COUNTER = cluster.iter().
                filter(|se| se.tot() > tot_threshold).
                map(|se| se.spim_slice()).
                next().
                unwrap();
            
            let tot_sum: u16 = cluster.iter().
                filter(|se| se.tot() > tot_threshold).
                map(|se| se.tot() as usize).
                sum::<usize>() as u16;

            let cluster_size: usize = cluster_size;

            Some(SingleElectron {
                data: (t_mean, x_mean, y_mean, time_dif, slice, tot_sum, cluster_size),
            })
        }


        fn new_from_cluster(cluster: &[SingleElectron]) -> Option<SingleElectron> {
            let cluster_size = cluster.len();

            let t_mean:TIME = cluster.iter().map(|se| se.time()).sum::<TIME>() / cluster_size as TIME;
            let x_mean:POSITION = cluster.iter().map(|se| se.x()).sum::<POSITION>() / cluster_size as POSITION;
            let y_mean:POSITION = cluster.iter().map(|se| se.y()).sum::<POSITION>() / cluster_size as POSITION;
            let time_dif: TIME = cluster.iter().map(|se| se.frame_dt()).next().unwrap();
            let slice: COUNTER = cluster.iter().map(|se| se.spim_slice()).next().unwrap();
            let tot_sum: u16 = cluster.iter().map(|se| se.tot() as usize).sum::<usize>() as u16;
            let cluster_size: usize = cluster_size;

            Some(SingleElectron {
                data: (t_mean, x_mean, y_mean, time_dif, slice, tot_sum, cluster_size),
            })
        }

        pub fn get_or_not_spim_index(&self, spim_tdc: Option<PeriodicTdcRef>, xspim: POSITION, yspim: POSITION) -> Option<POSITION> {
            if let Some(frame_tdc) = spim_tdc {
                spimlib::get_spimindex(self.x(), self.frame_dt(), &frame_tdc, xspim, yspim)
            } else {
                None
            }
        }
    }
}

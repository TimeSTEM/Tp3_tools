pub mod cluster {
    
    use crate::packetlib::Packet;
    use crate::spimlib;
    use crate::tdclib::PeriodicTdcRef;
    use std::fs::OpenOptions;
    use std::io::Write;
    use rayon::prelude::*;
    
    const VIDEO_TIME: usize = 3_200; //Video time for spim (in 640 Mhz or 1.5625 ns).
    const CLUSTER_DET:usize = 128; //Cluster time window (in 640 Mhz or 1.5625).
    const CLUSTER_SPATIAL: isize = 2; // If electron hit position in both X or Y > CLUSTER_SPATIAL, then we have a new cluster.
    pub const SPIM_PIXELS: usize = 1024;

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

    /*
    impl IntoIterator for CollectionElectron {
        type Item = SingleElectron;
        type IntoIter = std::vec::IntoIter<Self::Item>;

        fn into_iter(self) -> Self::IntoIter {
            self.data.into_iter()
        }
    }
    
    impl<'a> IntoIterator for &'a CollectionElectron {
        type Item = &'a SingleElectron;
        type IntoIter = std::slice::Iter<'a, SingleElectron>;

        fn into_iter(self) -> Self::IntoIter {
            self.data.iter()
        }
    }
    */
    
    

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
                    if x.cluster_size() == 1 {
                        if x.is_new_cluster(&last) {
                            let new_from_cluster = SingleElectron::new_from_cluster(&cluster_vec);
                            nelist.push(new_from_cluster);
                            cluster_vec = Vec::new();
                        }
                        last = *x;
                        cluster_vec.push(*x);
                    } else {
                        nelist.push(*x);
                    }
            }
            self.data = nelist;
        }

        fn sort(&mut self) {
            self.data.par_sort_unstable_by(|a, b| (a.data).partial_cmp(&b.data).unwrap());
        }

        pub fn clean(&mut self) {
            self.sort();
            self.remove_clusters();
        }

        pub fn try_clean(&mut self, min_size: usize, remove: bool) -> bool {
            if self.data.len() > min_size && remove {
                let nelectrons = self.data.len();
                self.sort();
                for _x in 0..2 {
                    self.remove_clusters();
                }
                self.sort();
                let new_nelectrons = self.data.len();
                println!("Number of electrons: {}. Number of clusters: {}. Electrons per cluster: {}", nelectrons, new_nelectrons, nelectrons as f32/new_nelectrons as f32); 
                return true
            }
            !remove
        }

        pub fn output_data(&self, filename: String, slice: usize) {
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
    }

    ///ToA, X, Y, Spim dT, Spim Slice, ToT, Cluster Size
    #[derive(Copy, Clone, Debug)]
    pub struct SingleElectron {
        data: (usize, usize, usize, usize, usize, u16, usize),
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
        pub fn new<T: Packet>(pack: &T, begin_frame: Option<PeriodicTdcRef>, slice: usize) -> Self {
            let ele_time = pack.electron_time();
            match begin_frame {
                Some(spim_tdc) => {
                    let mut frame_time = spim_tdc.begin_frame;
                    if ele_time < frame_time + VIDEO_TIME {
                        frame_time -= spim_tdc.period*spim_tdc.ticks_to_frame.unwrap();
                    }
                    SingleElectron {
                        data: (ele_time, pack.x(), pack.y(), ele_time-frame_time-VIDEO_TIME, slice, pack.tot(), 1),
                    }
                },
                None => {
                    SingleElectron {
                        data: (ele_time, pack.x(), pack.y(), 0, slice, pack.tot(), 1),
                    }
                },
            }
        }

        pub fn x(&self) -> usize {
            self.data.1
        }
        pub fn y(&self) -> usize {
            self.data.2
        }
        pub fn time(&self) -> usize {
            self.data.0
        }
        pub fn tot(&self) -> u16 {
            self.data.5
        }
        pub fn frame_dt(&self) -> usize {
            self.data.3
        }
        pub fn image_index(&self) -> usize {
            self.data.1 + SPIM_PIXELS*self.data.2
        }
        pub fn relative_time(&self, reference_time: usize) -> isize {
            self.data.0 as isize - reference_time as isize
        }
        pub fn spim_slice(&self) -> usize {
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

        fn new_from_cluster(cluster: &[SingleElectron]) -> SingleElectron {
            let cluster_size: usize = cluster.len();
            
            let t_mean:usize = cluster.iter().map(|se| se.time()).sum::<usize>() / cluster_size;
            let x_mean:usize = cluster.iter().map(|se| se.x()).sum::<usize>() / cluster_size;
            let y_mean:usize = cluster.iter().map(|se| se.y()).sum::<usize>() / cluster_size;
            let time_dif: usize = cluster.iter().map(|se| se.frame_dt()).next().unwrap();
            let slice: usize = cluster.iter().map(|se| se.spim_slice()).next().unwrap();
            let tot_sum: u16 = cluster.iter().map(|se| se.tot() as usize).sum::<usize>() as u16;
            let cluster_size: usize = cluster_size;

            SingleElectron {
                data: (t_mean, x_mean, y_mean, time_dif, slice, tot_sum, cluster_size),
            }
        }

        pub fn get_or_not_spim_index(&self, spim_tdc: Option<PeriodicTdcRef>, xspim: usize, yspim: usize) -> Option<usize> {
            
            if let Some(frame_tdc) = spim_tdc {
                let index = spimlib::get_spimindex(0, self.frame_dt(), &frame_tdc, xspim, yspim);
                index
                
                /*
                let interval = frame_tdc.low_time;
                let period = frame_tdc.period;

                let val = self.frame_dt() % period;
                if val >= interval {return None;}
                let mut r = self.frame_dt() / period;
                let rin = val * xspim / interval;

                if r > yspim -1 {
                    if r > 4096 {return None;}
                    r %= yspim;
                }

                let result = r * xspim + rin;
                Some(result)
            */
            } else {
                None
            }
        }

    }
}

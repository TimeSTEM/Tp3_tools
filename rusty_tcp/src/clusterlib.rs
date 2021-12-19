pub mod cluster {
    
    use crate::packetlib::{Packet, PacketEELS as Pack};
    use crate::tdclib::PeriodicTdcRef;
    use std::fs::OpenOptions;
    use std::io::Write;
    use rayon::prelude::*;
    
    const VIDEO_TIME: usize = 5000; //Video time for spim (ns).
    const CLUSTER_DET:usize = 200; //Cluster time window (ns).
    const CLUSTER_SPATIAL: isize = 2; // If electron hit position in both X or Y > CLUSTER_SPATIAL, then we have a new cluster.
    pub const SPIM_PIXELS: usize = 1024;

    pub struct CollectionElectron {
        data: Vec<SingleElectron>,
    }

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
            let nelectrons = self.data.len();
            let mut nelist: Vec<SingleElectron> = Vec::new();
            let mut cs_list: Vec<usize> = Vec::new();

            let mut last: SingleElectron = self.data[0];
            let mut cluster_vec: Vec<SingleElectron> = Vec::new();
            for x in &self.data {
                if x.is_new_cluster(&last) {
                    let cluster_size: usize = cluster_vec.len();
                    let new_from_cluster = SingleElectron::new_from_cluster(&cluster_vec);
                    nelist.push(new_from_cluster);
                    cs_list.push(cluster_size);
                    cluster_vec = Vec::new();
                }
                last = *x;
                cluster_vec.push(*x);
            }
            self.data = nelist;
            let new_nelectrons = self.data.len();
            println!("Number of electrons: {}. Number of clusters: {}. Electrons per cluster: {}", nelectrons, new_nelectrons, nelectrons as f32/new_nelectrons as f32); 
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
                self.sort();
                self.remove_clusters();
                return true
            }
            !remove
        }

        pub fn output_time(&self, mut filename: String, code: usize) {
            filename.push_str(&code.to_string());
            let mut tfile = OpenOptions::new()
                .append(false)
                .write(true)
                .truncate(true)
                .create(true)
                .open(&filename).expect("Could not output time histogram.");
            let out: String = self.data.iter().map(|x| x.time().to_string()).collect::<Vec<String>>().join(", ");
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
            let out: String = self.data.iter().map(|x| x.x().to_string()).collect::<Vec<String>>().join(", ");
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
            let out: String = self.data.iter().map(|x| x.y().to_string()).collect::<Vec<String>>().join(", ");
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
            let out: String = self.data.iter().map(|x| x.tot().to_string()).collect::<Vec<String>>().join(", ");
            tfile.write_all(out.as_ref()).expect("Could not write time to file.");
        }
    }

    ///ToA, X, Y, Spim dT, Spim Slice, ToT
    #[derive(Copy, Clone, Debug)]
    pub struct SingleElectron {
        data: (usize, usize, usize, usize, usize, u16),
    }


    impl SingleElectron {
        pub fn new(pack: &Pack, begin_frame: Option<usize>, slice: usize) -> Self {
            let ele_time = pack.electron_time();
            match begin_frame {
                Some(frame_time) => {
                    SingleElectron {
                        data: (ele_time, pack.x(), pack.y(), ele_time-frame_time-VIDEO_TIME, slice, pack.tot()),
                    }
                },
                None => {
                    SingleElectron {
                        data: (ele_time, pack.x(), pack.y(), 0, slice, pack.tot()),
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

        fn is_new_cluster(&self, s: &SingleElectron) -> bool {
            if self.data.0 > s.data.0 + CLUSTER_DET || (self.data.1 as isize - s.data.1 as isize).abs() > CLUSTER_SPATIAL || (self.data.2 as isize - s.data.2 as isize).abs() > CLUSTER_SPATIAL {
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
            //let tot_mean: u16 = (cluster.iter().map(|se| se.tot() as usize).sum::<usize>() / cluster_size) as u16;
            let tot_mean: u16 = cluster.iter().map(|se| se.tot() as usize).sum::<usize>() as u16;
            
            let time_dif: usize = cluster.iter().map(|se| se.frame_dt()).next().unwrap();
            let slice: usize = cluster.iter().map(|se| se.spim_slice()).next().unwrap();

            SingleElectron {
                data: (t_mean, x_mean, y_mean, time_dif, slice, tot_mean),
            }
        }

        pub fn get_or_not_spim_index(&self, spim_tdc: Option<PeriodicTdcRef>, xspim: usize, yspim: usize) -> Option<usize> {
            
            if let Some(frame_tdc) = spim_tdc {
                let interval = frame_tdc.low_time;
                let period = frame_tdc.period;

                let val = self.data.3 % period;
                if val >= interval {return None;}
                let mut r = self.data.3 / period;
                let rin = val * xspim / interval;

                if r > yspim -1 {
                    if r > Pack::electron_reset_time() {return None;}
                    r %= yspim;
                }

                let result = r * xspim + rin;
                Some(result)
            } else {
                None
            }
        }

    }
}

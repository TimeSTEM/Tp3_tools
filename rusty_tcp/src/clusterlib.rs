pub mod cluster {
    
    use crate::packetlib::{Packet, PacketEELS as Pack};
    use crate::tdclib::PeriodicTdcRef;
    use rayon::prelude::*;
    
    const VIDEO_TIME: usize = 5000; //Video time for spim (ns).
    const CLUSTER_DET:usize = 50; //Cluster time window (ns).
    const SPIM_PIXELS: usize = 1024;

    pub struct CollectionElectron {
        pub data: Vec<SingleElectron>,
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
        pub fn remove_clusters(&mut self) {
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

        pub fn try_clean_and_append(&mut self, array: &mut Vec<Vec<usize>>, frame_tdc: Option<PeriodicTdcRef>, xspim: usize, yspim: usize) {
            if self.data.len() > 0 {
                self.sort();
                self.remove_clusters();
                for val in &self.data {
                    if let Some(index) = val.get_or_not_spim_index(frame_tdc, xspim, yspim) {
                        val.append_sliced_array(array, index);
                    }
                }
                self.data = Vec::new();
            }
        }
    }

    ///ToA, X, Y, Spim dT, Spim Slice
    #[derive(Copy, Clone, Debug)]
    pub struct SingleElectron {
        pub data: (usize, usize, usize, usize, usize),
    }


    impl SingleElectron {
        pub fn new(pack: &Pack, begin_frame: Option<usize>, slice: usize) -> Self {
            let ele_time = pack.electron_time();
            match begin_frame {
                Some(frame_time) => {
                    SingleElectron {
                        data: (ele_time, pack.x(), pack.y(), ele_time-frame_time-VIDEO_TIME, slice),
                    }
                },
                None => {
                    SingleElectron {
                        data: (ele_time, pack.x(), pack.y(), 0, 0),
                    }
                },
            }
        }

        fn is_new_cluster(&self, s: &SingleElectron) -> bool {
            if self.data.0 > s.data.0 + CLUSTER_DET || (self.data.1 as isize - s.data.1 as isize).abs() > 2 || (self.data.2 as isize - s.data.2 as isize).abs() > 2 {
                true
            } else {
                false
            }
        }


        fn new_from_cluster(cluster: &[SingleElectron]) -> SingleElectron {
            let cluster_size: usize = cluster.len();
            
            let t_mean:usize = cluster.iter().map(|se| se.data.0).sum::<usize>() / cluster_size as usize;
            //let t_mean:usize = cluster.iter().map(|se| se.data.0).next().unwrap();
            
            let x_mean:usize = cluster.iter().map(|se| se.data.1).sum::<usize>() / cluster_size;
            //let x_mean:usize = cluster.iter().map(|se| se.data.1).next().unwrap();
            
            let y_mean:usize = cluster.iter().map(|se| se.data.2).sum::<usize>() / cluster_size;
            //let y_mean:usize = cluster.iter().map(|se| se.data.2).next().unwrap();
            
            //let tot_mean: u16 = (cluster_vec.iter().map(|&(_, _, _, tot, _)| tot as usize).sum::<usize>() / cluster_size) as u16;
            
            let time_dif: usize = cluster.iter().map(|se| se.data.3).next().unwrap();
            let slice: usize = cluster.iter().map(|se| se.data.4).next().unwrap();
            
            SingleElectron {
                data: (t_mean, x_mean, y_mean, time_dif, slice),
            }
        }

        fn get_or_not_spim_index(&self, spim_tdc: Option<PeriodicTdcRef>, xspim: usize, yspim: usize) -> Option<usize> {
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
                Some(0)
            }
        }

        fn append_sliced_array(&self, array: &mut Vec<Vec<usize>>, index: usize) {
            array[self.data.4][index*SPIM_PIXELS+self.data.1] += 1;
            
        }
    }
}

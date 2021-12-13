pub mod cluster {
    
    use crate::packetlib::{Packet, PacketEELS as Pack};
    use rayon::prelude::*;
    
    const VIDEO_TIME: usize = 5000; //Video time for spim (ns).
    const CLUSTER_DET:usize = 50; //Cluster time window (ns).

    pub struct CollectionElectron {
        pub data: Vec<SingleElectron>,
    }

    impl CollectionElectron {
        pub fn remove_clusters(&mut self) {
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
        }

        pub fn sort(&mut self) {
            self.data.par_sort_unstable_by(|a, b| (a.data).partial_cmp(&b.data).unwrap());
        }
    }

    ///ToA, X, Y, Spim dT
    #[derive(Copy, Clone, Debug)]
    pub struct SingleElectron {
        pub data: (usize, usize, usize, usize),
    }


    impl SingleElectron {
        pub fn try_new(pack: &Pack, begin_frame: Option<usize>) -> Option<Self> {
            let ele_time = pack.electron_time();
            match begin_frame {
                Some(frame_time) => {
                    Some(SingleElectron {
                        data: (ele_time, pack.x(), pack.y(), ele_time-frame_time-VIDEO_TIME),
                    })
                },
                None => {
                    Some(SingleElectron {
                        data: (ele_time, pack.x(), pack.y(), 0),
                    })
                },
            }
        }
        
        pub fn is_new_cluster(&self, s: &SingleElectron) -> bool {
            if self.data.0 > s.data.0 + CLUSTER_DET || (self.data.1 as isize - s.data.1 as isize).abs() > 2 || (self.data.2 as isize - s.data.2 as isize).abs() > 2 {
                true
            } else {
                false
            }
        }


        pub fn new_from_cluster(cluster: &[SingleElectron]) -> SingleElectron {
            let cluster_size: usize = cluster.len();
            
            let t_mean:usize = cluster.iter().map(|se| se.data.0).sum::<usize>() / cluster_size as usize;
            //let t_mean:usize = cluster.iter().map(|se| se.data.0).next().unwrap();
            
            let x_mean:usize = cluster.iter().map(|se| se.data.1).sum::<usize>() / cluster_size;
            //let x_mean:usize = cluster.iter().map(|se| se.data.1).next().unwrap();
            
            let y_mean:usize = cluster.iter().map(|se| se.data.2).sum::<usize>() / cluster_size;
            //let y_mean:usize = cluster.iter().map(|se| se.data.2).next().unwrap();
            
            //let tot_mean: u16 = (cluster_vec.iter().map(|&(_, _, _, tot, _)| tot as usize).sum::<usize>() / cluster_size) as u16;
            
            let time_dif: usize = cluster.iter().map(|se| se.data.3).next().unwrap();
            
            SingleElectron {
                data: (t_mean, x_mean, y_mean, time_dif),
            }
        }
    }
}


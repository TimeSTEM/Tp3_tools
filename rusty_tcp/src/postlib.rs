pub mod coincidence {

    use crate::packetlib::{Packet, PacketEELS as Pack};
    use crate::tdclib::TdcType;
    use std::io;
    use std::io::prelude::*;
    use std::fs;
    //use std::time::Instant;

    const TIME_WIDTH: f64 = 50.0e-9;
    const TIME_DELAY: f64 = 165.0e-9;
    const MIN_LEN: usize = 100; // This is the minimal TDC vec size. It reduces over time.
    const EXC: (usize, usize) = (20, 5); //This controls how TDC vec reduces. (20, 5) means if correlation is got in the time index >20, the first 5 items are erased.
    const CLUSTER_DET:f64 = 50.0e-09;

    pub struct ElectronData {
        pub time: Vec<f64>,
        pub rel_time: Vec<f64>,
        pub x: Vec<usize>,
        pub y: Vec<usize>,
        pub tot: Vec<usize>,
        pub cluster_size: Vec<usize>,
        pub spectrum: Vec<usize>,
        pub corr_spectrum: Vec<usize>,
    }

    impl ElectronData {
        fn add_electron(&mut self, index: usize) {
            self.spectrum[index] += 1;
        }

        fn add_coincident_electron(&mut self, index: usize) {
            self.corr_spectrum[index] += 1;
        }


        pub fn new() -> Self {
            Self {
                time: Vec::new(),
                rel_time: Vec::new(),
                x: Vec::new(),
                y: Vec::new(),
                tot: Vec::new(),
                cluster_size: Vec::new(),
                spectrum: vec![0; 1024],
                corr_spectrum: vec![0; 1024],
            }
        }
    }

    pub struct TempElectronData {
        pub tdc: Vec<f64>,
        pub electron: Vec<(f64, usize, usize, u16)>,
    }

    impl TempElectronData {
        fn new() -> Self {
            Self {
                tdc: Vec::new(),
                electron: Vec::new(),
            }
        }

        fn add_tdc(&mut self, my_pack: &Pack) {
            self.tdc.push(my_pack.tdc_time_norm() - TIME_DELAY);
        }

        fn add_electron(&mut self, my_pack: &Pack) {
            self.electron.push((my_pack.electron_time(), my_pack.x().unwrap(), my_pack.y().unwrap(), my_pack.tot()));
        }

        fn sort_all(&mut self) {
            self.tdc.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
            self.electron.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        }
    }
            

    //pub fn search_coincidence(file: &str, ele_vec: &mut [usize], cele_vec: &mut [usize], clusterlist: &mut Vec<(f64, f64, usize, usize, u16, usize)>) -> io::Result<usize> {
    pub fn search_coincidence(file: &str, coinc_data: &mut ElectronData) -> io::Result<usize> {
        
        let mut file = fs::File::open(file)?;
        let mut buffer:Vec<u8> = Vec::new();
        file.read_to_end(&mut buffer)?;
        
        let mytdc = TdcType::TdcTwoRisingEdge;
        let mut ci = 0;
        let mut tdc_vec:Vec<f64> = Vec::new();
        let mut elist:Vec<(f64, usize, usize, u16)> = Vec::new();

        let mut temp_edata = TempElectronData::new();
        
        let mut packet_chunks = buffer.chunks_exact(8);
        while let Some(pack_oct) = packet_chunks.next() {
            match pack_oct {
                &[84, 80, 88, 51, nci, _, _, _] => {ci=nci as usize;},
                _ => {
                    let packet = Pack { chip_index: ci, data: pack_oct };
                    match packet.id() {
                        6 if packet.tdc_type() == mytdc.associate_value() => {
                            //tdc_vec.push(packet.tdc_time_norm()-TIME_DELAY);
                            temp_edata.add_tdc(&packet);
                        },
                        11 => {
                            if let (Some(x), Some(y)) = (packet.x(), packet.y()) {
                                temp_edata.add_electron(&packet);
                                //elist.push((packet.electron_time(), x, y, packet.tot()));
                            }
                        },
                        _ => {},
                    };
                },
            };
        }

        temp_edata.sort_all();
        //tdc_vec.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let nphotons:usize = temp_edata.tdc.len();

        //elist.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
        let eclusters = cluster_centroid(&elist);
        println!("Electron events: {}. Number of clusters: {}. Ratio: {}", elist.len(), eclusters.len(), elist.len() as f32 / eclusters.len() as f32);

        let mut counter = 0;
        for val in &eclusters {
            //ele_vec[val.1]+=1;
            coinc_data.add_electron(val.1);
            let veclen = tdc_vec.len().min(2*MIN_LEN);
            if let Some((index, pht)) = testfunc(&tdc_vec[0..veclen], val.0) {
                counter+=1;
                //cele_vec[val.1+1024*val.2]+=1;
                coinc_data.add_coincident_electron(val.1);

                clusterlist.push((val.0, val.0 - pht, val.1, val.2, val.3, val.4));
                if index>EXC.0 && tdc_vec.len()>index+MIN_LEN{
                    tdc_vec = tdc_vec.into_iter().skip(index-EXC.1).collect();
                }
            }
        }
        println!("The number of correlated clusters is: {}. Number of detected TDC's is: {}.", counter, nphotons);

        Ok(nphotons)
    }

    fn testfunc(tdcrefvec: &[f64], value: f64) -> Option<(usize, f64)> {
        tdcrefvec.iter().cloned().enumerate().filter(|(_, x)| (x-value).abs()<TIME_WIDTH).next()
    }

    fn cluster_centroid(electron_list: &[(f64, usize, usize, u16)]) -> Vec<(f64, usize, usize, u16, usize)> {
        let mut nelist:Vec<(f64, usize, usize, u16, usize)> = Vec::new();
        let mut last: (f64, usize, usize, u16) = electron_list[0];
        let mut cluster_vec: Vec<(f64, usize, usize, u16)> = Vec::new();
        for x in electron_list {
            if x.0 > last.0 + CLUSTER_DET || (x.1 as isize - last.1 as isize).abs() > 2 || (x.2 as isize - last.2 as isize).abs() > 2 {
                let cluster_size: usize = cluster_vec.len();
                let t_mean:f64 = cluster_vec.iter().map(|&(t, _, _, _)| t).sum::<f64>() / cluster_size as f64;
                let x_mean:usize = cluster_vec.iter().map(|&(_, x, _, _)| x).sum::<usize>() / cluster_size;
                let y_mean:usize = cluster_vec.iter().map(|&(_, _, y, _)| y).sum::<usize>() / cluster_size;
                let tot_mean: u16 = (cluster_vec.iter().map(|&(_, _, _, tot)| tot as usize).sum::<usize>() / cluster_size) as u16;
                //println!("{:?} and {}", cluster_vec, t_mean);
                nelist.push((t_mean, x_mean, y_mean, tot_mean, cluster_size));
                cluster_vec = Vec::new();
            }
            last = *x;
            cluster_vec.push(*x);
        }
        nelist
    }
}

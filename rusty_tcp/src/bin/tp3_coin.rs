//use timepix3::auxiliar::{RunningMode, BytesConfig, Settings};
//use timepix3::tdclib::{TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
//use timepix3::{modes, misc};

//use plotters::prelude::*;
use timepix3::packetlib::Packet;
use timepix3::tdclib::TdcType;
use std::io;
use std::io::prelude::*;
use std::fs;
use std::time::Instant;

const TIME_WIDTH: f64 = 1000.0e-9;
const TIME_DELAY: f64 = 0.0e-9;
const MIN_LEN: usize = 100; // This is the minimal TDC vec size. It reduces over time.
const EXC: (usize, usize) = (20, 5); //This controls how TDC vec reduces. (20, 5) means if correlation is got in the time index >20, the first 5 items are erased.
const CLUSTER_DET:f64 = 50.0e-09;

fn search_coincidence(file: &str, ele_vec: &mut [usize], timelist: &mut Vec<(f64, f64, usize, usize, u16)>) -> io::Result<()> {
    
    let mut file = fs::File::open(file)?;
    let mut buffer:Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    let mytdc = TdcType::TdcTwoRisingEdge;
    let mut ci = 0;
    let mut tdc_vec:Vec<f64> = Vec::new();
    
    let mut packet_chunks = buffer.chunks_exact(8);
    while let Some(pack_oct) = packet_chunks.next() {
        match pack_oct {
            &[84, 80, 88, 51, nci, _, _, _] => {ci=nci;},
            _ => {
                let packet = Packet { chip_index: ci, data: pack_oct };
                match packet.id() {
                    6 if packet.tdc_type() == mytdc.associate_value() => {
                        tdc_vec.push(packet.tdc_time_norm()-TIME_DELAY);
                    },
                    _ => {},
                };
            },
        };
    }
    

    let mut packet_chunks = buffer.chunks_exact(8);
    while let Some(x) = packet_chunks.next() {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => {ci=nci;},
            _ => {
                let packet = Packet { chip_index: ci, data: x };
                match packet.id() {
                    11 => {
                        if let Some(x) = packet.x() {
                            ele_vec[x]+=1;
                            let ele_time = packet.electron_time();
                            let veclen = tdc_vec.len().min(2*MIN_LEN);
                            if let Some((index, pht)) = testfunc(&tdc_vec[0..veclen], ele_time) {
                                let y = packet.y();
                                timelist.push((ele_time, ele_time - pht, x, y, packet.tot()));
                                if index>EXC.0 && tdc_vec.len()>index+MIN_LEN{
                                    tdc_vec = tdc_vec.into_iter().skip(index-EXC.1).collect();
                                }
                            }
                        }
                    },
                    _ => {},
                };
            },
        };
    }

    Ok(())
}

fn testfunc(tdcrefvec: &[f64], value: f64) -> Option<(usize, f64)> {
    tdcrefvec.iter().cloned().enumerate().filter(|(_, x)| (x-value).abs()<TIME_WIDTH).next()
}

fn find_avgt(data: &[(f64, f64, usize, usize, u16)]) -> Option<f64> {
    let size=data.len();
    if size==0 {return None}
    let t: f64 = data.iter().map(|&(_, tph, _, _, _)| tph).sum();
    Some(t / size as f64)
}

fn find_centroid(data: &[(f64, f64, usize, usize, u16)]) -> Option<(usize, usize, usize)> {
    let size = data.len();
    if size == 0 {return None}
    let x: usize = data.iter().map(|&(_, _, x, _, _)| x).sum();
    let y: usize = data.iter().map(|&(_, _, _, y, _)| y).sum();
    let tot: usize = data.iter().map(|&(_, _, _, _, tot)| tot as usize).sum();

    Some((x / size, y / size, tot / size))
}

fn cs_tot(data: &[(f64, f64, usize, usize, u16)]) -> Option<(usize, usize)> {
    let size = data.len();
    if size == 0 {return None}
    let totsum: usize = data.iter().map(|&(_, _, _, _, tot)| tot as usize).sum();

    Some((size, totsum))
}

fn mean(data: &[usize]) -> Option<f32> {
    let sum = data.iter().sum::<usize>() as f32;
    let size = data.len();
    
    match size {
        size if size > 0 => Some(sum / size as f32),
        _ => None
    }
}

fn std_dev(data: &[usize]) -> Option<f32> {
    match (mean(data), data.len()) {
        (Some(data_mean), size) if size > 0 => {
            let variance = data.iter().map(|value| {
                let diff = data_mean - (*value as f32);
                diff*diff
            }).sum::<f32>() / size as f32;
            Some(variance.sqrt())
        },
        _ => None
    }
}



fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let mut ele_vec:Vec<usize> = vec![0; 1024];
    let mut cele_vec:Vec<usize> = vec![0; 1024];
    let mut time_list:Vec<(f64, f64, usize, usize, u16)> = Vec::new();
    let mut xarray:Vec<usize> = vec![0; 1024];
            
    for (i, val) in xarray.iter_mut().enumerate() {
        *val = i;
    }


    let mut entries = fs::read_dir("Data")?;
    while let Some(x) = entries.next() {
        let path = x?.path();
        let dir = path.to_str().unwrap();
        println!("Looping over file {:?}", dir);
        search_coincidence(dir, &mut ele_vec, &mut time_list)?;
    }

    println!("Number of events in time_list is: {}", time_list.len());
    let start = Instant::now();
    time_list.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    println!("Time elapsed during sorting is {:?}", start.elapsed());

    let mut cluster_vec: Vec<(f64, f64, usize, usize, u16)> = Vec::new();
    let mut sizetot: Vec<(usize, usize)> = Vec::new();
    let mut newtl: Vec<(f64)> = Vec::new();

    let mut last: (f64, f64, usize, usize, u16) = time_list[0];
    let start = Instant::now();
    for x in &time_list {
        if x.0 > last.0 + CLUSTER_DET || (x.2 as isize - last.2 as isize).abs() > 2 || (x.3 as isize - last.3 as isize).abs() > 2 {
            if let Some(val) = cs_tot(&cluster_vec) {sizetot.push(val);}
            if let Some(val) = find_centroid(&cluster_vec) {cele_vec[val.0]+=1}
            if let Some(val) = find_avgt(&cluster_vec) {newtl.push(val);}
            cluster_vec = Vec::new();
        }
        last = *x;
        cluster_vec.push(*x);
    }
    println!("Time elapsed during looping over is {:?}", start.elapsed());
    println!("Number of clusters is: {}", sizetot.len());


    
    let output_vec: Vec<String> = cele_vec.iter().map(|x| x.to_string()).collect();
    let output_string = output_vec.join(", ");
    fs::write("xH.txt", output_string)?;
    
    let output_vec: Vec<String> = newtl.iter().map(|t| t.to_string()).collect();
    let output_string = output_vec.join(", ");
    fs::write("tH.txt", output_string)?;

    let output_vec: Vec<String> = sizetot.iter().map(|(cs, _)| cs.to_string()).collect();
    let output_string = output_vec.join(", ");
    fs::write("cs.txt", output_string)?;
    
    let output_vec: Vec<String> = sizetot.iter().map(|(_, stot)| stot.to_string()).collect();
    let output_string = output_vec.join(", ");
    fs::write("stot.txt", output_string)?;

    /*
    let max = ele_vec.iter().fold(0, |acc, &x|
                                       if acc>x {acc} else {x}
                                       );
    
    let cmax = cele_vec.iter().fold(0, |acc, &x|
                                       if acc>x {acc} else {x}
                                       );

    let vecsum:usize = cele_vec.iter().sum();

    println!("Maximum value is: {} and sum is: {}", cmax, vecsum);

    let root = BitMapBackend::new("out.png", (2000, 1200)).into_drawing_area();
    root.fill(&WHITE)?;
    //let root = root.margin(10, 10, 10, 10);
    let mut chart = ChartBuilder::on(&root)
        .caption("TP3 Coincidence", ("sans_serif", 32).into_font())
        .x_label_area_size(40)
        .y_label_area_size(40)
        .right_y_label_area_size(40)
        .build_cartesian_2d(0f32..1024f32, 0f32..max as f32)?
        .set_secondary_coord(0f32..1024f32, 0f32..cmax as f32); 

    chart
        .configure_mesh()
        .disable_x_mesh()
        .disable_y_mesh()
        .y_desc("EELS")
        .y_label_formatter(&|x| format!("{:e}", x))
        .draw()?;

    chart
        .configure_secondary_axes()
        .y_desc("Coinc. EELS")
        .draw()?;

    chart
        .draw_series(LineSeries::new(
            xarray.iter().zip(ele_vec.iter()).map(|(a, b)| (*a as f32, *b as f32)),
            &RED,
        ))?
        .label("EELS")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x +20, y)], &RED));
    
    chart
        .draw_secondary_series(LineSeries::new(
            xarray.iter().zip(cele_vec.iter()).map(|(a, b)| (*a as f32, *b as f32)),
            &BLUE,
        ))?
        .label("cEELS")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x +20, y)], &BLUE));

    chart
        .configure_series_labels()
        .background_style(&WHITE.mix(0.8))
        .border_style(&BLACK)
        .draw()?;

    */
    Ok(())
}



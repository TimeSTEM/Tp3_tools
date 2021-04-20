//use timepix3::auxiliar::{RunningMode, BytesConfig, Settings};
//use timepix3::tdclib::{TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
//use timepix3::{modes, misc};

use plotters::prelude::*;
use timepix3::packetlib::Packet;
use timepix3::tdclib::TdcType;
use std::io;
use std::io::prelude::*;
use std::fs;
use std::time::Instant;

fn search_coincidence(file: &str, ele_vec: &mut [usize], cele_vec: &mut [usize]) -> io::Result<()> {
    
    let mut file = fs::File::open(file)?;
    let mut buffer:Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    let mytdc = TdcType::TdcTwoRisingEdge;
    let mut ci = 0;
    let mut tdc_vec:Vec<f64> = Vec::new();
    
    let mut packet_chunks = buffer.chunks_exact(8);
    while let Some(x) = packet_chunks.next() {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => {ci=nci;},
            _ => {
                let packet = Packet { chip_index: ci, data: x };
                match packet.id() {
                    6 if packet.tdc_type() == mytdc.associate_value() => {
                        tdc_vec.push(packet.tdc_time_norm());
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
                        ele_vec[packet.x()]+=1;
                        if let Some(()) = testfunc(&mut tdc_vec, packet.electron_time()) {
                            cele_vec[packet.x()]+=1;
                        }
                    },
                    _ => {},
                };
            },
        };
    }

    Ok(())
}

fn testfunc(tdcrefvec: &mut Vec<f64>, value: f64) -> Option<()> {
    let mut n = tdcrefvec.into_iter().enumerate().filter(|(_, x)| (**x-value).abs()<25.0e-9);
    let val = n.next();
    if val.is_some() {
        let val = val.unwrap(); let index = val.0; if index>5 {tdcrefvec.remove(0);}
        Some(())
    } else {None}
}


fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let mut ele_vec:Vec<usize> = vec![0; 1024];
    let mut cele_vec:Vec<usize> = vec![0; 1024];
    let mut xarray:Vec<usize> = vec![0; 1024];
            
    for (i, val) in xarray.iter_mut().enumerate() {
        *val = i;
    }

    let start = Instant::now();

    let mut entries = fs::read_dir("Data")?;
    while let Some(x) = entries.next() {
        let path = x?.path();
        let dir = path.to_str().unwrap();
        println!("Looping over file {:?}", dir);
        search_coincidence(dir, &mut ele_vec, &mut cele_vec)?;
    }



    println!("Time elapsed is {:?}", start.elapsed());

    let max = ele_vec.iter().fold(0, |acc, &x|
                                       if acc>x {acc} else {x}
                                       );
    
    let cmax = cele_vec.iter().fold(0, |acc, &x|
                                       if acc>x {acc} else {x}
                                       );

    println!("{}", cmax);

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
        //.y_label_formatter(&|x| format!("{:.5}", x))
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

    Ok(())
}



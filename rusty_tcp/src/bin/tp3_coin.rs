//use timepix3::auxiliar::{RunningMode, BytesConfig, Settings};
//use timepix3::tdclib::{TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
//use timepix3::{modes, misc};

use plotters::prelude::*;
use timepix3::packetlib::Packet;
use timepix3::tdclib::TdcType;
use std::io::prelude::*;
use std::fs::File;


fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = File::open("C:\\Users\\AUAD\\Documents\\Tp3_tools\\TCPFiletoStream\\laser_tdc\\raw000000.tpx3")?;
    let mut buffer:Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer)?;
    
    let mytdc = TdcType::TdcTwoRisingEdge;
    let mut ci = 0;
    let mut tdc_vec:Vec<f64> = Vec::new();
    let mut ele_vec:Vec<usize> = vec![0; 1024];
    let mut cele_vec:Vec<usize> = vec![0; 1024];
    let mut xarray:Vec<usize> = vec![0; 1024];
            
    for (i, val) in xarray.iter_mut().enumerate() {
        *val = i;
    }

    let mut packet_chunks = buffer.chunks_exact(8);
    while let Some(x) = packet_chunks.next() {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => {ci=nci;},
            _ => {
                let packet = Packet { chip_index: ci, data: x };
                match packet.id() {
                    6 if packet.tdc_type() == mytdc.associate_value() => {
                        tdc_vec.push(packet.tdc_time());
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
                        if let Some(()) = testfunc(&tdc_vec, packet.electron_time()) {
                            cele_vec[packet.x()]+=1;
                        }
                    },
                    _ => {},
                };
            },
        };
    }

    println!("{:?} and {} and {}", tdc_vec.len(), ele_vec.len(), cele_vec.len());
    println!("{:?}", cele_vec);



    let root = BitMapBackend::new("out.png", (640, 480)).into_drawing_area();
    root.fill(&WHITE)?;
    let root = root.margin(10, 10, 10, 10);
    let mut chart = ChartBuilder::on(&root)
        .caption("TP3 Coincidence", ("sans_serif", 40).into_font())
        .x_label_area_size(20)
        .y_label_area_size(40)
        .build_cartesian_2d(0f32..1024f32, 0f32..10f32)?;

    chart
        .configure_mesh()
        .x_labels(5)
        .y_labels(5)
        .y_label_formatter(&|x| format!("{:.3}", x))
        .draw()?;

    chart.draw_series(LineSeries::new(
            (-50..=50).map(|x| x as f32/50.0).map(|x| (x, x * x)),
            //cele_vec.iter().zip(xarray.iter(),
            &RED,
    ))?;

    Ok(())
}


fn testfunc(tdcrefvec: &[f64], value: f64) -> Option<()> {
    let n = tdcrefvec.into_iter().filter(|x| (**x-value).abs()<1.0e-7).count();
    if n>0 {Some(())} else {None}
}

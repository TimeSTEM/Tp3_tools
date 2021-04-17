//use timepix3::auxiliar::{RunningMode, BytesConfig, Settings};
//use timepix3::tdclib::{TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
//use timepix3::{modes, misc};


use timepix3::packetlib::Packet;
use timepix3::tdclib::TdcType;
use std::io;
use std::io::prelude::*;
use std::fs::File;


fn main() -> io::Result<()> {
    let mut file = File::open("C:\\Users\\AUAD\\Documents\\Tp3_tools\\TCPFiletoStream\\gain_data\\raw000000.tpx3")?;
    let mut buffer:Vec<u8> = Vec::new();
    file.read_to_end(&mut buffer);
    
    let mut ci = 0;
    let mut counter = 0;
    let mytdc = TdcType::TdcTwoRisingEdge;

    let mut packet_chunks = buffer.chunks_exact(8);
    while let Some(x) = packet_chunks.next() {
        match x {
            &[84, 80, 88, 51, nci, _, _, _] => {ci=nci; counter+=1;},
            _ => {
                let packet = Packet { chip_index: ci, data: x };
                match packet.id() {
                    6 if packet.tdc_type() == mytdc.associate_value() => {
                    
                    },
                    _ => {},
                };


            },
        };
        
    }

    println!("{}", buffer.len());

    Ok(())
}

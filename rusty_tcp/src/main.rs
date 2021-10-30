use timepix3::errorlib::Tp3ErrorKind;
use timepix3::auxiliar::Settings;
use timepix3::tdclib::{TdcControl, TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
use timepix3::speclib;
use timepix3::spimlib; use timepix3::spimlib::SpimKind;
use timepix3::chronolib;
use chrono::prelude::*;
use std::{fs::{OpenOptions, create_dir_all}, path::Path};
use std::io::Write;


fn connect_and_loop() -> Result<(), Tp3ErrorKind> {
    
    let (my_settings, mut pack, ns) = Settings::create_settings([192, 168, 199, 11], 8088)?;

    match my_settings.mode {
        0 => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack)?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack)?;
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, np_tdc)
        },
        1 => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack)?;
            let laser_tdc = PeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack)?;
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, laser_tdc)
        },
        2 => {
            let spim_tdc = PeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut pack)?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack)?;
            let measurement = spimlib::Live::new();
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement)
        },
        6 => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack)?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack)?;
            chronolib::build_chrono(pack, ns, my_settings, frame_tdc, np_tdc)
        },
        _ => Err(Tp3ErrorKind::MiscModeNotImplemented(my_settings.mode)),
    }
}

fn main() {
    let dir = Path::new("/Microscope/Log/");
    create_dir_all(&dir).expect("Could not create the log directory.");
    
    let date = Local::now().format("%Y-%m-%d").to_string() + ".txt";
    let file_path = dir.join(&date);
    let mut file = OpenOptions::new().write(true).truncate(false).create(true).append(true).open(file_path).expect("Could not create log file.");
    let date = Local::now().to_string();
    file.write(date.as_bytes()).unwrap();
    file.write(b" - Starting new loop\n").unwrap();
    loop {
        match connect_and_loop() {
            Ok(()) => {
                let date = Local::now().to_string();
                file.write(date.as_bytes()).unwrap();
                file.write(b" - OK\n").unwrap();
            },
            Err(e) => {
                let date = Local::now().to_string();
                file.write(date.as_bytes()).unwrap();
                file.write(b" - ERROR ").unwrap();
                let error = format!("{:?}", e);
                file.write(error.as_bytes()).unwrap();
                file.write(b"\n").unwrap();
            },
        }
    }
}

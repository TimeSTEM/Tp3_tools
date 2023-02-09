use timepix3::errorlib::Tp3ErrorKind;
use timepix3::auxiliar::*;
use timepix3::tdclib::{*, isi_box::*};
use timepix3::{speclib, speclib::IsiBoxKind, spimlib, spimlib::SpimKind};
use timepix3::isi_box_new;
use std::{thread, time};
use std::convert::TryInto;
use timepix3::constlib::*;


fn connect_and_loop() -> Result<u8, Tp3ErrorKind> {
    
    let (my_settings, mut pack, ns) = Settings::create_settings([192, 168, 199, 11], 8088)?;

    match my_settings.mode {
        0 if my_settings.bin => {
            let meas = speclib::SpecMeasurement::<speclib::Live1D, u32>::isi_new(&my_settings);
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack, None)?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoRisingEdge, &mut pack, None)?;
            speclib::build_spectrum_isi(pack, ns, my_settings, frame_tdc, np_tdc, meas)?;
            Ok(my_settings.mode)
        },
        0 if !my_settings.bin => {
            speclib::run_spectrum(pack, ns, my_settings, speclib::Live2D)?;
            Ok(my_settings.mode)
        },
        2 => {
            let mut handler = isi_box_new!(spim);
            handler.bind_and_connect()?;
            handler.configure_scan_parameters(my_settings.xspim_size.try_into().unwrap(), my_settings.yspim_size.try_into().unwrap(), my_settings.pixel_time.try_into().unwrap())?;
            handler.configure_measurement_type(false)?;
            handler.start_threads();
            
            let spim_tdc = PeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut pack, Some(my_settings.yspim_size))?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoRisingEdge, &mut pack, None)?;
            let measurement = spimlib::Live::new();
            spimlib::build_spim_isi(pack, ns, my_settings, spim_tdc, np_tdc, measurement, handler)?;
            Ok(my_settings.mode)
        },
        8 => {
            let mut handler = isi_box_new!(spec);
            handler.bind_and_connect()?;
            handler.configure_scan_parameters(32, 32, 8334)?;
            handler.configure_measurement_type(true)?;
            thread::sleep(time::Duration::from_millis(1000));
            Ok(my_settings.mode)
        },
        _ => Err(Tp3ErrorKind::IsiBoxAttempt(my_settings.mode)),
    }
}

fn main() {
    let mut log_file = simple_log::start().unwrap();
    loop {
        match connect_and_loop() {
            Ok(val) => {
                simple_log::ok(&mut log_file, val).unwrap();
            },
            Err(e) => {
                println!("Error in measurement. Error message: {:?}.", e);
                simple_log::error(&mut log_file, e).unwrap();
            },
        }
    }
}

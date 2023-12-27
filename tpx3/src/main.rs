use timepix3::errorlib::Tp3ErrorKind;
use timepix3::auxiliar::{value_types::*, *};
use timepix3::tdclib::*;
use timepix3::constlib::*;
use timepix3::{speclib, spimlib, spimlib::SpimKind};


fn connect_and_loop() -> Result<u8, Tp3ErrorKind> {
    
    let (my_settings, mut pack, ns) = Settings::create_settings(NIONSWIFT_IP_ADDRESS, NIONSWIFT_PORT)?;

    match my_settings.mode {
        0 if my_settings.bin => {
            speclib::run_spectrum(pack, ns, my_settings, speclib::Live1D)?;
            Ok(my_settings.mode)
        },
        0 if !my_settings.bin => {
            speclib::run_spectrum(pack, ns, my_settings, speclib::Live2D)?;
            Ok(my_settings.mode)
        },
        1 if my_settings.bin => {
            speclib::run_spectrum(pack, ns, my_settings, speclib::LiveTR1D)?;
            Ok(my_settings.mode)
        },
        1 if !my_settings.bin => {
            speclib::run_spectrum(pack, ns, my_settings, speclib::LiveTR2D)?;
            Ok(my_settings.mode)
        },
        2 => {
            let spim_tdc = TdcRef::new_periodic(TdcType::TdcOneFallingEdge, &mut pack, Some(my_settings.yspim_size as COUNTER), 1)?;
            let np_tdc = TdcRef::new_no_read(TdcType::TdcTwoRisingEdge)?;
            let measurement = spimlib::Live::new(&my_settings);
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement, None)?;
            Ok(my_settings.mode)
        },
        3 => {
            let spim_tdc = TdcRef::new_periodic(TdcType::TdcOneFallingEdge, &mut pack, Some(my_settings.yspim_size as COUNTER), 1)?;
            let np_tdc = TdcRef::new_no_read(TdcType::TdcTwoRisingEdge)?;
            let measurement = spimlib::LiveFrame4D::new(&my_settings);
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement, None)?;
            Ok(my_settings.mode)
        },
        6 => {
            speclib::run_spectrum(pack, ns, my_settings, speclib::FastChrono)?;
            Ok(my_settings.mode)
        },
        7 => {
            //speclib::run_spectrum(pack, ns, my_settings, speclib::Chrono)?;
            speclib::run_spectrum(pack, ns, my_settings, speclib::Coincidence2D)?;
            Ok(my_settings.mode)
        },
        10 if my_settings.bin => {
            speclib::run_spectrum(pack, ns, my_settings, speclib::Live1DFrame)?;
            Ok(my_settings.mode)
        },
        10 if !my_settings.bin => {
            speclib::run_spectrum(pack, ns, my_settings, speclib::Live2DFrame)?;
            Ok(my_settings.mode)
        },
        11 => {
            speclib::run_spectrum(pack, ns, my_settings, speclib::Live1DFrameHyperspec)?;
            Ok(my_settings.mode)
        },
        12 => {
            let spim_tdc = TdcRef::new_periodic(TdcType::TdcOneFallingEdge, &mut pack, Some(my_settings.yspim_size as COUNTER), 1)?;
            let np_tdc = TdcRef::new_no_read(TdcType::TdcTwoRisingEdge)?;
            let measurement = spimlib::LiveCoincidence::new(&my_settings);
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement, None)?;
            Ok(my_settings.mode)
        },
        13 => {
            let spim_tdc = TdcRef::new_periodic(TdcType::TdcOneFallingEdge, &mut pack, Some(my_settings.yspim_size as COUNTER), 1)?;
            let np_tdc = TdcRef::new_no_read(TdcType::TdcTwoRisingEdge)?;
            let measurement = spimlib::Live4D::new(&my_settings);
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement, None)?;
            Ok(my_settings.mode)
        },
        14 => {
            let ratio = my_settings.xscan_size / my_settings.xspim_size;
            let number_of_points = my_settings.xscan_size * my_settings.yscan_size;
            let vec_list = spimlib::LiveScanList::create_list(&ns, number_of_points)?;
            let spim_tdc = TdcRef::new_periodic(TdcType::TdcOneFallingEdge, &mut pack, Some(my_settings.yspim_size as COUNTER), ratio)?;
            let np_tdc = TdcRef::new_no_read(TdcType::TdcTwoRisingEdge)?;
            let measurement = spimlib::LiveScanList::new(&my_settings);
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement, Some(&vec_list))?;
            Ok(my_settings.mode)
        },
        _ => Err(Tp3ErrorKind::MiscModeNotImplemented(my_settings.mode)),
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

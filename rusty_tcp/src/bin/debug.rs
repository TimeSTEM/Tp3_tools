use timepix3::errorlib::Tp3ErrorKind;
use timepix3::auxiliar::Settings;
use timepix3::tdclib::{TdcControl, TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
use timepix3::{speclib, spimlib, chronolib, speclib::SpecKind, spimlib::SpimKind};


fn connect_and_loop() -> Result<u8, Tp3ErrorKind> {
    
    let (my_settings, mut pack, ns) = Settings::create_debug_settings(false)?;

    match my_settings.mode {
        0 if my_settings.bin => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack)?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack)?;
            let measurement = speclib::SpecMeasurement::<speclib::Live1D>::new(&my_settings);
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, np_tdc, measurement)?;
            Ok(my_settings.mode)
        },
        0 if !my_settings.bin => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack)?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack)?;
            //let measurement = speclib::SpecMeasurement::<speclib::Live2D>::new(&my_settings);
            let measurement = speclib::SpecMeasurement::<speclib::SuperResolution>::new(&my_settings);
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, np_tdc, measurement)?;
            Ok(my_settings.mode)
        },
        1 => {
            //let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack)?;
            //let laser_tdc = PeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack)?;
            //speclib::build_spectrum(pack, ns, my_settings, frame_tdc, laser_tdc)?;
            Ok(my_settings.mode)
        },
        2 => {
            let spim_tdc = PeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut pack)?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack)?;
            let measurement = spimlib::Live::new();
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement)?;
            Ok(my_settings.mode)
        },
        6 => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack)?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack)?;
            chronolib::build_chrono(pack, ns, my_settings, frame_tdc, np_tdc)?;
            Ok(my_settings.mode)
        },
        _ => Err(Tp3ErrorKind::MiscModeNotImplemented(my_settings.mode)),
    }
}

fn main() {
    match connect_and_loop() {
        Ok(val) => {println!("Measurement Over. Type is {}.", val);},
        Err(_e) => {},
    }
}

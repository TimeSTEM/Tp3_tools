use timepix3::errorlib::Tp3ErrorKind;
use timepix3::auxiliar::Settings;
use timepix3::tdclib::{TdcControl, TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
use timepix3::{speclib, spimlib, speclib::SpecKind, spimlib::SpimKind};


fn connect_and_loop() -> Result<u8, Tp3ErrorKind> {
    
    let (my_settings, mut pack, ns) = Settings::create_debug_settings(false)?;

    match my_settings.mode {
        0 if my_settings.bin => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack, None)?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack, None)?;
            let measurement = speclib::SpecMeasurement::<speclib::Live1D, u32>::new(&my_settings);
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, np_tdc, measurement)?;
            Ok(my_settings.mode)
        },
        0 if !my_settings.bin => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack, None)?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack, None)?;
            let measurement = speclib::SpecMeasurement::<speclib::Live2D, u32>::new(&my_settings);
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, np_tdc, measurement)?;
            Ok(my_settings.mode)
        },
        1 => {
            Ok(my_settings.mode)
        },
        2 => {
            let spim_tdc = PeriodicTdcRef::new(TdcType::TdcOneFallingEdge, &mut pack, Some(my_settings.yspim_size))?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack, None)?;
            let measurement = spimlib::Live::new();
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement)?;
            Ok(my_settings.mode)
        },
        6 => {
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

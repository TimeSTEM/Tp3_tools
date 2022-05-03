use timepix3::errorlib::Tp3ErrorKind;
use timepix3::auxiliar::{Settings, ConfigAcquisition};
use timepix3::tdclib::{TdcControl, TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
use timepix3::{speclib, spimlib, spimlib::SpimKind};
use std::env;


fn connect_and_loop() -> Result<u8, Tp3ErrorKind> {
    
    let args: Vec<String> = env::args().collect();
    let config_set = ConfigAcquisition::new(&args);
    
    let (my_settings, mut pack, ns) = Settings::create_debug_settings(&config_set)?;

    match my_settings.mode {
        0 if my_settings.bin => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack, None)?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack, None)?;
            speclib::run_spectrum(pack, ns, my_settings, frame_tdc, np_tdc, speclib::Live1D)?;
            Ok(my_settings.mode)
        },
        0 if !my_settings.bin => {
            let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack, None)?;
            let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoFallingEdge, &mut pack, None)?;
            speclib::run_spectrum(pack, ns, my_settings, frame_tdc, np_tdc, speclib::Live2D)?;
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
        Err(e) => {println!("Error in the debug measurement. Message is: {:?}", e)},
    }
}

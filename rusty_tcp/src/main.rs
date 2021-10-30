use timepix3::errorlib::Tp3ErrorKind;
use timepix3::auxiliar::Settings;
use timepix3::tdclib::{TdcControl, TdcType, PeriodicTdcRef, NonPeriodicTdcRef};
use timepix3::speclib;
use timepix3::spimlib; use timepix3::spimlib::SpimKind;
use timepix3::chronolib;

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
    loop {
        println!{"Waiting for a new client"};
        match connect_and_loop() {
            Ok(()) => {},
            Err(e) => {println!("{:?}", e);},
        }
    }
}

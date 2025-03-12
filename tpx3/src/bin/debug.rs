use timepix3::errorlib::Tp3ErrorKind;
use timepix3::auxiliar::Settings;
use timepix3::tdclib::{TdcType, TdcRef};
use timepix3::{speclib, speclib::SpecKind, spimlib, spimlib::SpimKind};


fn connect_and_loop() -> Result<u8, Tp3ErrorKind> {
    
    let (my_settings, mut pack, ns) = Settings::create_debug_settings()?;
    let mut file_to_write = my_settings.create_file()?;

    match my_settings.mode {
        0 if my_settings.bin => {
            let measurement = speclib::Live1D::new(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let aux_tdc = measurement.build_aux_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement, file_to_write)?;
            Ok(my_settings.mode)
        },
        0 if !my_settings.bin => {
            let measurement = speclib::Live2D::new(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let aux_tdc = measurement.build_aux_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement, file_to_write)?;
            Ok(my_settings.mode)
        },
        1 => {
            Ok(my_settings.mode)
        },
        2 => {
            let spim_tdc = TdcRef::new_periodic(TdcType::TdcOneFallingEdge, &mut pack, &my_settings, &mut file_to_write)?;
            let np_tdc = TdcRef::new_no_read(TdcType::TdcTwoFallingEdge)?;
            let measurement = spimlib::Live::new(&my_settings);
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement, None, file_to_write)?;
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

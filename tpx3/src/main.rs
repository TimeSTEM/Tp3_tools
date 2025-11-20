use timepix3::errorlib::Tp3ErrorKind;
use timepix3::auxiliar::*;
use timepix3::tdclib::*;
use timepix3::constlib::*;
use timepix3::ttx;
use timepix3::{speclib, speclib::SpecKind, spimlib, spimlib::SpimKind};
use std::thread;
use std::time::Duration;


fn connect_and_loop() -> Result<u8, Tp3ErrorKind> {

    //let x = ttx::TimeTagger::new();
    let mut x = ttx::TTXRef::new(true, 2048);
    //x.start_stream();
    //thread::sleep(Duration::from_millis(250));
    x.prepare_periodic(vec![1]);
    for _ in 0..10 {
        x.build_data();
    }
    //x.add_channel(1, true);
    //x.start_stream();
    //println!("here");
    //x.get_data();

    //let data = x.get_timestamps();
    //let result = ttx::determine_spread_period(&data);
    drop(x);

    let (my_settings, mut pack, ns) = Settings::create_settings(NIONSWIFT_IP_ADDRESS, NIONSWIFT_PORT)?;
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
        2 => {
            let mut measurement = spimlib::Live::new(&my_settings);
            let spim_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let np_tdc = measurement.build_aux_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement, None, file_to_write)?;
            Ok(my_settings.mode)
        },
        3 => {
            let mut measurement = spimlib::LiveFrame4D::new(&my_settings);
            let spim_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let np_tdc = measurement.build_aux_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement, None, file_to_write)?;
            Ok(my_settings.mode)
        },
        6 => {
            let measurement = speclib::Chrono::new(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let aux_tdc = measurement.build_aux_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement, file_to_write)?;
            Ok(my_settings.mode)
        },
        7 => {
            let measurement = speclib::Coincidence2D::new(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let aux_tdc = measurement.build_aux_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement, file_to_write)?;
            Ok(my_settings.mode)
        },
        8 => {
            let measurement = speclib::ChronoFrame::new(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let aux_tdc = measurement.build_aux_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement, file_to_write)?;
            Ok(my_settings.mode)
        },
        10 if my_settings.bin => {
            let measurement = speclib::Live1DFrame::new(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let aux_tdc = measurement.build_aux_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement, file_to_write)?;
            Ok(my_settings.mode)
        },
        10 if !my_settings.bin => {
            let measurement = speclib::Live2DFrame::new(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let aux_tdc = measurement.build_aux_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement, file_to_write)?;
            Ok(my_settings.mode)
        },
        11 => {
            let measurement = speclib::Live1DFrameHyperspec::new(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let aux_tdc = measurement.build_aux_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement, file_to_write)?;
            Ok(my_settings.mode)
        },
        12 => {
            let mut measurement = spimlib::LiveCoincidence::new(&my_settings);
            let spim_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let np_tdc = measurement.build_aux_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement, None, file_to_write)?;
            Ok(my_settings.mode)
        },
        13 => {
            let mut measurement = spimlib::Live4D::new(&my_settings);
            let spim_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let np_tdc = measurement.build_aux_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement, None, file_to_write)?;
            Ok(my_settings.mode)
        },
        14 => {
            let number_of_points = my_settings.xscan_size * my_settings.yscan_size;
            let vec_list = misc::create_list(&ns, number_of_points)?;
            let spim_tdc = TdcRef::new_periodic(TdcType::TdcOneFallingEdge, &mut pack, &my_settings, &mut file_to_write)?;
            let np_tdc = TdcRef::new_no_read(TdcType::TdcTwoRisingEdge)?;
            let measurement = spimlib::Live::new(&my_settings);
            spimlib::build_spim(pack, ns, my_settings, spim_tdc, np_tdc, measurement, Some(&vec_list), file_to_write)?;
            Ok(my_settings.mode)
        },
        15 => {
            let measurement = speclib::Live2DFrameHyperspec::new(&my_settings);
            let frame_tdc = measurement.build_main_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            let aux_tdc = measurement.build_aux_tdc(&mut pack, &my_settings, &mut file_to_write)?;
            speclib::build_spectrum(pack, ns, my_settings, frame_tdc, aux_tdc, measurement, file_to_write)?;
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

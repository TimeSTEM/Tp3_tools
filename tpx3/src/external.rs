pub mod external_library_coincidence {
    use crate::postlib::coincidence;
    use std::ffi::CStr;
    use crate::clusterlib::cluster;
    use crate::auxiliar::Settings;
    use crate::auxiliar::value_types::*;
    
    #[no_mangle]
    pub extern "C" fn create_electron_data(dir: *const i8, save_locally: bool) -> *mut coincidence::ElectronData {
        //Convert the directory *const u8 to CStr
        let c_str = unsafe { CStr::from_ptr(dir) };

        //Convert to a rust byte slice
        let bytes = c_str.to_bytes();

        //Get the string slice
        let str_slice = std::str::from_utf8(&bytes).expect("Could not convert to string slice.");
        
        //Get the settings
        let settings = Settings::get_settings_from_json(&str_slice[0..bytes.len()-5]).expect("JSON not properly open.");

        //Creating the electron data structure
        let coinc_data = coincidence::ElectronData::new(str_slice.to_owned(), cluster::grab_cluster_correction("0"), settings, save_locally);

        //Returning the RAW pointer
        Box::into_raw(Box::new(coinc_data))
    }

    #[no_mangle]
    pub extern "C" fn search_coincidence_external(coinc_data: *mut coincidence::ElectronData, limit_read_size: u32) -> u8 {
        let deref = unsafe {&mut *coinc_data };
        if let Err(_) = deref.prepare_to_search() {
            println!("***External***: Could not prepare to search.");
        };
        coincidence::search_coincidence(deref, limit_read_size);
        0
    }

    #[no_mangle]
    pub extern "C" fn get_array_length(coinc_data: *mut coincidence::ElectronData, length: *mut u32) {
        unsafe {
            *length = (*coinc_data).get_electron_collection().len() as u32;
        }
    }
    
    #[no_mangle]
    pub extern "C" fn get_reduced_raw(coinc_data: *mut coincidence::ElectronData, length: u32, array: *mut u64) {
        unsafe {
            let length = length as usize;
            let python_data = std::slice::from_raw_parts_mut(array, length);
            let rust_data = (*coinc_data).create_reduced_raw();
            if rust_data.len() != length {
                return; //The sizes must be the same otherwise there is a problem.
            }
            python_data.copy_from_slice(rust_data);
        }
    }

    #[no_mangle]
    pub extern "C" fn get_condensed_packet(coinc_data: *mut coincidence::ElectronData, length: u32, array: *mut u64) {
        unsafe {
            let length = length as usize;
            let python_data = std::slice::from_raw_parts_mut(array, length);
            let rust_data = (*coinc_data).create_condensed_packet();
            if rust_data.len() != length {
                return; //The sizes must be the same otherwise there is a problem.
            }
            python_data.copy_from_slice(&rust_data);
        }
    }
    
    #[no_mangle]
    pub extern "C" fn get_x(coinc_data: *mut coincidence::ElectronData, length: u32, array: *mut u16) {
        unsafe {
            let length = length as usize;
            let python_data = std::slice::from_raw_parts_mut(array, length);
            let rust_data = (*coinc_data).create_x();
            if rust_data.len() != length {
                return; //The sizes must be the same otherwise there is a problem.
            }
            python_data.copy_from_slice(&rust_data);
        }
    }
    
    #[no_mangle]
    pub extern "C" fn get_y(coinc_data: *mut coincidence::ElectronData, length: u32, array: *mut u16) {
        unsafe {
            let length = length as usize;
            let python_data = std::slice::from_raw_parts_mut(array, length);
            let rust_data = (*coinc_data).create_y();
            if rust_data.len() != length {
                return; //The sizes must be the same otherwise there is a problem.
            }
            python_data.copy_from_slice(&rust_data);
        }
    }
    
    #[no_mangle]
    pub extern "C" fn get_time_relative(coinc_data: *mut coincidence::ElectronData, length: u32, array: *mut i16) {
        unsafe {
            let length = length as usize;
            let python_data = std::slice::from_raw_parts_mut(array, length);
            let rust_data = (*coinc_data).create_rel_time();
            if rust_data.len() != length {
                return; //The sizes must be the same otherwise there is a problem.
            }
            python_data.copy_from_slice(&rust_data);
        }
    }

    #[no_mangle]
    pub extern "C" fn get_time_absolute(coinc_data: *mut coincidence::ElectronData, length: u32, array: *mut TIME) {
        unsafe {
            let length = length as usize;
            let python_data = std::slice::from_raw_parts_mut(array, length);
            let rust_data = (*coinc_data).create_abs_time();
            if rust_data.len() != length {
                return; //The sizes must be the same otherwise there is a problem.
            }
            python_data.copy_from_slice(&rust_data);
        }
    }

    #[no_mangle]
    pub extern "C" fn get_channel(coinc_data: *mut coincidence::ElectronData, length: u32, array: *mut u8) {
        unsafe {
            let length = length as usize;
            let python_data = std::slice::from_raw_parts_mut(array, length as usize);
            let rust_data = (*coinc_data).create_channel();
            if rust_data.len() != length {
                return; //The sizes must be the same otherwise there is a problem.
            }
            python_data.copy_from_slice(&rust_data);
        }
    }

    #[no_mangle]
    pub extern "C" fn get_tot(coinc_data: *mut coincidence::ElectronData, length: u32, array: *mut u16) {
        unsafe {
            let length = length as usize;
            let python_data = std::slice::from_raw_parts_mut(array, length);
            let rust_data = (*coinc_data).create_tot();
            if rust_data.len() != length {
                return; //The sizes must be the same otherwise there is a problem.
            }
            python_data.copy_from_slice(&rust_data);
        }
    }
     
    #[no_mangle]
    pub extern "C" fn get_spim_index(coinc_data: *mut coincidence::ElectronData, length: u32, array: *mut INDEXHYPERSPEC) {
        unsafe {
            let length = length as usize;
            let python_data = std::slice::from_raw_parts_mut(array, length);
            let rust_data = (*coinc_data).create_spim_index();
            if rust_data.len() != length {
                return; //The sizes must be the same otherwise there is a problem.
            }
            python_data.copy_from_slice(&rust_data);
        }
    }

    #[no_mangle]
    pub extern "C" fn free_electron_data(coinc_data: *mut coincidence::ElectronData) {
        unsafe { drop(Box::from_raw(coinc_data)) } ;
    }

}

pub mod external_library_clusterlib {
    use crate::clusterlib::cluster;
    
    #[no_mangle]
    pub extern "C" fn get_x_from_collection(_collection: *mut cluster::CollectionElectron) {
    }
}

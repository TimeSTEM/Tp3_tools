//!`tdclib` is a collection of tools to facilitate manipulation and choice of tdcs. Module is built
//!in around `TdcType` enum.

///The four types of TDC's.
pub enum TdcType {
    TdcOneRisingEdge,
    TdcOneFallingEdge,
    TdcTwoRisingEdge,
    TdcTwoFallingEdge,
}

impl TdcType {
    ///Convenient method. Return value is the 4 bits associated to each TDC.
    pub fn associate_value(&self) -> u8 {
        match *self {
            TdcType::TdcOneRisingEdge => 15,
            TdcType::TdcOneFallingEdge => 10,
            TdcType::TdcTwoRisingEdge => 14,
            TdcType::TdcTwoFallingEdge => 11,
        }
    }

    ///Convenient method from enumerator to a standard print.
    pub fn associate_string(&self) -> &str {
        match *self {
            TdcType::TdcOneRisingEdge => "One_Rising",
            TdcType::TdcOneFallingEdge => "One_Falling",
            TdcType::TdcTwoRisingEdge => "Two_Rising",
            TdcType::TdcTwoFallingEdge => "Two_Falling",
        }
    }
    
    ///From associate value to enum TdcType.
    pub fn associate_value_to_enum(value: u8) -> Result<TdcType, &'static str> {
        match value {
            15 => Ok(TdcType::TdcOneRisingEdge),
            10 => Ok(TdcType::TdcOneFallingEdge),
            14 => Ok(TdcType::TdcTwoRisingEdge),
            11 => Ok(TdcType::TdcTwoFallingEdge),
            _ => Err("Bad TDC receival"),
        }
    }
    
    ///Reads the vector and counts the number of found TDC's. Return array is sorted as the TdcType
    ///enum. Index 0 is, therefore, the number of rising edges in the first tdc found.
    fn count_tdcs(tdc_vec: &Vec<(f64, TdcType)>) -> [usize; 4] {
        let mut result = [0usize; 4];
        for (_time, tdc_type) in tdc_vec {
            match tdc_type {
                TdcType::TdcOneRisingEdge => result[0]+=1,
                TdcType::TdcOneFallingEdge => result[1]+=1,
                TdcType::TdcTwoRisingEdge => result[2]+=1,
                TdcType::TdcTwoFallingEdge => result[3]+=1,
            }
        }
        result
    }

    ///This method returns True if the number of TDC's found for each TDC is greater or equal the
    ///minimal input array. False otherwise. This method can be used to enforce that an specific
    ///TDC requirement is fullfilled before an acquisition.
    pub fn check_all_tdcs(min: &[usize; 4], tdc_vec: &Vec<(f64, TdcType)>) -> bool {
        let val = TdcType::count_tdcs(tdc_vec);
        let how_many = val.iter().zip(min.iter()).filter(|(min, val)| min>=val).count();
        match how_many {
            4 => true,
            _ => false,
        }
    }
    
    ///Similar to `check_all_tdcs` but return a 4-dimensional vector with booleans for each TDC.
    pub fn check_each_tdc(min: &[usize; 4], tdc_vec: &Vec<(f64, TdcType)>) -> Vec<bool> {
        let val = TdcType::count_tdcs(tdc_vec);
        let seq = val.iter().zip(min.iter()).map(|(min, val)| min>=val).collect::<Vec<bool>>();
        seq
    }
    
    ///Outputs the time list for a specific TDC.
    pub fn vec_from_tdc(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: u8) -> Vec<f64> {
        let result: Vec<_> = tdc_vec.iter()
            .filter(|(_time, tdct)| tdct.associate_value()==tdc_type)
            .map(|(time, _tdct)| *time)
            .collect();
        result
    }

    ///Outputs the last encountered time for a specific TDC.
    pub fn last_time_from_tdc(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: u8) -> f64 {
        let last_time = tdc_vec.iter()
            .filter(|(_time, tdct)| tdct.associate_value()==tdc_type)
            .map(|(time, _tdct)| *time)
            .last().unwrap();
        last_time
    }
    
    ///Outputs the number of encountered TDC for a specific TDC.
    pub fn howmany_from_tdc(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: u8) -> usize {
        let counter = tdc_vec.iter()
            .filter(|(_time, tdct)| tdct.associate_value()==tdc_type)
            .count();
        counter
    }

    ///Returns the deadtime between consecutive scan lines.
    pub fn find_width(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: u8) -> f64 {
        let fal_tdc_type = match tdc_type {
            10 | 15 => 10,
            11 | 14 => 11,
            _ => panic!("Bad TDC receival in `find_width`"),
        };
        
        let ris_tdc_type = match tdc_type {
            10 | 15 => 15,
            11 | 14 => 14,
            _ => panic!("Bad TDC receival in `find_width`"),
        };

        let fal = TdcType::vec_from_tdc(tdc_vec, fal_tdc_type);
        let ris = TdcType::vec_from_tdc(tdc_vec, ris_tdc_type);
        if (fal[0] - ris[0])>0.0 {fal[0] - ris[0]} else {fal[1] - ris[0]}
    }

    ///Returns the effective time interval between lines.
    pub fn find_period(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: u8) -> f64 {
        let tdc_time = TdcType::vec_from_tdc(tdc_vec, tdc_type);
        tdc_time[2] - tdc_time[1]
    }
}

pub enum TdcType {
    TdcOneRisingEdge,
    TdcOneFallingEdge,
    TdcTwoRisingEdge,
    TdcTwoFallingEdge,
}

impl TdcType {
    pub fn associate_value(&self) -> u8 {
        match *self {
            TdcType::TdcOneRisingEdge => 15,
            TdcType::TdcOneFallingEdge => 10,
            TdcType::TdcTwoRisingEdge => 14,
            TdcType::TdcTwoFallingEdge => 11,
        }
    }

    pub fn associate_string(&self) -> &str {
        match *self {
            TdcType::TdcOneRisingEdge => "One_Rising",
            TdcType::TdcOneFallingEdge => "One_Falling",
            TdcType::TdcTwoRisingEdge => "Two_Rising",
            TdcType::TdcTwoFallingEdge => "Two_Falling",
        }
    }
    
    pub fn associate_value_to_enum(value: u8) -> Result<TdcType, &'static str> {
        match value {
            15 => Ok(TdcType::TdcOneRisingEdge),
            10 => Ok(TdcType::TdcOneFallingEdge),
            14 => Ok(TdcType::TdcTwoRisingEdge),
            11 => Ok(TdcType::TdcTwoFallingEdge),
            _ => Err("Bad TDC receival"),
        }
    }
    
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

    pub fn check_all_tdcs(min: &[usize; 4], tdc_vec: &Vec<(f64, TdcType)>) -> bool {
        let val = TdcType::count_tdcs(tdc_vec);
        let how_many = val.iter().zip(min.iter()).filter(|(min, val)| min>=val).count();
        match how_many {
            4 => true,
            _ => false,
        }
    }
    
    pub fn check_each_tdc(min: &[usize; 4], tdc_vec: &Vec<(f64, TdcType)>) -> Vec<bool> {
        let val = TdcType::count_tdcs(tdc_vec);
        let seq = val.iter().zip(min.iter()).map(|(min, val)| min>=val).collect::<Vec<bool>>();
        seq
    }
    
    pub fn vec_from_tdc(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: u8) -> Vec<f64> {
        let result: Vec<_> = tdc_vec.iter()
            .filter(|(_time, tdct)| tdct.associate_value()==tdc_type)
            .map(|(time, _tdct)| *time)
            .collect();
        result
    }

    pub fn last_time_from_tdc(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: u8) -> f64 {
        let last_time = tdc_vec.iter()
            .filter(|(_time, tdct)| tdct.associate_value()==tdc_type)
            .map(|(time, _tdct)| *time)
            .last().unwrap();
        last_time
    }
    
    pub fn howmany_from_tdc(tdc_vec: &Vec<(f64, TdcType)>, tdc_type: u8) -> usize {
        let counter = tdc_vec.iter()
            .filter(|(_time, tdct)| tdct.associate_value()==tdc_type)
            .count();
        counter
    }
    
}

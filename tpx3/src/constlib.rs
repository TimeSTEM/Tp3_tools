use crate::auxiliar::value_types::*;

//***General Values***//
pub const TIME_INTERVAL_FRAMES: u128 = 10; //in milliseconds
pub const VIDEO_TIME: TIME = 3200;
pub const SPIM_PIXELS: POSITION = 1025 + 200;
pub const BUFFER_SIZE: usize = 16384 * 2;
pub const NIONSWIFT_IP_ADDRESS: [u8; 4] = [192, 168, 199, 11];
pub const NIONSWIFT_PORT: u16 = 8088;
pub const PHOTON_LIST_STEP: usize = 10; //How many photons in the list before a step is taken during coicncidence searching

//***POSTLIB***//
//Coincidence values using the IsiBox//
pub const ISI_BUFFER_SIZE: usize = 128_000_000; //Buffer size when reading files
pub const ISI_TIME_WIDTH: u64 = 200; //Time width considered for coincidence (units of 1.5625 ns)
pub const ISI_TIME_DELAY: u64 = 78; //Time delay considered for coincidence (units of 1.5625 ns)
pub const ISI_LINE_OFFSET: i64 = 0; //Line offset when searching coincidences
pub const ISI_TP3_MAX_DIF: u64 = 1000; //Maximum clock difference to synchronize IsiBox and Timepix3 (in units of 1.5625 ns)

//IsiBox alone constants//
pub const ISI_CORRECTION_MAX_DIF: u64 = 1_000; //Maximum clock difference between two detected lines. If the difference is bigger than this value, a new line is put in between (in units of 120 ps)
pub const ISI_NB_CORRECTION_ITERACTION: u64 = 100; //How many times your IsiBox will execute the line check algorithm. 


//Coincidence values using the Timepix3//
pub const TP3_BUFFER_SIZE: usize = 512_000_000; //Buffer size when reading files
pub const TP3_TIME_WIDTH: u64 = 50; //Time width considered for coincidence
pub const TP3_TIME_DELAY: u64 = 104; //Time delay considered for coincidence

//***TDCLIB***//
pub const CHANNELS: usize = 200;
pub const ISI_IP_PORT: &str = "192.168.199.10:9592";
pub const THREAD_POOL_PERIOD: u64 = 10; //Pooling time from socket thread for the IsiBox;

//***4D STEM***//
pub const MASK_FILE: &str = "C:\\ProgramData\\Microscope\\masks.dat";
pub const DETECTOR_SIZE: (POSITION, POSITION) = (256, 256);
pub const DETECTOR_LIMITS: ((POSITION, POSITION), (POSITION, POSITION)) = ((512, 768), (0, 256));
pub const MAX_CHANNELS: usize = 8;
pub const TIME_INTERVAL_4DFRAMES: u128 = 100; //In milliseconds

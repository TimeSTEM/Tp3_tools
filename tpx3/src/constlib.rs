use crate::auxiliar::value_types::*;

//***General Values***//
pub const CONFIG_SIZE: usize = 512;
pub const TIME_INTERVAL_FRAMES: u128 = 50; //in milliseconds
pub const HYPERSPECTRAL_PIXEL_CHUNK: POSITION = 500; //In number of pixels
pub const TIME_INTERVAL_COINCIDENCE_HISTOGRAM: u128 = 2000; //in milliseconds
pub const VIDEO_TIME: TIME = 0;
pub const PHOTON_LIST_STEP: usize = 10; //How many photons in the list before a step is taken during coincidence searching
pub const LIST_SIZE_AUX_EVENTS: usize = 5;
pub const ELECTRON_OVERFLOW: TIME = 17_179_869_184;
pub const TDC_OVERFLOW: TIME = 68_719_476_736;
pub const SYNC_MODE: u8 = 0; //0 synchronizes on the frame, 1 synchronizes on the line.
pub const REMOVE_RETURN: bool = true; //This removes the electrons in the flyback mode. UNIFORM_PIXEL must be false to this in order to take place.
pub const INTERNAL_TIMER_FRAME: bool = false; //If true, the TDC is not needed for event-based acquisition in focus mode;
pub const HIGH_DYNAMIC_FRAME_BASED: bool = false; //This sums up 10 frames when using the frame-based mode;
pub const HIGH_DYNAMIC_FRAME_BASED_VALUE: COUNTER = 16; //This sums up *VALUE* frames when using the frame-based mode;

//***Connection, TCP, and transfer values***//
pub const BUFFER_SIZE: usize = 16384 * 2;
pub const NIONSWIFT_IP_ADDRESS: [u8; 4] = [192, 168, 0, 11];
pub const NIONSWIFT_PORT: u16 = 8088;
pub const SAVE_LOCALLY_FILE: &str = "/media/asi/Data21/TP3_Data/";
pub const READ_DEBUG_FILE: &str = "C:\\Users\\AUAD\\Documents\\Tp3_tools\\tpx3\\src\\bin\\Data\\raw000000_spim.tpx3";
pub const READ_DEBUG_FILE_JSON: &str = "C:\\Users\\AUAD\\Documents\\Tp3_tools\\tpx3\\src\\bin\\Data\\raw000000_spim";

//***Packet-related values***//
pub const PIXELS_X: POSITION = 1025;
pub const PIXELS_Y: POSITION = 256;
pub const INVERSE_DETECTOR: bool = true; //This mirror the detector in the dispersive direction (EELS);
pub const CORRECT_ELECTRON_TIME_COARSE: bool = true;

//***List***//
pub const UNIFORM_PIXEL: bool = false; //Assumption that the time per pixel is uniform.
pub const DACX_BITDEPTH: usize = 14;
pub const DACY_BITDEPTH: usize = 14;

//***POSTLIB***//
pub const LIMIT_READ: bool = false; //early break of the file processing
pub const LIMIT_READ_SIZE: usize = 5_000_000_000; //5GB limitations
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
pub const TP3_TIME_WIDTH: u64 = 200; //Time width considered for coincidence (in units of 1.5625 ns)
pub const TP3_TIME_DELAY: u64 = 100; //Time delay considered for coincidence (in units of 1.5625 ns)

//***TDCLIB***//
pub const TDC_TIMEOUT: u64 = 10;
pub const CHANNELS: usize = 200;
pub const ISI_IP_PORT: &str = "192.168.199.10:9592";
pub const THREAD_POOL_PERIOD: u64 = 10; //Pooling time from socket thread for the IsiBox;

//***4D STEM***//
pub type MaskValues = i16;
pub const MASK_FILE: &str = "C:\\ProgramData\\Microscope\\masks.dat";
//pub const MASK_FILE: &str = "/home/asi/CHROMATEM/masks.dat";
pub const DETECTOR_SIZE: (POSITION, POSITION) = (256, 256);
pub const DETECTOR_LIMITS: ((POSITION, POSITION), (POSITION, POSITION)) = ((512, 768), (0, 256));
pub const MAX_CHANNELS: usize = 8;
pub const TIME_INTERVAL_4DFRAMES: u128 = 100; //In milliseconds

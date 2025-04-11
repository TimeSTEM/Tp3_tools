use crate::auxiliar::value_types::*;
use crate::tdclib::TdcType;

//***General Values***//
pub const CONFIG_SIZE: usize = 512;
pub const TIME_INTERVAL_FRAMES: u128 = 200; //in milliseconds
pub const HYPERSPECTRAL_PIXEL_CHUNK: POSITION = 500; //In number of pixels
pub const TIME_INTERVAL_COINCIDENCE_HISTOGRAM: u128 = 2000; //in milliseconds
pub const VIDEO_TIME: TIME = 0;
pub const ELECTRON_OVERFLOW: TIME = 17_179_869_184;
pub const TDC_OVERFLOW: TIME = 68_719_476_736;
pub const SYNC_MODE: u8 = 0; //0 synchronizes on the frame, 1 synchronizes on the line.
pub const REMOVE_RETURN: bool = true; //This removes the electrons in the flyback mode. UNIFORM_PIXEL must be false to this in order to take place.
pub const HIGH_DYNAMIC_FRAME_BASED: bool = false; //This sums up *VALUE* frames when using the frame-based mode;
pub const HIGH_DYNAMIC_FRAME_BASED_VALUE: COUNTER = 16; //This sums up *VALUE* frames when using the frame-based mode;
pub const MAIN_TDC: TdcType = TdcType::TdcOneRisingEdge; //The main TDC, used for external sync
pub const SECONDARY_TDC: TdcType = TdcType::TdcTwoRisingEdge; //Secondary TDC
pub const PERIOD_DIVIDER: TIME = 65536; //This divides the period detected by the Timepix3. The divided value is PERIOD_DIVIDER;

//***Connection, TCP, and transfer values***//
pub const BUFFER_SIZE: usize = 16384 * 2;
pub const NIONSWIFT_IP_ADDRESS: [u8; 4] = [192, 168, 0, 11];
pub const NIONSWIFT_PORT: u16 = 8088;
pub const SAVE_LOCALLY_FILE: &str = "/media/asi/Data21/TP3_Data/";
pub const READ_DEBUG_FILE: &str = "C:\\Users\\AUAD\\Documents\\Tp3_tools\\tpx3\\src\\bin\\Data\\Test_TPX_ps_04042025/2025_04_04_14_22_47.tpx3";
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

//***Cluster settings***//
pub const CLUSTER_DET: TIME = 32; //Cluster time window (in 640 Mhz or 1.5625).
pub const CLUSTER_SPATIAL: isize = 4; // If electron hit position in both X or Y > CLUSTER_SPATIAL, then we have a new cluster.
pub static ATOT: &[u8; 1024 * 256 * 4] = include_bytes!("atot_v2.dat");
pub static BTOT: &[u8; 1024 * 256 * 4] = include_bytes!("btot_v2.dat");

//Coincidence values using the Timepix3//
pub const TP3_BUFFER_SIZE: usize = 512_000_000; //Buffer size when reading files
pub const PHOTON_LIST_STEP: usize = 5; //How many photons in the list before a step is taken during coincidence searching
pub const LIST_SIZE_AUX_EVENTS: usize = 4; //List size of Coincidence2D struct in speclib.
pub const CIRCULAR_BUFFER: usize = 4096;

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

//***POSTLIB***//
//Coincidence values using the IsiBox//
pub const ISI_BUFFER_SIZE: usize = 128_000_000; //Buffer size when reading files
pub const ISI_TIME_WIDTH: u64 = 200; //Time width considered for coincidence (units of 1.5625 ns)
pub const ISI_TIME_DELAY: u64 = 78; //Time delay considered for coincidence (units of 1.5625 ns)
pub const ISI_TP3_MAX_DIF: u64 = 1000; //Maximum clock difference to synchronize IsiBox and Timepix3 (in units of 1.5625 ns)


//IsiBox alone constants//
pub const ISI_CORRECTION_MAX_DIF: u64 = 1000; //Maximum clock difference between two detected lines. If the difference is bigger than this value, a new line is put in between (in units of 120 ps)
pub const ISI_NB_CORRECTION_ITERACTION: u64 = 100; //How many times your IsiBox will execute the line check algorithm. 


//Coincidence values using the Timepix3//
pub const TP3_BUFFER_SIZE: usize = 512_000_000; //Buffer size when reading files
pub const TP3_TIME_WIDTH: u64 = 50; //Time width considered for coincidence
pub const TP3_TIME_DELAY: u64 = 104; //Time delay considered for coincidence

//***TDCLIB***//
pub const CHANNELS: usize = 200;
pub const ISI_IP_PORT: &str = "192.168.199.10:9592";
pub const THREAD_POOL_PERIOD: u64 = 10; //Pooling time from socket thread for the IsiBox;

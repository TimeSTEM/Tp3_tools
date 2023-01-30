//Coincidence values using the IsiBox
pub const ISI_BUFFER_SIZE: usize = 512_000_000; //Buffer size when reading files
pub const ISI_TIME_WIDTH: u64 = 200; //Time width considered for coincidence (units of 1.5625 ns)
pub const ISI_TIME_DELAY: u64 = 78; //Time delay considered for coincidence (units of 1.5625 ns)
pub const ISI_TP3_MAX_DIF: u64 = 1000; //Maximum clock difference to synchronize IsiBox and Timepix3 (in units of 1.5625 ns)

//Coincidence values using the Timepix3
pub const TP3_BUFFER_SIZE: usize = 512_000_000; //Buffer size when reading files
pub const TP3_TIME_WIDTH: u64 = 50; //Time width considered for coincidence
pub const TP3_TIME_DELAY: u64 = 104; //Time delay considered for coincidence

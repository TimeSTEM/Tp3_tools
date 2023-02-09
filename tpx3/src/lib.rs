//!# Timepix3
//!
//!`timepix3` is a collection of tools to run and analyze the detector TimePix3 in live conditions. This software is
//!intented to be run in a different computer in which the data will be shown. Raw data is supossed to
//!be collected via a socket in localhost and be sent to a client prefentiably using a 10 Gbit/s
//!Ethernet.

pub mod auxiliar;
pub mod constlib;
pub mod tdclib;
pub mod packetlib;
pub mod postlib;
pub mod speclib;
pub mod spimlib;
pub mod errorlib;
pub mod clusterlib;

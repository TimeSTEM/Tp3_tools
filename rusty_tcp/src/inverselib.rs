pub mod inverselib {
    pub struct InvPacketEELS {
        x: usize,
        y: usize,
        time: usize,
    }

    impl InvPacketEELS {
        fn time_to_ticks(&self) -> (usize, usize, usize) {
            let spidr_ticks = self.time / 409_600;
            let ctoa = self.time % 409_600;
            let toa_ticks = ctoa / 25;
            let ftoa_ticks = ctoa % 25;
            (spidr_ticks, toa_ticks, ftoa_ticks)
        }
    }



}

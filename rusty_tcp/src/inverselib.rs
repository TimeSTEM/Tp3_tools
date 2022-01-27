pub mod inverselib {
    use crate::packetlib::{PacketEELS as Pack};
    pub struct InvPacketEELS {
        pub x: usize,
        pub y: usize,
        pub time: usize,
    }

    impl InvPacketEELS {
        fn time_to_ticks(&self) -> (usize, usize, usize) {
            let spidr_ticks = self.time / 409_600;
            let ctoa = self.time % 409_600;
            let toa_ticks = ctoa / 25;
            let ftoa_ticks = ctoa % 25;
            (spidr_ticks, toa_ticks, ftoa_ticks)
        }

        pub fn create_electron_event(&self) -> Pack {
            let a = self.x as u8;
            let b = self.y as u8;
            let (_c, _d, _e) = self.time_to_ticks();
            let data: [u8; 8] = [a, b, 0, 1, 2, 0, 0, 0]; 
            Pack {
                chip_index: 0,
                data: data
            }
        }
                
    }
    
    #[test]
    fn it_works() {
        println!("k");
        assert_eq!(2+2, 4);
    }


}

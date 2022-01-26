pub mod inverselib {
    use crate::packetlib::{Packet, PacketEELS as Pack};
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

        fn create_electron_event(&self) {
            let data: [u8; 8] = [0, 0, 0, 0, 0, 0, 0, 0]; 
            let packet = Pack {
                chip_index: 0,
                data: &data
            };
        }
                
    }
    
    #[test]
    fn it_works() {
        println!("k");
        assert_eq!(2+2, 4);
    }


}

//!`timepix3` is a collection of tools to run and analyze the detector TimePix3 in live conditions. This software is
//!intented to be run in a different computer in which the data will be shown. Raw data is supossed to
//!be collected via a socket in localhost and be sent to a client prefentiably using a 10 Gbit/s
//!Ethernet.

pub mod auxiliar;
pub mod tdclib;
pub mod packetlib;
pub mod postlib;
pub mod speclib;
pub mod spimlib;
pub mod chronolib;
pub mod errorlib;
pub mod clusterlib;
pub mod inverselib;

/*
///`message_board` is a module containing tools to display HTTP based informations about the
///detector status.
pub mod message_board {
    use std::fs;
    use std::net::{TcpListener, TcpStream};
    use std::io::{Read, Write};

    pub fn start_message_board() {
        //let (mut mb_sock, mb_addr) = mb_listener.accept().expect("Could not connect to Message Board.");
        
        let mb_listener = TcpListener::bind("127.0.0.1:9098").expect("Could not bind to Message Board.");
        for stream in mb_listener.incoming() {
            let stream = stream.unwrap();
            handle_connection(stream);
        }
    }

    fn handle_connection(mut stream: TcpStream) {
        let mut buffer = [0; 1024];
        stream.read(&mut buffer).unwrap();
        let contents = fs::read_to_string("page.html").unwrap();
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{}",
            contents.len(),
            contents
        );
        stream.write(response.as_bytes()).unwrap();
        stream.flush().unwrap();
    }
}
*/

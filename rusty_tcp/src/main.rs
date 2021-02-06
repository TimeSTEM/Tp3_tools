use std::io::prelude::*;
use std::fs::File;
use std::net::TcpListener;
use std::time::Instant;
use std::thread;
use std::sync::mpsc;
 
struct Packet {
    chip_index: u8,
    i08: u8,
    i09: u8,
    i10: u8,
    i11: u8,
    i12: u8,
    i13: u8,
    i14: u8,
    i15: u8,
}

impl Packet {
    fn x(&self) -> usize {
        let mut temp = ((((self.i14 & 224))>>4 | ((self.i15 & 15))<<4) | (((self.i13 & 112)>>4)>>2)) as usize;
        match self.chip_index {
            0 => 255 - temp,
            1 => 255 * 4 - temp,
            2 => 255 * 3 - temp,
            3 => 255 * 2 - temp,
            _ => temp,
        }
    }
    
    fn y(&self) -> usize {
        (   ( ((self.i13 & 128))>>5 | ((self.i14 & 31))<<3 ) | ( (((self.i13 & 112)>>4)) & 3 )   ) as usize
    }

    fn id(&self) -> u8 {
        (self.i15 & 240) >> 4
    }

    fn spidr(&self) -> u16 {
        (self.i08 as u16) | (self.i09 as u16)<<8
    }

    fn ftoa(&self) -> u8 {
        self.i10 & 15
    }

    fn tot(&self) -> u8 {
        (self.i10 & 240)>>4 | (self.i11 & 63)<<4
    }

    fn toa(&self) -> u16 {
        ((self.i11 & 192) as u16)>>6 | (self.i12 as u16)<<2 | ((self.i13 & 15) as u16)<<10
    }

}

fn build_data(data: &[u8], bin: bool, section: usize) -> Vec<u8> {
    
    let file_data = data;
    let mut final_data = vec![0; 2048 * (256 - 255 * (bin as usize))];
    let mut index = 0;
    let mut packet_chunks = file_data.chunks_exact(8);

    while file_data.get(index..index+4) != Some(&[84, 80, 88, 51]) {
        packet_chunks.next();
        index+=1;
    }
    
    let mut ci: u8 = 0;
    loop {
        match packet_chunks.next() {
            None => break,
            Some(&[84, 80, 88, 51, nci, _, _, _]) => ci = nci,
            Some(x) => {
                let packet = Packet {
                    chip_index: ci,
                    i08: 0,
                    i09: 0,
                    i10: 0,
                    i11: 0,
                    i12: 0,
                    i13: x[5],
                    i14: x[6],
                    i15: x[7],
                };
                
                if packet.id()==11 {
                    let array_pos: usize;
                    match bin {
                        false => array_pos = 2*packet.x() + 2048*packet.y(),
                        true => array_pos = 2*packet.x()
                    };
                    final_data[array_pos] += 1;
                    if final_data[array_pos]==255 {
                        final_data[(array_pos+1)] += 1;
                        final_data[array_pos] = 0;
                    }
                }
                
            },
        }
    }
    final_data.push(10);
    final_data
}

fn has_data(number: u16) -> bool {
    let mut path: String = String::from("C:\\Users\\AUAD\\Documents\\Tp3_tools\\TCPFiletoStreamProcessed\\Files_00\\raw00000");
    let ct: String = number.to_string();
    path.push_str(&ct);
    path.push_str(".tpx3");

    match File::open(path) {
        Ok(_myfile) => true,
        Err(_) => false,
    }
}

fn open_and_read(number: u16) -> Vec<u8> {
    let mut path: String = String::from("C:\\Users\\AUAD\\Documents\\Tp3_tools\\TCPFiletoStreamProcessed\\Files_00\\raw00000");
    let ct: String = number.to_string();
    path.push_str(&ct);
    path.push_str(".tpx3");
    let mut buffer: Vec<u8> = Vec::new();
    
    if let Ok(myfile) = File::open(path) {
        let mut myfile = myfile;
        myfile.read_to_end(&mut buffer).expect("Error in read to buffer.");
    }
    buffer
}

fn create_header(time: f64, frame: u16, data_size: u32, bitdepth: u8, width: u16, height: u16) -> Vec<u8> {
    let mut msg: String = String::from("{\"timeAtFrame\":");
    msg.push_str(&(time.to_string()));
    msg.push_str(",\"frameNumber\":");
    msg.push_str(&(frame.to_string()));
    msg.push_str(",\"measurementID:\"Null\",\"dataSize\":");
    msg.push_str(&(data_size.to_string()));
    msg.push_str(",\"bitDepth\":");
    msg.push_str(&(bitdepth.to_string()));
    msg.push_str(",\"width\":");
    msg.push_str(&(width.to_string()));
    msg.push_str(",\"height\":");
    msg.push_str(&(height.to_string()));
    msg.push_str("}\n");
    let s: Vec<u8> = msg.into_bytes();
    s
}

fn from_16_to_8(data: &[u16]) -> Vec<u8> {
    let vec1 = data;
    let mut new_vec: Vec<u8> = vec1.iter().map(|&x| x as u8).collect();
    let mut it1 = vec1.iter().enumerate().filter(|&(a, b)| b>&255);
    for (i, val) in it1 {
        new_vec[i+1] += (val / 255) as u8;
        new_vec[i] = (val % 255) as u8;
    }
    new_vec
}

fn double_from_16_to_8(data: &[u16], data2: &[u16]) -> Vec<u8> {
    let vec1 = data;
    let vec2 = data2;
    let mut new_vec: Vec<u8> = vec1.iter().zip(vec2.iter()).map(|(a, b)| (a+b) as u8).collect();
    let mut it1 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a+b).enumerate().filter(|&(a, b)| b>255);
    for (i, val) in it1 {
        new_vec[i+1] += (val / 255) as u8;
        new_vec[i] = (val % 255) as u8;
    }
    new_vec
}

fn main() {

    let mut counter = 0u16;
    let mut bin: bool = true;

    let listener = TcpListener::bind("127.0.0.1:8088").unwrap();
    if let Ok((socket, addr)) = listener.accept() {
        println!("new client {:?}", addr);
        let mut sock = socket;
        
        
        let mut my_data = [0 as u8; 16];
        if let Ok(size) = sock.read(&mut my_data){
            match my_data[0] {
                0 => bin = false,
                1 => bin = true,
                _ => panic!("crash and burn"),
            };
        };
        
        let mut gt = 0;
        let start = Instant::now();
        loop {
            let msg = create_header(0.352, counter, 2048*(256-255*(bin as u32)), 16, 1024, 256 - 255*(bin as u16));

            while has_data(counter)==false {
                counter = 0;
            }
            
            let mydata = open_and_read(counter);
            let received = build_data(&mydata[..], bin, 1);
            
            /*
            let mydata2 = mydata.clone();
            let (tx, rx) = mpsc::channel();

            let tx1 = mpsc::Sender::clone(&tx);
            let tx2 = mpsc::Sender::clone(&tx);

            thread::spawn(move || {
                let val = build_data(&mydata[..], bin, 1);
                tx.send(val).unwrap();
            });
            
            thread::spawn(move || {
                let val = build_data(&mydata2[1000000..], bin, 1);
                tx1.send(val).unwrap();
            });
            
            let received = rx.recv().unwrap();
            let received2 = rx.recv().unwrap();
            let final_received = double_from_16_to_8(&received, &received2);
            */

            sock.write(&msg).expect("Header not sent.");
            sock.write(&received).expect("Data not sent.");
            counter+=1;
            gt+=1;
            if gt==1000 {
                let elapsed = start.elapsed();
                println!("Time elapsed for each 1000 iterations is: {:?}", elapsed);
                gt = 0;
            }
        }
    }
}

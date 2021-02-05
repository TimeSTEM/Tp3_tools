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

}

fn correct_x_position(chip_index: u8, pos: u8) -> usize {
    let x = pos as usize;
    match chip_index {
        0 => 255 - x,
        1 => 255 * 4 - x,
        2 => 255 * 3 - x,
        3 => 255 * 2 - x,
        _ => x,
    }
}

fn build_data(data: &[u8], bin: bool, section: usize) -> [u8; 2049] {
    
    let file_data = data;
    //let mut file_data_iter = file_data.iter();
    let mut array_final_data = [0; 2049];
    let file_data_len = file_data.len();
    let mut index = 0;
    let mut total_size: usize;
    let mut chip_index: u8;

    while file_data.get(index..index+4) != Some(&[84, 80, 88, 51]) {
        index+=1;
    }
    
    while index < file_data.len()/section {
        assert_eq!(Some(&[84, 80, 88, 51][..]), file_data.get(index..index+4));
        
        chip_index = file_data[index+4];
        total_size = (file_data[index+6] as usize) | (file_data[index+7] as usize)<<8;

        if (index + (total_size as usize)) > file_data.len()/section {break}
        let _nloop = total_size / 8;
        for _i in 0.._nloop  {
        
            let packet = Packet {
                chip_index: chip_index,
                i08: file_data[index+8],
                i09: file_data[index+9],
                i10: file_data[index+10],
                i11: file_data[index+11],
                i12: file_data[index+12],
                i13: file_data[index+13],
                i14: file_data[index+14],
                i15: file_data[index+15],
            };
        
            /*
            _spidr = (file_data[index+8] as u16) | (file_data[index+9] as u16)<<8;
            _ftoa = file_data[index+10] & 15;
            _tot = ((file_data[index+10] & 240) as u16)>>4 | ((file_data[index+11] & 63) as u16)<<4;
            _toa = ((file_data[index+11] & 192) as u16)>>6 | (file_data[index+12] as u16)<<2 | ((file_data[13] & 15) as u16)<<10;
            */

            if packet.id()==11 {
                let array_pos = 2*packet.x();
                array_final_data[array_pos] += 1;
                if array_final_data[array_pos]==255 {
                    array_final_data[(array_pos+1)] += 1;
                    array_final_data[array_pos] = 0;
                }
            }
            

            index = index + 8;
        }
        index = index + 8;
    }
    array_final_data[2048] = 10;
    array_final_data
}


    fn has_data(number: u16) -> bool {
        let mut path: String = String::from("C:\\Users\\AUAD\\Documents\\Tp3_tools\\TCPFiletoStreamProcessed\\Files_00\\raw00000");
        let ct: String = number.to_string();
        path.push_str(&ct);
        path.push_str(".tpx3");
        
    let valid;
    
    if let Ok(_myfile) = File::open(path) {
        valid = true;
    }
    else {
        valid = false;
    }

    valid
}


fn open_and_read(number: u16) -> (Vec<u8>, bool) {
    let mut path: String = String::from("C:\\Users\\AUAD\\Documents\\Tp3_tools\\TCPFiletoStreamProcessed\\Files_00\\raw00000");
    let ct: String = number.to_string();
    path.push_str(&ct);
    path.push_str(".tpx3");
    let mut buffer: Vec<u8> = Vec::new();
    let valid;
    
    if let Ok(myfile) = File::open(path) {
        let mut myfile = myfile;
        myfile.read_to_end(&mut buffer).expect("Error in read to buffer.");
        valid = true;
    }
    else {
        valid = false;
    }
    (buffer, valid)
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

fn sum_vec(vec1: &[u8], vec2: &[u8]) -> Vec<u8> {
    let new_vec:Vec<u8> = vec1.iter().zip(vec2.iter()).map(|(a, b)| a+b).collect();
    new_vec
}

fn sum2_vec(vec1: &[u8], vec2: &[u8]) -> Vec<u8> {
    let mut new_vec:Vec<u8> = vec![127; 2048];
    for i in 0..vec1.len()-1 {
        let mut new_value = (vec1[i] as u16) + (vec2[i] as u16);
        if new_value > 255 {
            new_value = new_value - 255;
            new_vec[i+1] += 1;
        }
        new_vec[i] = new_value as u8;
    }
    new_vec.push(10);
    new_vec
}


fn main() {

    let mut counter = 0u16;

    let listener = TcpListener::bind("127.0.0.1:8088").unwrap();
    if let Ok((socket, addr)) = listener.accept() {
        println!("new client {:?}", addr);
        let mut sock = socket;
        
        /*
        let mut my_data = [0 as u8; 50];
        while let Ok(size) = sock.read(&mut my_data){
            sock.write(&my_data[0..size]);
            if size<50 {
                break
            }
        }
        */
        let mut global_counter = 0;
        let start = Instant::now();
        loop {
            let msg = create_header(0.352, counter, 2048, 16, 1024, 1);

            while has_data(counter)==false {
                counter = 0;
            }
            
            let (mydata, _) = open_and_read(counter);
            let mydata2 = mydata.clone();
            let mydatalen = mydata.len();
            
            let (tx, rx) = mpsc::channel();

            let tx1 = mpsc::Sender::clone(&tx);
            let tx2 = mpsc::Sender::clone(&tx);

            thread::spawn(move || {
                let val = build_data(&mydata, false, 1);
                tx.send(val).unwrap();
            });
            
            //thread::spawn(move || {
            //    let val = build_data(&mydata2[mydatalen/2..], false, 1);
            //    tx1.send(val).unwrap();
            //});
            
            let received = rx.recv().unwrap();
            //let received2 = rx.recv().unwrap();

            //let received_finish: Vec<u8> = sum_vec(&received, &received2);

            sock.write(&msg).expect("Header not sent.");
            sock.write(&received).expect("Data not sent.");
            counter+=1;
            global_counter+=1;
            if global_counter==500 {
                let elapsed = start.elapsed();
                println!("{} and {:?}", counter, elapsed);
            }
        }
    }
}

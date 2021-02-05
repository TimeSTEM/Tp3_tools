use std::io::prelude::*;
use std::fs::File;
use std::net::TcpListener;
use std::time::Instant;
use std::time::Duration;
use std::thread;
use std::sync::mpsc;

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

fn build_data(data: &[u8], bin: bool, section: usize) -> Vec<u8> {
    
    let file_data = data;
    let mut final_data = vec![0; 2048*(255 * (bin as usize) + 1)];
    let file_data_len = file_data.len();
    let mut index = 0;
    
    let mut total_size: u16;
    let mut chip_index: u8;
    let mut id: u8;
    /*
    let mut _spidr: u16;
    let mut _ftoa: u8;
    let mut _tot: u16;
    let mut _toa: u16;
    let mut _y: u16;
    */
    let mut pix: u8;
    let mut dcol: u8;
    let mut spix: u8;

    while file_data.get(index..index+4) != Some(&[84, 80, 88, 51]) {
        index+=1;
    }


    //println!("{}", index);
    while index < file_data.len()/section {
        assert_eq!(Some(&[84, 80, 88, 51][..]), file_data.get(index..index+4));
        
        chip_index = file_data[index+4];
        total_size = (file_data[index+6] as u16) | (file_data[index+7] as u16)<<8;

        let _nloop = total_size / 8;
        if (index + (total_size as usize)) > file_data.len()/section {break}
        for _i in 0.._nloop {
            /*
            _spidr = (file_data[index+8] as u16) | (file_data[index+9] as u16)<<8;
            _ftoa = file_data[index+10] & 15;
            _tot = ((file_data[index+10] & 240) as u16)>>4 | ((file_data[index+11] & 63) as u16)<<4;
            _toa = ((file_data[index+11] & 192) as u16)>>6 | (file_data[index+12] as u16)<<2 | ((file_data[13] & 15) as u16)<<10;
            */
            pix = (file_data[index+13] & 112)>>4;
            spix = ((file_data[index+13] & 128))>>5 | ((file_data[index+14] & 31))<<3;
            dcol = ((file_data[index+14] & 224))>>4 | ((file_data[index+15] & 15))<<4;
            id = (file_data[index+15] & 240) >> 4;
            let x: u8 = dcol | pix >> 2;
            let y: usize = (spix | (pix & 3)) as usize;

            let x = correct_x_position(chip_index, x);

            let array_pos = 2*x + 2048*y*(bin as usize);

            if id==11 {
                final_data[array_pos]+=1;
                if final_data[array_pos]==255 {
                    final_data[(array_pos+1)] += 1;
                    final_data[array_pos] = 0;
                }
            }
            index = index + 8;
        }
        index = index + 8;
    }
    final_data.push(10);
    final_data
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
    let mut new_vec:Vec<u8> = vec![127; 2048*256];
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
        loop {
            let start = Instant::now();
            let msg = create_header(0.352, counter, 524288, 16, 1024, 256);

            while has_data(counter)==false {
                counter = 0;
            }
            
            let (mydata, _) = open_and_read(counter);
            let mydata2 = mydata.clone();
            //let my_real_data = build_data(&mydata, false, 0, 1);
            
            let (tx, rx) = mpsc::channel();

            let tx1 = mpsc::Sender::clone(&tx);
            let tx2 = mpsc::Sender::clone(&tx);

            thread::spawn(move || {
                let val = build_data(&mydata[..], true, 1);
                tx.send(val).unwrap();
            });
            
            //thread::spawn(move || {
            //    let val = build_data(&mydata2[1000000..], true, 1);
            //    tx1.send(val).unwrap();
            //});
            
            let received = rx.recv().unwrap();
            //let received2 = rx.recv().unwrap();

            //let received_finish: Vec<u8> = sum2_vec(&received, &received2);

            sock.write(&msg).expect("Header not sent.");
            sock.write(&received).expect("Data not sent.");
            counter+=1;
            let elapsed = start.elapsed();
            println!("{} and {:?}", counter, elapsed);
        }
    }
}

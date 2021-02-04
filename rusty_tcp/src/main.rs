use std::io::prelude::*;
use std::fs::File;
use std::net::TcpListener;
use std::time::Instant;


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

    
fn build_data(data: &Vec<u8>) -> Vec<u8> {
    
    let file_data = data;
    let mut final_data = vec![0; 2048];
    let mut index = 0;
    
    let mut total_size: u16;
    let mut chip_index: u8;
    /*
    let mut _spidr: u16;
    let mut _ftoa: u8;
    let mut _tot: u16;
    let mut _toa: u16;
    let mut _y: u16;
    let mut _spix: u16;
    */
    let mut _pix: u8;
    let mut _dcol: u8;

    let mut x: u8;
    
    while index < file_data.len() {
        assert_eq!(Some(&[84, 80, 88, 51][..]), file_data.get(index..index+4));
        
        chip_index = file_data[index+4];
        total_size = (file_data[index+6] as u16) | (file_data[index+7] as u16)<<8;

        let _nloop = total_size / 8;
        for _i in 0.._nloop {
            /*
            _spidr = (file_data[index+8] as u16) | (file_data[index+9] as u16)<<8;
            _ftoa = file_data[index+10] & 15;
            _tot = ((file_data[index+10] & 240) as u16)>>4 | ((file_data[index+11] & 63) as u16)<<4;
            _toa = ((file_data[index+11] & 192) as u16)>>6 | (file_data[index+12] as u16)<<2 | ((file_data[13] & 15) as u16)<<10;
            */
            _pix = (file_data[index+13] & 112)>>4;
            //_spix = ((file_data[index+13] & 128) as u16)>>5 | ((file_data[index+14] & 31) as u16)<<3;
            _dcol = ((file_data[index+14] & 224))>>4 | ((file_data[index+15] & 15))<<4;

            x = _dcol | _pix >> 2;
            //_y = _spix | (_pix & 3);

            //let x = x as usize;
            let x = correct_x_position(chip_index, x);

            final_data[2*x]+=1;
            if final_data[2*x]==255 {
                final_data[(2*x+1)] += 1;
                final_data[2*x] = 0;
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
            let msg = create_header(0.352, counter, 2048, 16, 1024, 1);

            if has_data(counter)==false {
                counter = 0;
            }
            
            let (mydata, _) = open_and_read(counter);
            let my_real_data = build_data(&mydata);

            sock.write(&msg).expect("Header not sent.");
            sock.write(&my_real_data).expect("Data not sent.");
            let elapsed = start.elapsed();
            println!("{} and {:?}", counter, elapsed);
            counter+=1;
        }
    }
}

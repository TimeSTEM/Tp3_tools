use std::io::prelude::*;
use std::{fs, io};
use std::io::BufReader;
use std::net::TcpListener;
use std::time::{Duration, Instant};
use std::{thread, time};
use std::sync::mpsc;

enum RunningMode {
    debug_stem7482,
    debug_cheetah,
    tp3,
}

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
        let temp = ((((self.i14 & 224))>>4 | ((self.i15 & 15))<<4) | (((self.i13 & 112)>>4)>>2)) as usize;
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

fn build_data(data: &[u8], bin: bool, final_data: &mut [u8]) {
    
    let file_data = data;
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
                    i08: x[0],
                    i09: x[1],
                    i10: x[2],
                    i11: x[3],
                    i12: x[4],
                    i13: x[5],
                    i14: x[6],
                    i15: x[7],
                };
                
                if packet.id()==11 {
                    let array_pos = match bin {
                        false => 4*packet.x() + 4*1024*packet.y(),
                        true => 4*packet.x()
                    };
                    
                    final_data[array_pos+3] += 1;
                    if final_data[array_pos+3]==255 {
                        final_data[array_pos+3] = 0;
                        final_data[array_pos+2] += 1;
                        if final_data[array_pos+2]==255 {
                            final_data[array_pos+1] += 1;
                            final_data[array_pos+2] = 0;
                            if final_data[array_pos+1]==255 {
                                final_data[array_pos] += 1;
                                final_data[array_pos+1] = 0;
                            };
                        };
                        
                        
                    };
                
                };
            },
        };
    }
}

fn has_data(path: &str, number: usize) -> bool {
    let mut path = path.to_string();
    let ct: String = format!("{:06}", number);
    path.push_str(&ct);
    path.push_str(".tpx3");
    match fs::File::open(path) {
        Ok(_myfile) => true,
        Err(_) => false,
    }
}

fn remove_all(path: &str) -> io::Result<()> {
    //fs::remove_dir_all(path).expect("Could not remove directory.");
    //fs::create_dir(path);
    let mut entries = fs::read_dir(path)?
        .map(|res| res.map(|e| e.path()));
    
    for val in entries {
        let dir = val?;
        fs::remove_file(dir);
    }

    Ok(())

}

fn open_and_read(path: &str, number: usize, delete: bool) -> Vec<u8> {
    let mut path = path.to_string();
    let ct: String = format!("{:06}", number);
    path.push_str(&ct);
    path.push_str(".tpx3");
    
    let mut buffer: Vec<u8> = Vec::new();

    if let Ok(mut myfile) = fs::File::open(&path) {
        match myfile.read_to_end(&mut buffer) {
            Ok(_) => {
                if delete {
                    match fs::remove_file(&path) {
                        Ok(_) => {},
                        Err(_) => {},
                    };
                };
            },
            Err(error) => panic!("Problem opening the file {}. Error: {:?}", number, error),
        };
    };
    buffer
}

fn create_header(time: f64, frame: usize, data_size: u32, bitdepth: u8, width: u16, height: u16) -> Vec<u8> {
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

#[allow(dead_code)]
fn from_16_to_8(data: &[u16]) -> Vec<u8> {
    let vec1 = data;
    let mut new_vec: Vec<u8> = vec1.iter().map(|&x| x as u8).collect();
    let it1 = vec1.iter().enumerate().filter(|&(a, b)| b>&255);
    for (i, val) in it1 {
        new_vec[i+1] += (val / 255) as u8;
        new_vec[i] = (val % 255) as u8;
    }
    new_vec
}

#[allow(dead_code)]
fn double_from_16_to_8(data: &[u16], data2: &[u16]) -> Vec<u8> {
    let vec1 = data;
    let vec2 = data2;
    let mut new_vec: Vec<u8> = vec1.iter().zip(vec2.iter()).map(|(a, b)| (a+b) as u8).collect();
    let it1 = vec1.iter().zip(vec2.iter()).map(|(a, b)| a+b).enumerate().filter(|&(a, b)| b>255);
    for (i, val) in it1 {
        new_vec[i+1] += (val / 255) as u8;
        new_vec[i] = (val % 255) as u8;
    }
    new_vec
}

fn connect_and_loop(runmode: RunningMode) {

    let mut bin: bool = true;
    let deletefile: bool;

    let mut my_path: String = match runmode {
        RunningMode::debug_stem7482 => String::from("C:\\Users\\AUAD\\Documents\\wobbler_data\\raw"),
        RunningMode::debug_cheetah => String::from("/home/yves/Documents/wobbler_data/raw"),
        RunningMode::tp3 => String::from("/home/asi/load_files/data"),
    };

    match runmode {
        RunningMode::debug_stem7482 | RunningMode::debug_cheetah => {
            println!("Running locally. Won't delete folder neither file. Attempting to connect to localhost...");
            deletefile = false;
        },
        RunningMode::tp3 => {
            remove_all(&my_path).expect("Cannot remove all files");
            my_path.push_str("//raw");
            deletefile = true;
        },
    };

    let listener = match runmode {
        RunningMode::debug_stem7482 => TcpListener::bind("127.0.0.1:8088").unwrap(),
        RunningMode::tp3 | RunningMode::debug_cheetah=> TcpListener::bind("192.168.199.11:8088").unwrap(),
    };

    if let Ok((socket, addr)) = listener.accept() {
        println!("New client connected at {:?}", addr);
        let mut sock = socket;
        
        let mut my_data = [0 as u8; 4];
        if let Ok(_) = sock.read(&mut my_data){
            bin = match my_data[0] {
                0 => false,
                1 => true,
                _ => true, //panic!("Binning choice must be 0 | 1."),
            };
        };
        sock.set_read_timeout(Some(Duration::from_micros(100))).unwrap();
        println!("Received data is {:?}.", my_data);
        
        let mut counter = 0usize;
        let start = Instant::now();
        'global: loop {
            let msg = create_header(0.0, counter, 4*1024*(256-255*(bin as u32)), 32, 1024, 256 - 255*(bin as u16));

            while has_data(&my_path, counter+1)==false {
                if let Ok(size) = sock.read(&mut my_data) {
                    if size == 0 {
                        break 'global;
                    };
                };
            }
            
            let mydata = open_and_read(&my_path, counter, deletefile);
            let mut myarray:Vec<u8> = if bin {vec![0; 4*1024]} else {vec![0; 256*4*1024]};
            build_data(&mydata[..], bin, &mut myarray);
            myarray.push(10);
            
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

            match sock.write(&msg) {
                Ok(_) => {},
                Err(_) => {
                    println!("Client {} disconnected on header. Waiting a new one.", addr);
                    break;
                },
            };
            match sock.write(&myarray) {
                Ok(_) => {},
                Err(_) => {
                    println!("Client {} disconnected on data. Waiting a new one.", addr);
                    break;
                },
            };
            counter+=1;
            if counter % 100 == 0 {
                counter = 0;
                let elapsed = start.elapsed();
                println!("Total elapsed time is: {:?}", elapsed);
            }
        };
        println!("Number of loops were: {}.", counter);
    }
}

fn main() {
    loop {
        let myrun = RunningMode::debug_stem7482;
        //let myrun = RunningMode::tp3;
        println!{"Waiting for a new client"};
        connect_and_loop(myrun);
    }
}

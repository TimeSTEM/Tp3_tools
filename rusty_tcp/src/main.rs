use std::io::prelude::*;
use std::{fs, io};
use std::io::BufReader;
use std::net::{Shutdown, TcpListener, TcpStream};
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
    
    fn x_unmod(&self) -> usize {
        !((((self.i14 & 224))>>4 | ((self.i15 & 15))<<4) | (((self.i13 & 112)>>4)>>2)) as usize
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

    fn tdcCoarseT(&self) -> u64 {
        ((self.i09 & 254) as u64)>>1 | ((self.i10) as u64)<<7 | ((self.i11) as u64)<<15 | ((self.i12) as u64)<<23 | ((self.i13 & 15) as u64)<<31
    }
    
    fn tdcFineT(&self) -> u8 {
        (self.i08 & 224)>>5 | (self.i09 & 1)<<3
    }

    fn tdcT(coarse: u64, fine: u8) -> f64 {
        (coarse as f64) * (1.0/320e6) + (fine as f64) * 260e-12
    }

    fn append_to_array(data: &mut [u8], index:usize, bytedepth: usize) -> bool{
        match bytedepth {
            4 => {
                data[index+3] = data[index+3].wrapping_add(1);
                if data[index+3]==0 {
                    data[index+2] = data[index+2].wrapping_add(1);
                    if data[index+2]==0 {
                        data[index+1] = data[index+1].wrapping_add(1);
                        if data[index+1]==0 {
                            data[index] = data[index].wrapping_add(1);
                        };
                    };
                };
                false
            },
            2 => {
                data[index+1] = data[index+1].wrapping_add(1);
                if data[index+1]==0 {
                    data[index] = data[index].wrapping_add(1);
                    true
                } else {
                    false
                }
            },
            1 => {
                data[index] = data[index].wrapping_add(1);
                if data[index]==0 {
                    true
                } else {
                    false
                }
            },
            _ => {panic!("Bytedepth must be 1 | 2 | 4.");},
        }
    }
}

fn build_data_in_thread(data: &[u8], bin: bool, final_data: &mut [Vec<u8>; 4], last_ci: u8, remainder: &mut [Vec<u8>; 4], bytedepth: usize) -> (u8, bool, f64) {
    
    let file_data = data;
    let rem_data = remainder;
    let bin = bin;
    let mut packet_chunks = file_data.chunks_exact(8);
    let mut hasTdc: bool = false;
    let mut main_array:bool = true;
    let mut time = 0.0f64;
    
    let mut ci: u8 = last_ci;
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
                
                match packet.id() {
                    11 => {
                        let array_pos = match bin {
                            false => bytedepth*packet.x_unmod() + bytedepth*256*packet.y(),
                            true => bytedepth*packet.x_unmod()
                        };
                        match main_array {
                            true => {
                                Packet::append_to_array(&mut final_data[ci as usize], array_pos, bytedepth);
                            },
                            false => {
                                Packet::append_to_array(&mut rem_data[ci as usize], array_pos, bytedepth);
                            },
                        };
                    },
                    6 => {
                        time = Packet::tdcT(packet.tdcCoarseT(), packet.tdcFineT());
                        hasTdc = true;
                        main_array = false;
                    },
                    7 => {continue;},
                    4 => {continue;},
                    _ => {},
                };
            },
        };
    };
    (ci, hasTdc, time)

}

fn build_data(data: &[u8], bin: bool, final_data: &mut [u8], last_ci: u8, remainder: &mut [u8], bytedepth: usize) -> (u8, bool, f64) {
    
    let file_data = data;
    let rem_data = remainder;
    let bin = bin;
    let mut packet_chunks = file_data.chunks_exact(8);
    let mut hasTdc: bool = false;
    let mut main_array:bool = true;
    let mut time = 0.0f64;

    let mut ci: u8 = last_ci;
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
                
                match packet.id() {
                    11 => {
                        let array_pos = match bin {
                            false => bytedepth*packet.x() + bytedepth*1024*packet.y(),
                            true => bytedepth*packet.x()
                        };
                        match main_array {
                            true => {
                                Packet::append_to_array(final_data, array_pos, bytedepth);
                            },
                            false => {
                                Packet::append_to_array(rem_data, array_pos, bytedepth);
                            },
                        };
                    },
                    6 => {
                        time = Packet::tdcT(packet.tdcCoarseT(), packet.tdcFineT());
                        hasTdc = true;
                        main_array = false;
                    },
                    7 => {continue;},
                    4 => {continue;},
                    _ => {},
                };
            },
        };
    };
    (ci, hasTdc, time)
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

fn create_header(time: f64, frame: usize, data_size: usize, bitdepth: usize, width: usize, height: usize) -> Vec<u8> {
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

    let pack_listener = TcpListener::bind("127.0.0.1:8098").unwrap();
    
    let ns_listener = match runmode {
        RunningMode::debug_stem7482 => TcpListener::bind("127.0.0.1:8088").unwrap(),
        RunningMode::tp3 | RunningMode::debug_cheetah=> TcpListener::bind("192.168.199.11:8088").unwrap(),
    };

    if let Ok((packet_socket, packet_addr)) = pack_listener.accept() {
        println!("Localhost TP3 detected at {:?}", packet_addr);
        if let Ok((ns_socket, ns_addr)) = ns_listener.accept() {
            println!("Nionswift connected at {:?}", ns_addr);

            let mut pack_sock = packet_socket;
            let mut ns_sock = ns_socket;
            
            let mut my_data = [0 as u8; 4];
            if let Ok(_) = ns_sock.read(&mut my_data){
                bin = match my_data[0] {
                    0 => false,
                    1 => true,
                    _ => false, //panic!("Binning choice must be 0 | 1."),
                };
            };
            
            pack_sock.set_read_timeout(Some(Duration::from_micros(1_000))).unwrap();
            ns_sock.set_read_timeout(Some(Duration::from_micros(100))).unwrap();
            println!("Received data is {:?}.", my_data);
            
            let mut counter = 0usize;
            let mut last_ci = 0u8;
            let mut hasTdc: bool = false;
            let mut remain: usize = 0;
            let mut buffer_pack_data: [u8; 64000] = [0; 64000];
            let bytedepth = 1usize;
            let mut rem_array:Vec<u8> = if bin {vec![0; bytedepth*1024]} else {vec![0; bytedepth*256*1024]};
            
            let mut rem_array_ind: [Vec<u8>; 4] = if bin {[vec![0; 256], vec![0; 256], vec![0; 256], vec![0; 256]]} else {[vec![0;256*256], vec![0;256*256], vec![0;256*256], vec![0; 256*256]]};
            
            let start = Instant::now();
            'global: loop {
                
                let mut data_array:Vec<u8> = rem_array.clone();
                let mut rem_array:Vec<u8> = if bin {vec![0; bytedepth*1024]} else {vec![0; 256*bytedepth*1024]};
                
                let mut data_array_ind: [Vec<u8>; 4] = rem_array_ind.clone();
                let mut rem_array_ind: [Vec<u8>; 4] = if bin {[vec![0; 256], vec![0; 256], vec![0; 256], vec![0; 256]]} else {[vec![0;256*256], vec![0;256*256], vec![0;256*256], vec![0; 256*256]]};
                
                data_array.push(10);
                data_array_ind[3].push(10);
                
                loop {
                    if let Ok(size) = pack_sock.read(&mut buffer_pack_data) {
                        if size>0 {
                            let new_data = &buffer_pack_data[0..size];
                            let result = build_data(new_data, bin, &mut data_array, last_ci, &mut rem_array, bytedepth);
                            //let result = build_data_in_thread(new_data, bin, &mut data_array_ind, last_ci, &mut rem_array_ind, bytedepth);
                            last_ci = result.0;
                            hasTdc = result.1;
                            let frame_time = result.2;
                            let msg = create_header(frame_time, counter, bytedepth*1024*(256-255*(bin as usize)), bytedepth<<3, 1024, 256 - 255*(bin as usize));
                            if hasTdc==true {
                                counter+=1;

                                match ns_sock.write(&msg) {
                                    Ok(_) => {},
                                    Err(_) => {
                                        println!("Client {} disconnected on header. Waiting a new one.", ns_addr);
                                        break 'global;
                                    },
                                };
                                
                                match ns_sock.write(&data_array) {
                                    Ok(_) => {},
                                    Err(_) => {
                                        println!("Client {} disconnected on data. Waiting a new one.", ns_addr);
                                        break 'global;
                                    },
                                };

                                /*for i in 0..4 {
                                    match ns_sock.write(&data_array_ind[i]) {
                                        Ok(_) => {},
                                        Err(_) => {
                                            println!("Client {} disconnected on data. Waiting a new one.", ns_addr);
                                            break 'global;
                                        },
                                    };
                                };
                                */
                                //println!("Tdc on");
                                break;
                            } 
                        }
                        else {
                            println!("Received zero packages");
                            break 'global
                        }
                    }
                }
                
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

                
                if counter % 100 == 0 {
                    let elapsed = start.elapsed();
                    println!("Total elapsed time is: {:?}", elapsed);
                }
            }
            println!("Number of loops were: {}.", counter);
            ns_sock.shutdown(Shutdown::Both).expect("Shutdown call failed");
        }
    }
}

fn main() {
    loop {
        let myrun = RunningMode::debug_stem7482;
        println!{"Waiting for a new client"};
        connect_and_loop(myrun);
    }
}



/*
 *
 *
 * THIS IS OLD CODE WHEN READING FROM FILE
 *
 *
 *
let mut my_path: String = match runmode {
    RunningMode::debug_stem7482 => String::from("C:\\Users\\AUAD\\Documents\\wobbler_data\\raw"),
    RunningMode::debug_cheetah => String::from("/home/asi/load_files/wobbler_data/raw"),
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

while has_data(&my_path, counter+1)==false {
    if let Ok(size) = ns_sock.read(&mut my_data) {
        if size == 0 {
            break 'global;
        };
    };
};

let mydata = open_and_read(&my_path, counter, deletefile);
let mut myarray:Vec<u8> = if bin {vec![0; 4*1024]} else {vec![0; 256*4*1024]};
let (last_ci, _, _) = build_data(&mydata[..], bin, &mut myarray, last_ci);
myarray.push(10);


match ns_sock.write(&msg) {
    Ok(_) => {},
    Err(_) => {
        println!("Client {} disconnected on header. Waiting a new one.", ns_addr);
        break;
    },
};
match ns_sock.write(&myarray) {
    Ok(_) => {},
    Err(_) => {
        println!("Client {} disconnected on data. Waiting a new one.", ns_addr);
        break;
    },
};

*
* 
* 
*/

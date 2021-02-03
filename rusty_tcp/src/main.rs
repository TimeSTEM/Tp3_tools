use std::io;
use std::io::prelude::*;
use std::fs::File;
use std::net::{TcpListener, TcpStream};

fn main() -> io::Result<()>{

    let listener = TcpListener::bind("127.0.0.1:8088").unwrap();

    match listener.accept() {
        Ok((socket, addr)) => {
            println!("new client {:?}", addr);
            let mut sock = socket;

            let mut my_data = [0 as u8; 512];
            
            match sock.read(&mut my_data) {
                Ok(size) => {
                    sock.write(&my_data[0..size]);
                },
                Err(e) => {
                    println!("failed");
                }
            }
        }
        Err(e) => println!("failed {}", e),
    }
    

    let mut f = File::open("C:\\Users\\AUAD\\Documents\\Tp3_tools\\TCPFiletoStreamProcessed\\Files_00\\raw000000.tpx3")?;
    
    let mut buffer: Vec<u8> = Vec::new();

    f.read_to_end(&mut buffer)?;
    println!("Buffer size is {}", buffer.len());

    let mut data = vec![0; 1024];
    
    let mut index = 0;
    let mut total_size: u16;
    let mut _chip_index: u8;
    let mut _spidr: u16;
    let mut _ftoa: u8;
    let mut _tot: u16;
    let mut _toa: u16;
    let mut _pix: u16;
    let mut _spix: u16;
    let mut _dcol: u16;

    let mut x: u16;
    let mut _y: u16;

    let mut counter = 0;

    while index < buffer.len() {
        assert_eq!(Some(&[84, 80, 88, 51][..]), buffer.get(index..index+4));
        
        _chip_index = buffer[index+4];
        total_size = (buffer[index+6] as u16) | (buffer[index+7] as u16)<<8;

        let _nloop = total_size / 8;
        for _i in 0.._nloop {
            _spidr = (buffer[index+8] as u16) | (buffer[index+9] as u16)<<8;
            _ftoa = buffer[index+10] & 15;
            _tot = ((buffer[index+10] & 240) as u16)>>4 | ((buffer[index+11] & 63) as u16)<<4;
            _toa = ((buffer[index+11] & 192) as u16)>>6 | (buffer[index+12] as u16)<<2 | ((buffer[13] & 15) as u16)<<10;
            _pix = (buffer[index+13] & 112) as u16>>4;
            _spix = ((buffer[index+13] & 128) as u16)>>5 | ((buffer[index+14] & 31) as u16)<<3;
            _dcol = ((buffer[index+14] & 224) as u16)>>4 | ((buffer[index+15] & 15) as u16)<<4;

            x = _dcol | _pix >> 2;
            _y = _spix | (_pix & 3);

            data[x as usize]+=1;
            counter+=1;

            //println!("{} and {} and {}", x, y, _spidr);
            //println!("{} and {}", &buffer[index+15], buffer[index+15]);
            index = index + 8;
        }
        index = index + 8;
    }

    println!("{}", counter);
    
    Ok(())



    //let mut reader = BufReader::new(f);
    //let mut buffer = String::new();

    //for line in reader.lines() {
    //    println!("{}", line.unwrap());
    //}

    //reader.read_line(&mut buffer)?;

    //println!("{:?}", buffer);


    //let metadata = fs::metadata("C:\\Users\\AUAD\\Documents\\Tp3_tools\\TCPFiletoStreamProcessed\\Files_00\\raw000000.tpx3");
    //println!("vector length is: {:?}", metadata.unwrap().len());

    /*
     * for i in &filedata{
     * let i: &i32 = i;
     * println!("{}", i);
     }
     */
        

}

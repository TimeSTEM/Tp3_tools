//!`spimlib` is a collection of tools to set hyperspectral EELS acquisition.

use crate::packetlib::Packet;
use crate::auxiliar::{Settings, misc::{TimepixRead, as_bytes, as_bytes_mut, packet_change}, FileManager};
use crate::tdclib::{TdcRef, isi_box::{IsiBoxHand, IsiBoxType}};
use crate::errorlib::Tp3ErrorKind;
use std::time::Instant;
use std::io::Write;
use std::sync::mpsc;
use std::thread;
use crate::auxiliar::value_types::*;
use crate::constlib::*;

///`SpimKind` is the main trait that measurement types must obey. Custom measurements must all
///implement these methods.
pub trait SpimKind {
    type InputData;
    
    fn data(&self) -> &Vec<Self::InputData>;
    fn build_main_tdc<V: TimepixRead>(&mut self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind>;
    fn build_aux_tdc<V: TimepixRead>(&self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind>;
    fn add_electron_hit(&mut self, packet: &Packet, line_tdc: &TdcRef, ref_tdc: &TdcRef, set: &Settings);
    fn add_tdc_hit(&mut self, packet: &Packet, line_tdc: &TdcRef, ref_tdc: &mut TdcRef);
    fn upt_line(&self, packet: &Packet, settings: &Settings, line_tdc: &mut TdcRef);
    fn build_output(&mut self, set: &Settings, spim_tdc: &TdcRef, list_scan: SlType) -> &[u8];
    fn copy_empty(&mut self) -> Self;
    fn is_ready(&mut self, line_tdc: &TdcRef) -> bool;
    fn new(settings: &Settings) -> Self;
}

#[inline]
pub fn get_spimindex(x: POSITION, dt: TIME, spim_tdc: &TdcRef, xspim: POSITION, yspim: POSITION, list_scan: SlType) -> Option<INDEXHYPERSPEC> {
    Some(spim_tdc.get_positional_index(dt, xspim, yspim, list_scan)? * PIXELS_X + x)
}

#[inline]
pub fn get_return_spimindex(x: POSITION, dt: TIME, spim_tdc: &TdcRef, xspim: POSITION, yspim: POSITION, list_scan: SlType) -> Option<INDEXHYPERSPEC> {
    Some(spim_tdc.get_return_positional_index(dt, xspim, yspim, list_scan)? * PIXELS_X + x)
}

#[inline]
pub fn get_4dindex(x: POSITION, y: POSITION, dt: TIME, spim_tdc: &TdcRef, xspim: POSITION, yspim: POSITION, list_scan: SlType) -> Option<INDEX4D> {
    Some(spim_tdc.get_positional_index(dt, xspim, yspim, list_scan)? as INDEX4D * (PIXELS_X * PIXELS_Y) as INDEX4D + (y * PIXELS_X + x)as INDEX4D)
}

#[inline]
pub fn get_return_4dindex(x: POSITION, y: POSITION, dt: TIME, spim_tdc: &TdcRef, xspim: POSITION, yspim: POSITION, list_scan: SlType) -> Option<INDEX4D> {
    Some(spim_tdc.get_return_positional_index(dt, xspim, yspim, list_scan)? as INDEX4D * (PIXELS_X * PIXELS_Y) as INDEX4D + (y * PIXELS_X + x) as INDEX4D)
}

#[inline]
pub fn get_coincidence_spimindex(x: POSITION, dt: TIME, spim_tdc: &TdcRef, xspim: POSITION, yspim: POSITION, list_scan: SlType, my_settings: &Settings) -> Option<INDEX4D> {
    Some(spim_tdc.get_positional_index(dt, xspim, yspim, list_scan)? as INDEX4D * (PIXELS_X as INDEX4D * my_settings.time_width * 2) + x as INDEX4D)
}

///It outputs list of indices (max `u32`) that
///must be incremented. This is Hyperspectral Imaging
pub struct Live {
    data: Vec<(POSITION, TIME)>,
    data_out: Vec<INDEXHYPERSPEC>,
    _timer: Instant,
}

///It outputs a list of indices that must
///be increment. This is Hyperspectral imaging with
///coincident photons around
pub struct LiveCoincidence {
    data: Vec<(POSITION, TIME)>,
    aux_data: Vec<TIME>,
    data_out: Vec<INDEX4D>,
    _timer: Instant,
}

///It outputs a list of indices that must
///be increment. This is Hyperspectral imaging with
///coincident photons around
pub struct Live4D {
    data: Vec<(POSITION, TIME)>,
    data_out: Vec<INDEX4D>,
    _timer: Instant,
}

///It outputs a frame-based to be used to reconstruct a channel in 4D STEM.
pub struct LiveFrame4D<T> {
    data: Vec<(POSITION, TIME)>,
    data_out: Vec<T>,
    com: (T, T),
    scan_size: (POSITION, POSITION),
    channels: Vec<Live4DChannelMask<T>>,
    debouncer: bool,
    timer: Instant,
}

//Only method of this struct. Must be called to initialize the data_out array, as it is not
//a list-based output
impl LiveFrame4D<MaskValues> {
    fn create_data_channels(&mut self) {
        self.data_out = vec![0; (self.scan_size.0 * self.scan_size.1) as usize * self.number_of_masks() as usize];
    }
    fn number_of_masks(&self) -> u8 {
        //self.channels.len() as u8
        4_u8
    }
    fn create_mask<R: std::io::Read>(&mut self, array: R) -> Result<(), Tp3ErrorKind> {
        //Ok(self.channels.push(grab_mask(array)?))
        self.channels = Live4DChannelMask::grab_mask(array)?;
        Ok(())
    }
}

//T is mask data type
struct Live4DChannelMask<T> {
    mask: [T; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize],
}

impl Live4DChannelMask<MaskValues> {
    fn new(array: [MaskValues; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize]) -> Self {
        Self {mask: array}
    }
    fn grab_mask<R: std::io::Read>(mut array: R) -> Result<Vec<Self>, Tp3ErrorKind> {
        let mut vec_of_channels: Vec<Self> = Vec::new();
        let mut mask = [0; (DETECTOR_SIZE.0 * DETECTOR_SIZE.1) as usize];
        while array.read_exact(as_bytes_mut(&mut mask)).is_ok() {
            vec_of_channels.push(Live4DChannelMask::new(mask));
        }
        Ok(vec_of_channels)
    }
}

impl SpimKind for Live {
    type InputData = (POSITION, TIME);

    fn data(&self) -> &Vec<Self::InputData> {
        &self.data
    }
    #[inline]
    fn add_electron_hit(&mut self, packet: &Packet, line_tdc: &TdcRef, ref_tdc: &TdcRef, set: &Settings) {
        let ele_time = line_tdc.sync_electron_frame_time(packet).unwrap();
        if set.time_resolved {
            if ref_tdc.tr_check_if_in(packet.electron_time(), set).is_some() {
                self.data.push((packet.x(), ele_time)); //This added the overflow.
            } 
        } else {
            self.data.push((packet.x(), ele_time)); //This added the overflow.
        }
    }
    fn build_main_tdc<V: TimepixRead>(&mut self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_periodic(MAIN_TDC, pack, my_settings, file_to_write)
    }
    fn build_aux_tdc<V: TimepixRead>(&self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        if my_settings.time_resolved {
            TdcRef::new_periodic(SECONDARY_TDC, pack, my_settings, file_to_write)
        } else {
            TdcRef::new_no_read(SECONDARY_TDC)
        }
    }
    fn add_tdc_hit(&mut self, packet: &Packet, line_tdc: &TdcRef, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(packet);
        let tdc_time = line_tdc.sync_tdc_frame_time(packet).unwrap();
        self.data.push((PIXELS_X-1, tdc_time));
    }
    fn upt_line(&self, packet: &Packet, _settings: &Settings, line_tdc: &mut TdcRef) {
        line_tdc.upt(packet);
    }
    #[inline]
    fn build_output(&mut self, set: &Settings, spim_tdc: &TdcRef, list_scan: SlType) -> &[u8] {

        //First step is to find the index of the (X, Y) of the spectral image in a flattened way
        //(last index is X*Y). The line value is thus multiplied by the spim size in the X
        //direction. The column must be between [0, X]. So we have, for the position:
        //
        //index = line * xspim + column
        //
        //To find the actuall index value, one multiply this value by the number of signal pixels
        //(the spectra) because every spatial point has SPIM_PIXELS channels.
        //
        //index = index * SPIM_PIXELS
        //
        //With this, we place every electron in the first channel of the signal dimension. We must
        //thus add the pixel address to correct reconstruct the spectral image
        //
        //index = index + x
        
        
        self.data_out = self.data.iter()
            .filter_map(|&(x, dt)| {
                get_spimindex(x, dt, spim_tdc, set.xspim_size, set.yspim_size, list_scan)
            }).collect::<Vec<POSITION>>();

        /*
        let temp = &mut self.data_out;
        let my_vec = self.data.iter()
            .filter_map(|&(x, dt)| {
                get_spimindex(x, dt, spim_tdc, set.xspim_size, set.yspim_size)
            }).for_each(|x| temp.push(x));
        */
        
        as_bytes(&self.data_out)
    }
    fn is_ready(&mut self, _line_tdc: &TdcRef) -> bool {
        true
    }
    fn copy_empty(&mut self) -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8) , data_out: Vec::new(), _timer: Instant::now()}
    }
    fn new(_settings: &Settings) -> Self {
        Live{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), _timer: Instant::now()}
    }
}

impl SpimKind for LiveCoincidence {
    type InputData = (POSITION, TIME);

    fn data(&self) -> &Vec<Self::InputData> {
        &self.data
    }
    #[inline]
    fn add_electron_hit(&mut self, packet: &Packet, line_tdc: &TdcRef, _ref_tdc: &TdcRef, set: &Settings) {
        let ele_time = packet.electron_time();
        for phtime in self.aux_data.iter() {
            if (*phtime < ele_time + set.time_delay + set.time_width) && (ele_time + set.time_delay < *phtime + set.time_width) {
                let delay = (phtime - set.time_delay + set.time_width - ele_time) as POSITION;
                let ele_time_corr = line_tdc.sync_electron_frame_time(packet).unwrap();
                let index = packet.x() + delay * PIXELS_X;
                self.data.push((index, ele_time_corr)); //This added the overflow.
            }
        }
    }
    fn build_main_tdc<V: TimepixRead>(&mut self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_periodic(MAIN_TDC, pack, my_settings, file_to_write)
    }
    fn build_aux_tdc<V: TimepixRead>(&self, _pack: &mut V, _my_settings: &Settings, _file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_no_read(SECONDARY_TDC)
    }
    fn add_tdc_hit(&mut self, packet: &Packet, _line_tdc: &TdcRef, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(packet);
        self.aux_data.push(packet.tdc_time_norm());
        self.aux_data.remove(0);
    }
    fn upt_line(&self, packet: &Packet, _settings: &Settings, line_tdc: &mut TdcRef) {
        line_tdc.upt(packet);
    }
    #[inline]
    fn build_output(&mut self, set: &Settings, spim_tdc: &TdcRef, list_scan: SlType) -> &[u8] {

        self.data_out = self.data.iter()
            .filter_map(|&(x, dt)| {
                get_coincidence_spimindex(x, dt, spim_tdc, set.xspim_size, set.yspim_size, list_scan, set)
            }).collect::<Vec<INDEX4D>>();
        
        as_bytes(&self.data_out)
    }
    fn is_ready(&mut self, _line_tdc: &TdcRef) -> bool {
        true
    }
    fn copy_empty(&mut self) -> Self {
        LiveCoincidence{ data: Vec::with_capacity(BUFFER_SIZE / 8) , aux_data: vec![0; LIST_SIZE_AUX_EVENTS], data_out: Vec::new(), _timer: Instant::now()}
    }
    fn new(_settings: &Settings) -> Self {
        LiveCoincidence{ data: Vec::with_capacity(BUFFER_SIZE / 8), aux_data: vec![0; LIST_SIZE_AUX_EVENTS], data_out: Vec::new(), _timer: Instant::now()}
    }
}

impl SpimKind for Live4D {
    type InputData = (POSITION, TIME);

    fn data(&self) -> &Vec<Self::InputData> {
        &self.data
    }
    #[inline]
    fn add_electron_hit(&mut self, packet: &Packet, line_tdc: &TdcRef, ref_tdc: &TdcRef, set: &Settings) {
        let ele_time = line_tdc.sync_electron_frame_time(packet).unwrap();
        if set.time_resolved {
            if ref_tdc.tr_check_if_in(packet.electron_time(), set).is_some() {
                self.data.push(((packet.x() << 16) + (packet.y() & 65535), ele_time)); //This added the overflow.
            } 
        } else {
            self.data.push(((packet.x() << 16) + (packet.y() & 65535), ele_time)); //This added the overflow.
        }
    }
    fn build_main_tdc<V: TimepixRead>(&mut self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_periodic(MAIN_TDC, pack, my_settings, file_to_write)
    }
    fn build_aux_tdc<V: TimepixRead>(&self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        if my_settings.time_resolved {
            TdcRef::new_periodic(SECONDARY_TDC, pack, my_settings, file_to_write)
        } else {
            TdcRef::new_no_read(SECONDARY_TDC)
        }
    }
    fn add_tdc_hit(&mut self, packet: &Packet, _line_tdc: &TdcRef, ref_tdc: &mut TdcRef) {
        ref_tdc.upt(packet);
    }
    fn upt_line(&self, packet: &Packet, _settings: &Settings, line_tdc: &mut TdcRef) {
        line_tdc.upt(packet);
    }
    #[inline]
    fn build_output(&mut self, set: &Settings, spim_tdc: &TdcRef, list_scan: SlType) -> &[u8] {

        self.data_out = self.data.iter()
            .filter_map(|&(x_y, dt)| {
                get_4dindex(x_y >> 16, x_y & 65535, dt, spim_tdc, set.xspim_size, set.yspim_size, list_scan)
            }).collect::<Vec<INDEX4D>>();
        
        as_bytes(&self.data_out)
    }
    fn is_ready(&mut self, _line_tdc: &TdcRef) -> bool {
        true
    }
    fn copy_empty(&mut self) -> Self {
        Live4D{ data: Vec::with_capacity(BUFFER_SIZE / 8) , data_out: Vec::new(), _timer: Instant::now()}
    }
    fn new(_settings: &Settings) -> Self {
        Live4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), _timer: Instant::now()}
    }
}

impl SpimKind for LiveFrame4D<MaskValues> {
    type InputData = (POSITION, TIME);

    fn data(&self) -> &Vec<Self::InputData> {
        &self.data
    }
    #[inline]
    fn add_electron_hit(&mut self, packet: &Packet, line_tdc: &TdcRef, _ref_tdc: &TdcRef, _set: &Settings) {
        let ele_time = line_tdc.sync_electron_frame_time(packet);
        self.data.push(((packet.x() << 16) + (packet.y() & 65535), ele_time.unwrap())); //This added the overflow.

    }
    fn build_main_tdc<V: TimepixRead>(&mut self, pack: &mut V, my_settings: &Settings, file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_periodic(MAIN_TDC, pack, my_settings, file_to_write)
    }
    fn build_aux_tdc<V: TimepixRead>(&self, _pack: &mut V, _my_settings: &Settings, _file_to_write: &mut FileManager) -> Result<TdcRef, Tp3ErrorKind> {
        TdcRef::new_no_read(SECONDARY_TDC)
    }
    fn add_tdc_hit(&mut self, _packet: &Packet, _line_tdc: &TdcRef, _ref_tdc: &mut TdcRef) {
    }
    fn upt_line(&self, packet: &Packet, _settings: &Settings, line_tdc: &mut TdcRef) {
        line_tdc.upt(packet);
    }
    #[inline]
    fn build_output(&mut self, set: &Settings, spim_tdc: &TdcRef, _list_scan: SlType) -> &[u8] {

        let channel_array_index = |x: POSITION, y: POSITION| -> usize
        {
            (y * DETECTOR_SIZE.0 + (x - DETECTOR_LIMITS.0.0)) as usize
        };
        let is_inside = |x: POSITION, y: POSITION| -> bool {
            (x > DETECTOR_LIMITS.0.0) && (x < DETECTOR_LIMITS.0.1) && (y > DETECTOR_LIMITS.1.0) && (y < DETECTOR_LIMITS.1.1)
        };

        let mut frequency = vec![0i16; (set.xspim_size * set.yspim_size) as usize];
        let number_of_masks = self.number_of_masks() as POSITION;
        let com = self.com;
        let temp = &mut self.data_out;
        let temp2 = &self.channels;


        self.data
            .iter()
            .filter_map(|&(x_y, dt)| {
                spim_tdc.get_positional_index(dt, set.xspim_size, set.yspim_size, None)
                    .map(|index| (x_y >> 16, x_y & 65535, index))
            })
            .filter(|&(x, y, _index)| is_inside(x, y))
            .for_each(|(x, y, index)| {
                frequency[index as usize] += 1;
                temp[(index * number_of_masks) as usize + 0] += x as i16 - com.0;
                temp[(index * number_of_masks) as usize + 1] += y as i16 - com.1;
                for (channel_number, channel) in temp2.iter().enumerate() {
                    let value = channel.mask[channel_array_index(x, y)];
                    temp[(index * number_of_masks) as usize + channel_number+2] += value;
                }
                  
            });
        
        self.data_out
            .chunks_exact_mut(number_of_masks as usize)
            .zip(frequency.iter())
            .for_each(|(chunk, frequency)| {
                if *frequency > 0 {
                    chunk[0] /= frequency;
                    chunk[1] /= frequency;
                }
            });

        as_bytes(&self.data_out)
    }
    #[inline]
    fn is_ready(&mut self, line_tdc: &TdcRef) -> bool {
        let is_new_frame = line_tdc.new_frame();
        if is_new_frame {
            if self.debouncer { 
                self.debouncer = false;
                if self.timer.elapsed().as_millis() < TIME_INTERVAL_4DFRAMES {
                    self.data.clear();
                    return false
                }
                return true
            }
        } else {
            self.debouncer = true;
        }
        false
    }
    fn copy_empty(&mut self) -> Self {
        let mut frame = LiveFrame4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: vec![0; (self.scan_size.0 * self.scan_size.1) as usize * self.number_of_masks() as usize], com: self.com, scan_size: self.scan_size, channels: Vec::new(), debouncer: false, timer: Instant::now()};
        let file = std::fs::File::open(MASK_FILE).unwrap();
        frame.create_mask(file).unwrap();
        frame.create_data_channels();
        frame
    }
    fn new(settings: &Settings) -> Self {
        let mut frame = LiveFrame4D{ data: Vec::with_capacity(BUFFER_SIZE / 8), data_out: Vec::new(), com: (0, 0), scan_size: (settings.xspim_size, settings.yspim_size), channels: Vec::new(), debouncer: false, timer: Instant::now()};
        let file = std::fs::File::open(MASK_FILE).unwrap();
        frame.create_mask(file).unwrap();
        frame.create_data_channels();
        frame
    }
}

///Reads timepix3 socket and writes in the output socket a list of frequency followed by a list of unique indexes. First TDC must be a periodic reference, while the second can be nothing, periodic tdc or a non periodic tdc.
pub fn build_spim<V, W, U>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut line_tdc: TdcRef, mut ref_tdc: TdcRef, mut meas_type: W, scan_list: SlType, mut file_to_write: FileManager) -> Result<(), Tp3ErrorKind>
    where V: 'static + Send + TimepixRead,
          W: 'static + Send + SpimKind,
          U: 'static + Send + Write,
{
    let (tx, rx) = mpsc::channel();
    let mut last_ci = 0;
    let mut buffer_pack_data = [0; BUFFER_SIZE];

    thread::spawn(move || {
        while let Ok(size) = pack_sock.read_timepix(&mut buffer_pack_data) {
            file_to_write.write_all(&buffer_pack_data[0..size]).expect("Could not save data into file.");
            build_spim_data(&mut meas_type, &buffer_pack_data[0..size], &mut last_ci, &my_settings, &mut line_tdc, &mut ref_tdc);
            if meas_type.is_ready(&line_tdc) {
               let list2 = meas_type.copy_empty();
               if tx.send(meas_type).is_err() {println!("Cannot send data over the thread channel."); break;}
               meas_type = list2;
            }
        }
    });
 
    let start = Instant::now();
    for mut tl in rx {
        let result = tl.build_output(&my_settings, &line_tdc, scan_list);
        if ns_sock.write(result).is_err() {println!("Client disconnected on data."); break;}
    }

    let elapsed = start.elapsed(); 
    println!("Total elapsed time is: {:?}.", elapsed);
    Ok(())
}

pub fn build_spim_isi<V, W, U>(mut pack_sock: V, mut ns_sock: U, my_settings: Settings, mut line_tdc: TdcRef, mut ref_tdc: TdcRef, mut meas_type: W, mut handler: IsiBoxType<Vec<u32>>) -> Result<(), Tp3ErrorKind>
    where V: 'static + Send + TimepixRead,
          W: 'static + Send + SpimKind,
          U: 'static + Send + Write,
{
    let (tx, rx) = mpsc::channel();
    let mut last_ci = 0;
    let mut buffer_pack_data = [0; BUFFER_SIZE];
    let mut list = meas_type.copy_empty();

    
    thread::spawn(move || {
        while let Ok(size) = pack_sock.read_timepix(&mut buffer_pack_data) {
            build_spim_data(&mut list, &buffer_pack_data[0..size], &mut last_ci, &my_settings, &mut line_tdc, &mut ref_tdc);
            if tx.send(list).is_err() {println!("Cannot send data over the thread channel."); break;}
            list = meas_type.copy_empty();
        }
    });
 
    let start = Instant::now();
    for mut tl in rx {
        let result = tl.build_output(&my_settings, &line_tdc, None);
        let x = handler.get_data();
        if ns_sock.write(as_bytes(result)).is_err() {println!("Client disconnected on data."); break;}
        if x.len() > 0 && ns_sock.write(as_bytes(&x)).is_err() {println!("Client disconnected on data."); break;}
    }

    handler.stop_threads();
    let elapsed = start.elapsed(); 
    println!("Total elapsed time is: {:?}.", elapsed);
    Ok(())
}

fn build_spim_data<W: SpimKind>(list: &mut W, data: &[u8], last_ci: &mut u8, settings: &Settings, line_tdc: &mut TdcRef, ref_tdc: &mut TdcRef) {
    data.chunks_exact(8).for_each(|x| {
        match *x {
            [84, 80, 88, 51, nci, _, _, _] => *last_ci = nci,
            _ => {
                let packet = Packet::new(*last_ci, packet_change(x)[0]);
                let id = packet.id();
                match id {
                    11 => {
                        list.add_electron_hit(&packet, line_tdc, ref_tdc, settings);
                    },
                    6 if packet.tdc_type() == line_tdc.id() => {
                        list.upt_line(&packet, settings, line_tdc);
                    },
                    6 if packet.tdc_type() == ref_tdc.id()=> {
                        list.add_tdc_hit(&packet, line_tdc, ref_tdc);
                    },
                    _ => {},
                };
            },
        };
    });
}

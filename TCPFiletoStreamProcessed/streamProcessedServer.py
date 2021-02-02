import socket
import os
import numpy
import time
import pickle
import sys

def create_tdc(Tdif, trigger='tdc1Ris'):
    
    #Message Header
    data = b'TPX3'
    data+=b'\x03' #Chip index
    data+=b'\x00' #Mode
    data+=b'\x08' #Number of pixels in chunk [LSB]
    data+=b'\x00' #Number of pixels in chunk [MSB]

    end = '0110' #0xb

    if trigger=='tdc1Ris': #tdc1 Rising Edge
        triggerType = '1111'
    elif trigger=='tdc1Fal': #tdc1 Falling Edge
        triggerType = '1010'
    elif trigger=='tdc2Ris': #tdc2 Rising Edge
        triggerType = '1110'
    elif trigger=='tdc2Fal': #tdc2 Falling Edge
        triggerType = '1011'

    timeDif = Tdif - int(Tdif/107374182396)*107374182396 #12 bits. Max time is ~107.37s
    
    triggerCounter = '000000000000' #12 bits.
    TimeStamp = bin(int(timeDif/1e9*320e6))[2:].zfill(35) #35 bits
    RFine = int(numpy.random.rand()*15) # Random fine value
    Fine = bin(RFine)[2:].zfill(4) #4 bits
    Reserved = '00000' #5 bits
    
    msg = int(end+triggerType+triggerCounter+TimeStamp+Fine+Reserved, 2) #64 bits = 8 bytes
    hex_msg = hex(msg)
    hex_msg=hex_msg[2:]
    data2 = bytes.fromhex(hex_msg)
    return data+data2[::-1] #Second part is inversed because it is easier to read.



"""
Set Script Parameters Here
Options for server are:
    - 129.175.81.162 for my PC;
    - 129.175.108.58 for Patrick's;
    - 192.0.0.11 in my old dell computer (Ubuntu);
    - 192.168.199.11 in CheeTah's computer (Ubuntu);
"""
FOLDER = 'Files_00'
SERVER_HOST = '127.0.0.1' #127.0.0.1 is LOCALHOST. Not visible in the network.
#SERVER_HOST = '192.0.0.11' #When not using in localhost
SERVER_PORT = 65431 #Pick a port to connect your socket
INFINITE_SERVER = False #This hangs for a new client after a client has been disconnected.
CREATE_TDC = False #if you wanna to add a tdc after the end of each read frame
TIME_INTERVAL = 0.000 #If no sleep, streaming is too fast
MAX_LOOPS = 0 #Max number of loops

"""
Script starts here
"""
serv = socket.socket(socket.AF_INET, socket.SOCK_STREAM) #AF_INET is IPV4. It expects a tuple (HOST, PORT). SOCK_STREAM is TCP protocol
serv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1) #To reuse the socket.
serv.bind((SERVER_HOST, SERVER_PORT))
serv.listen()

isRunning = True

def open_and_read(filepath, number):
    with open(filepath, "rb") as f:
        data = f.read()
        if CREATE_TDC: data += create_tdc(int(TIME_INTERVAL*loop*1e9))
    return data

def check_data(data):
    electron_data = b''
    tdc_data = b''
    nbytes = len(data)
    assert nbytes % 8 == 0
    index = 0
    while index<nbytes:
        packet = data[index:index+8]
        packet = packet[::-1]
        tpx3_header = packet[4:8]
        assert tpx3_header == b'3XPT'
        chip_index = packet[3]
        size1 = packet[1]
        size2 = packet[2]
        total_size = size1 + size2 * 256
        for j in range(int(total_size/8)):
            index+=8
            packet = data[index:index+8]
            packet = packet[::-1]
            packet_id = (packet[0] & 240) >> 4
            if packet_id == 11:
                electron_data+=packet + bytes([chip_index])
            elif packet_id == 6:
                tdc_data += packet + bytes([chip_index])
            else:
                pass
        index+=8
    return electron_data, tdc_data


def data_from_raw_electron(data, softBinning=True, toa=False, TimeDelay=None, TimeWidth=None):
    total_size = len(data)

    pos = list()
    gt = list()

    try:
        assert not total_size % 9 and bool(total_size)
    except AssertionError:
        return (pos, gt)

    def append_position(chip_index, data, softBinning):
        y = 0
        dcol = ((data[0] & 15) << 4) + ((data[1] & 224) >> 4)
        pix = (data[2] & 112) >> 4
        x = int(dcol + pix / 4)
        if not softBinning:
            spix = ((data[1] & 31) << 3) + ((data[2] & 128) >> 5)
            y = int(spix + (pix & 3))

        if chip_index == 0:
            x = 255 - x
            y = y
        elif chip_index == 1:
            x = 255 * 4 - x
            y = y
        elif chip_index == 2:
            x = 255 * 3 - x
            y = y
        elif chip_index == 3:
            x = 255 * 2 - x
            y = y

        pos.append([x, y])

    def get_time(data):
        toa = ((data[2] & 15) << 10) + ((data[3] & 255) << 2) + ((data[4] & 192) >> 6)
        ftoa = (data[5] & 15)
        spidr = ((data[6] & 255) << 8) + (data[7] & 255)
        ctoa = toa << 4 | ~ftoa & 15
        spidrT = spidr * 25.0 * 16384.0
        toa_ns = toa * 25.0
        return spidrT + ctoa * 25.0 / 16.0

    if toa:
        t0 = get_time(data[0:8])
        for i in range(int(total_size / 9)):
            ci = data[8 + i*9]  # Chip Index
            time = get_time(data[i*9:8+i*9])
            if TimeDelay <= (time - t0) <= TimeDelay + TimeWidth:
                append_position(ci, data[i*9:8+i*9], softBinning=softBinning)
            gt.append(time/1e9)
    else:
        for i in range(int(total_size / 9)):
            ci = data[8 + i*9] #Chip Index
            append_position(ci, data[i*9:8+i*9], softBinning=softBinning)
            gt.append(0)

    #print(f'{gt[0]} and {gt[-1]} and {TimeWidth}')
    return (pos, gt)

def data_from_raw_tdc(data):
    """
    Notes
    -----
    Trigger type can return 15 if tdc1 Rising edge; 10 if tdc1 Falling Edge; 14 if tdc2 Rising Edge;
    11 if tdc2 Falling edge. tdcT returns time in seconds up to ~107s
    """
    assert not len(data)%9

    coarseT = ((data[2] & 15) << 31) + ((data[3] & 255) << 23) + ((data[4] & 255) << 15) + (
                (data[5] & 255) << 7) + ((data[6] & 254) >> 1)
    fineT = ((data[6] & 1) << 3) + ((data[7] & 224) >> 5)
    tdcT = coarseT * (1 / 320e6) + fineT * 260e-12

    triggerType = data[0] & 15
    a = tdcT - int(tdcT / 26.8435456) * 26.8435456
    return (tdcT, triggerType, a)

def create_image_from_events(data):
    imagedata = numpy.zeros((1, 1024), dtype=numpy.int16)
    unique, frequency = numpy.unique(data, return_counts=True, axis=0)
    try:
        rows, cols = zip(*unique)
        imagedata[cols, rows] = frequency
    except ValueError:
        pass
    return imagedata
        

while isRunning:
    if not INFINITE_SERVER: isRunning=False
    print('Waiting a new client connection..')
    conn, addr = serv.accept() #It hangs here until a client connects.
    conn.settimeout(0.005)
    with conn:
        print('connected by', addr)
        loop = 0
        now_data=b''
        
        while True:
            now_file = os.path.join(FOLDER, "tdc_check_000"+format(loop, '.0f').zfill(3)+".tpx3")
            if os.path.isfile(now_file):
                now_data = open_and_read(now_file, loop)
            else:
                while not os.path.isfile(now_file):
                    try:
                        data = conn.recv(64)
                        if not data:
                            break
                    except socket.timeout:
                        """
                        Just so we dont hang in conn.recv
                        """
                        print(f'Timeout at loop {loop}.')
                        break
                    except ConnectionResetError:
                        print(f'Nionswift closed without Stoping camera. Reinitializating')
                        break
                try:
                    now_data = open_and_read(now_file, loop)
                    print(f'New file found. Opening it.')
                except FileNotFoundError:
                    print(f'Connection broken by client. Reinitializating')
                    break

            electron_event, tdc_event = check_data(now_data)
            pos, _ = data_from_raw_electron(electron_event)
            final_data = create_image_from_events(pos)

            final_send_data = pickle.dumps(final_data)

            try:
                conn.send(final_data)
                now_data = b''
            except ConnectionResetError:
                break
            except socket.timeout:
                pass

            loop+=1
            time.sleep(TIME_INTERVAL)

            if MAX_LOOPS and loop==MAX_LOOPS:
                break

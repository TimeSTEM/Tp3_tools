import socket
import os
import numpy
import time

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
#SERVER_HOST = '127.0.0.1' #127.0.0.1 is LOCALHOST. Not visible in the network.
SERVER_HOST = '192.0.0.11' #When not using in localhost
SERVER_PORT = 65431 #Pick a port to connect your socket
INFINITE_SERVER = True #This hangs for a new client after a client has been disconnected.
CREATE_TDC = True #if you wanna to add a tdc after the end of each read frame
TIME_INTERVAL = 0.001 #If no sleep, streaming is too fast
MAX_LOOPS = 0 #Max number of loops
FILE_EXISTS = True #False if you streaming directly

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
                now_data += open_and_read(now_file, loop)
            else:
                if FILE_EXISTS:
                    loop = 0
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
                            pass
                        except ConnectionResetError:
                            print(f'Nionswift closed without Stoping camera. Reinitializating')
                            break
                    try:
                        now_data += open_and_read(now_file, loop)
                        print(f'New file found. Opening it.')
                    except FileNotFoundError:
                        print(f'Connection broken by client. Reinitializating')
                        break

            try:
                conn.send(now_data)
                now_data = b''
            except ConnectionResetError:
                break
            except socket.timeout:
                pass

            loop+=1
            time.sleep(TIME_INTERVAL)

            if MAX_LOOPS and loop==MAX_LOOPS:
                break

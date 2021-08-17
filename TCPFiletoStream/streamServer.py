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
"""

FOLDER = '23072021-00'
CREATE_TDC = False #if you wanna to add a tdc after the end of each read frame
HOST = '127.0.0.1' #127.0.0.1 is LOCALHOST. Not visible in the network.
PORT = 8098 #Pick a port to connect your socket
INFINITE_SERVER = True #This hangs for a new client after a client has been disconnected.
loop_min = 0

"""
Uncomment code below if you wish a meaningfull wobbler data with a fake tdc
given by start time TIME_OFFSET and interval by TIME_INTERVAL
"""
#FOLDER = 'wobbler_data'
#TIME_OFFSET = 0.026
#TIME_INTERVAL = 0.035 #If no sleep, streaming is too fast
#CREATE_TDC = True #if you wanna to add a tdc after the end of each read frame


"""
Script starts here
"""
serv = socket.socket(socket.AF_INET, socket.SOCK_STREAM) #AF_INET is IPV4. It expects a tuple (HOST, PORT). SOCK_STREAM is TCP protocol
serv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1) #To reuse the socket.

isRunning = True

def open_and_read(filepath, number):
    with open(filepath, "rb") as f:
        data = f.read()
        if CREATE_TDC: data += create_tdc(int((TIME_OFFSET + TIME_INTERVAL*loop)*1e9))
    return data

while isRunning:
    if not INFINITE_SERVER: isRunning=False
    start = time.time()
    print('Waiting a new client connection..')
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as serv:
        try:
            serv.connect((HOST, PORT))
        except ConnectionRefusedError:
            pass
        print('TCP3 raw connected.')
        loop = loop_min
        now_data=b''
        
        while True:
            now_file = os.path.join(FOLDER, "raw000"+format(loop, '.0f').zfill(3)+".tpx3")
            if os.path.isfile(now_file):
                print(now_file)
                np_data = numpy.fromfile(now_file, dtype='uint8')
            else:
                print('Resetting. Current loop is', loop, '. Time is', time.time()-start)
                start = time.time()
                loop = loop_min
                now_file = os.path.join(FOLDER, "raw000"+format(loop, '.0f').zfill(3)+".tpx3")
                np_data = numpy.fromfile(now_file, dtype='uint8')

            #split_data = numpy.array_split(np_data, 32)

            try:
                send = serv.send(np_data)
                #print(send)
                #for val in split_data:
                #    size = serv.send(val)
                #    print(size)
            except ConnectionResetError:
                break
            except ConnectionAbortedError:
                """Breaking NS"""
                break
            except ConnectionRefusedError:
                """Too much time off"""
                break

            loop+=1

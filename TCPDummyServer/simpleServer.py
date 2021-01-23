import socket
import time
import numpy

def create_data(Tdif):
    
    #Message First Part
    data = b'TPX3'
    data+=b'\x03'
    data+=b'\x00'
    #data+=b'\x08'
    sending = int(numpy.random.rand()*5)*8
    data+=bytes([sending])
    data+=b'\x00' #More significant byte
    
    data2=b''
    for j in range(int(sending/8)):
        #End Message
        end = '1011' #4 bits. This is 0xb
        Tend = int('0x000000000000000b', 16) #4 bits

        #Pix Address
        Rdcol = int(numpy.random.rand()*127)
        Rspix = int(numpy.random.rand()*63)
        Rpix = int(numpy.random.rand()*7)
        
        dcol = bin(Rdcol)[2:].zfill(7)
        spix = bin(Rspix)[2:].zfill(6)
        pix = bin(Rpix)[2:].zfill(3)
        #spix = '011111' #6 bits
        #pix = '001' #3bits
        #Tpix = int('0x00000000000E0000', 16) #3 bits
        #Tspix = int('0x000000000001f800', 16) #6 bits
        #Tdcol = int('0x00000000000007f0', 16) #7 bits
        
        #FTOA, TOT, TOA
        toa = '00000000000000' #14 bits
        tot = '0000001100' #10 bits
        ftoa = '1111' #4 bits

        #SPIDR
        timeDif = Tdif - int(Tdif/26843136000)*26843136000
        curCT = (timeDif) / (25.0 * 16384.0)
        MSB_spidr = bytes([int(curCT/256)])
        LSB_spidr = bytes([int(curCT%256)])

        #FTOA, TOT, TOA and Pix Address message
        msg = int(end+dcol+spix+pix+toa+tot+ftoa, 2) #48 bits = 6 bytes
        hex_msg = hex(msg)
        hex_msg=hex_msg[2:]
        data2 += bytes.fromhex(hex_msg)
        data2+=MSB_spidr
        data2+=LSB_spidr
    return data+data2[::-1]

def create_tdc(Tdif, trigger='tdc1Ris'):
    
    
    #Message First Part
    data = b'TPX3'
    data+=b'\x03'
    data+=b'\x00'
    data+=b'\x08'
    data+=b'\x00' #More significant byte

    end = '0110'

    if trigger=='tdc1Ris':
        triggerType = '1111'
    elif trigger=='tdc1Fal':
        triggerType = '1010'
    elif trigger=='tdc2Ris':
        triggerType = '1110'
    elif trigger=='tdc2Fal':
        triggerType = '1011'

    timeDif = Tdif - int(Tdif/107374182396)*107374182396
    
    triggerCounter = '000000000000'
    TimeStamp = bin(int(timeDif/1e9*320e6))[2:].zfill(35)
    RFine = int(numpy.random.rand()*15) #4 bits
    Fine = bin(RFine)[2:].zfill(4)
    Reserved = '00000'
    
    msg = int(end+triggerType+triggerCounter+TimeStamp+Fine+Reserved, 2) #64 bits = 8 bytes
    hex_msg = hex(msg)
    hex_msg=hex_msg[2:]
    data2 = bytes.fromhex(hex_msg)
    return data+data2[::-1]

"""
Set Script Parameters Here
"""
filename = '../RawAnalysis/temp.tpx3'
SERVER_HOST = '127.0.0.1' #127.0.0.1 is LOCALHOST. Not visible in the network.
#SERVER_HOST = '129.175.81.162' #When not using in localhost
SERVER_PORT = 65431 #Pick a port to connect your socket
SAVE_FILE = False #Save a file in filename $PATH.
INFINITE_SERVER = False #This hangs for a new client after a client has been disconnected.
MAX_LOOPS = 1 #Maximum number of loops. MAX_LOOPS = 0 means not maximal value.

"""
Script starts here
"""

serv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
serv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
serv.bind((SERVER_HOST, SERVER_PORT))
serv.listen()

isRunning = True

while isRunning:
    if not INFINITE_SERVER: isRunning=False
    print('Waiting a new client connection..')
    conn, addr = serv.accept()
    with conn:
        print('connected by', addr)
        myFile = open(filename, 'wb')
        loop = 0
        
        final_data = b''
        for i in range(512):
            final_data+=create_data(time.perf_counter_ns())
        conn.send(final_data)
        
        while True:
            loop+=1

            final_data=b''
            
            final_data+=create_tdc(time.perf_counter_ns(), 'tdc1Ris')
            time.sleep(0.001)
            num = 1000
            for i in range(num):
                final_data+=create_data(time.perf_counter_ns())
            
            if SAVE_FILE: myFile.write(final_data)
            
            try:
                conn.send(final_data)
            except:
                print('Connection broken by client. Opening a new one..')
                break

            if MAX_LOOPS and loop>=MAX_LOOPS:
                break

            

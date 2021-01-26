import socket
import time
import numpy
import math

def create_data(Tdif):
    
    #Message Header
    data = b'TPX3'
    ci = int(numpy.random.rand()*4)
    data+=bytes([ci]) #This is chip index. Always 03 for no reason.
    data+=b'\x00' #Mode. 
    sending = int(numpy.random.rand()*3+1)*8
    data+=bytes([sending]) #Number of pixels in chunk [LSB]
    data+=b'\x00' #Number of pixels in chunk [MSB]
    
    data2=b''
    for j in range(int(sending/8)):
        #End Message
        end = '1011' #4 bits. This is 0xb

        #Pix Address
        Rdcol = int(numpy.random.rand()*127)
        Rspix = int(numpy.random.rand()*63)
        Rpix = int(numpy.random.rand()*7)
        
        dcol = bin(Rdcol)[2:].zfill(7) #7 bits
        spix = bin(Rspix)[2:].zfill(6) #6 bits
        pix = bin(Rpix)[2:].zfill(3) #3 bits
        
        #FTOA, TOT, TOA
        toa = '00000000000000' #14 bits
        tot = '0000001100' #10 bits
        ftoa = '1111' #4 bits

        #SPIDR
        timeDif = Tdif - int(Tdif/26843136000)*26843136000 #16 bits. Max time ~26.84s
        curCT = (timeDif) / (25.0 * 16384.0)
        MSB_spidr = bytes([int(curCT/256)]) #SPIDR [MSB]
        LSB_spidr = bytes([int(curCT%256)]) #SPIDR [LSB]

        #FTOA, TOT, TOA and Pix Address message
        msg = int(end+dcol+spix+pix+toa+tot+ftoa, 2) #48 bits = 6 bytes
        hex_msg = hex(msg)
        hex_msg=hex_msg[2:]
        data2 += bytes.fromhex(hex_msg)
        data2+=MSB_spidr
        data2+=LSB_spidr
    return data+data2[::-1] #Second part is inversed because it is easier to read.

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
"""
filename = '../RawAnalysis/temp.tpx3'
#SERVER_HOST = '127.0.0.1' #127.0.0.1 is LOCALHOST. Not visible in the network.
SERVER_HOST = '129.175.108.58' #When not using in localhost
SERVER_PORT = 65431 #Pick a port to connect your socket
SAVE_FILE = False #Save a file in filename $PATH.
INFINITE_SERVER = True #This hangs for a new client after a client has been disconnected.
MAX_LOOPS = 0 #Maximum number of loops. MAX_LOOPS = 0 means not maximal value.
INTERVAL_TDC = 0.05
SEND_TIME = True #If True you must send a time to replace interval tdc.


"""
Script starts here
"""
serv = socket.socket(socket.AF_INET, socket.SOCK_STREAM) #AF_INET is IPV4. It expects a tuple (HOST, PORT). SOCK_STREAM is TCP protocol
serv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1) #To reuse the socket.
serv.bind((SERVER_HOST, SERVER_PORT))
serv.listen()

isRunning = True

while isRunning:
    if not INFINITE_SERVER: isRunning=False
    print('Waiting a new client connection..')
    conn, addr = serv.accept() #It hangs here until a client connects.
    with conn:
        print('connected by', addr)
        start = time.perf_counter_ns()
        if SEND_TIME:
            INTERVAL_TDC = float(conn.recv(8))
        if SAVE_FILE: myFile = open(filename, 'wb')
        loop = 0
        
        final_data = b''

        for i in range(1000):
            final_data+=create_data(time.perf_counter_ns())
        conn.send(final_data)

        while True:
            
            if loop * INTERVAL_TDC < time.perf_counter_ns()/1e9:
                loop = math.ceil( (time.perf_counter_ns()/1e9) / INTERVAL_TDC )
                final_data+=create_tdc(time.perf_counter_ns(), 'tdc1Ris')
            else:
                final_data+=create_data(time.perf_counter_ns())
            
            if SAVE_FILE: myFile.write(final_data)
            
            try:
                conn.send(final_data)
                final_data=b''
            except:
                print('Connection broken by client. Opening a new one..')
                finish = time.perf_counter_ns()
                print(f"Total elapsed time is {(finish-start)/1e9} s. Number of loops are {loop}.")
                break

            if MAX_LOOPS and loop>=MAX_LOOPS:
                finish = time.perf_counter_ns()
                print(f"Total elapsed time is {(finish-start)/1e9} s. Number of loops are {loop}.")
                break

            

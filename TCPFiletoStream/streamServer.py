import socket
import os

"""
Set Script Parameters Here
Options for server are:
    - 129.175.81.162 for my PC;
    - 129.175.108.58 for Patrick's;
"""
FOLDER = 'Files_00'
SERVER_HOST = '127.0.0.1' #127.0.0.1 is LOCALHOST. Not visible in the network.
#SERVER_HOST = '129.175.108.58' #When not using in localhost
SERVER_PORT = 65430 #Pick a port to connect your socket
INFINITE_SERVER = True #This hangs for a new client after a client has been disconnected.

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
        loop = 0
        
        while True:
            try:
                now_file = os.path.join(FOLDER, "tdc_check_000"+format(loop, '.0f').zfill(3)+".tpx3")
                f = open(now_file, "rb")
                now_data = f.read()
            except FileNotFoundError:
                now_data = b''

            conn.send(now_data)
            loop+=1

            

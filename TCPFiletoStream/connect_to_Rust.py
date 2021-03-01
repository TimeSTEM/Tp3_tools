import socket
import time
import struct

HOST = '127.0.0.1'
PORT = 8088

c1 = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
c2 = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
c1.connect((HOST, PORT))
c2.connect((HOST, PORT))

msg = b'\x00\x02\x00\x01'
msg+= struct.pack('>H', 256)
msg+= struct.pack('>H', 256)
msg+= struct.pack('>H', 256)
msg+= struct.pack('>H', 256)
msg+= struct.pack('>d', 0.0)
msg+= struct.pack('>d', 0.0)
c1.send(msg)
time.sleep(5)
data = c1.recv(10000000)
print(len(data))

import socket
import time

HOST = '127.0.0.1'
PORT = 8088
buf = 4096 * 8

with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as s:
    s.connect((HOST, PORT))
    size = 0
    #s.sendall(bytes([0, 2, 0, 0, 0, 1, 0, 1, 2, 0, 2, 0, 0, 0, 3, 232, 0, 1, 0, 1]))
    s.sendall(bytes([1, 2, 0, 2, 2, 0, 2, 0, 2, 0, 2, 0, 0, 0, 0, 0, 0, 1, 0, 1])) #Spec Image
    while True:
        a = s.recv(buf)
        size += len(a)
        if len(a) == 0:
            break
print(size / 1e6)

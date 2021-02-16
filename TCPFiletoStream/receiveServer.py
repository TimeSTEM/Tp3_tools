import socket

SERVER_HOST = '127.0.0.1'
SERVER_PORT = 8098

print('waiting for TPX3')

serv = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
serv.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
serv.bind((SERVER_HOST, SERVER_PORT))
serv.listen()
conn, addr = serv.accept()
with conn:
    print('connected by', addr)
    while True:
        data = conn.recv(512)
        if not data:
            break
        print(data)
#serv.close()

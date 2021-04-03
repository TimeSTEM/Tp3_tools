import numpy
import matplotlib.pyplot as plt
import time
import os

xL = list() #x position
yL = list() #Y position
globalTL = list() #Global Time
tdc1RL = list() #Rising Edge tdc1 Time
tdc1RL_dif = list() #Difference in tdc1 Time

i = [0, 0] #Counter. First index is electron event and second is tdc event.
final_tdc = 0 #Last tdc time received
start = time.time() 

#FOLDER = '22-03-2021'
FOLDER = '../TCPFiletoStream/laser_tdc'

for data in os.listdir(FOLDER):# in datas:
    print(f'Looping over file {data}.')
    with open(os.path.join(FOLDER, data), "rb") as f:
        all_data = f.read()
        index = 0 #Reading index.
        while True:
            byte = all_data[index: index+8] #8 bytes header
            byte=byte[::-1]
            if byte==b'': #Breaks if there is none
                break
            tpx3_header = byte[4:8] #4 bytes=32 bits
            assert tpx3_header==b'3XPT' #must be this
            chip_index = byte[3] #1 byte
            #mode = byte[2] #1 byte
            size_chunk1 = byte[1] #1 byte
            size_chunk2 = byte[0] #1 byte
            total_size = size_chunk1+ size_chunk2*256 #Number of pixels in chunk
            for j in range(int(total_size/8)):
                index+=8 #Go to next 8 bytes
                byte = all_data[index:index+8]
                byte=byte[::-1]
                id = (byte[0] & 240)>>4 #240 = 1111 0000. 240 >> 4 = 0000 11111.
                if id==11: #11 = 0xb (Electron Event)
                    i[0]+=1 #Increment Electron Event
                    
                    #Bit displacement syntax. Fastest way to do it.
                    dcol = ((byte[0] & 15)<<4) + ((byte[1] & 224)>>4)
                    spix = ((byte[1] & 31)<<3) + ((byte[2] & 128)>>5)
                    pix = (byte[2] & 112)>>4
                    
                    x = int(dcol + pix/4)
                    y = int(spix + (pix & 3))

                    toa = ((byte[2] & 15)<<10) + ((byte[3] & 255)<<2) + ((byte[4] & 192)>>6)
                    tot = ((byte[4] & 63)<<4) + ((byte[5] & 240)>>4)
                    ftoa = (byte[5] & 15)
                    spidr = ((byte[6] & 255)<<8) + ((byte[7] & 255))
                    ctoa = toa<<4 | ~ftoa & 15
                    


                    spidrT = spidr * 25.0 * 16384.0
                    toa_ns = toa * 25.0
                    tot_ns = tot * 25.0
                    global_time = spidrT + ctoa * 25.0/16.0

                    if chip_index==0:
                        x = 255 - x
                        y = y
                    elif chip_index==1:
                        x = 255*4 - x
                        y = y
                    elif chip_index==2:
                        x = 255*3 - x
                        y = y
                    elif chip_index==3:
                        x = 255*2 - x
                        y = y

                    xL.append(x)
                    yL.append(y)
                    globalTL.append(global_time/1e9)

                
                elif id==6: #6 = 0xb. This is a tdc event.
                    i[1]+=1 #Increment tdc event.
                    coarseT = ((byte[2] & 15)<<31) + ((byte[3] & 255)<<23) + ((byte[4] & 255)<<15) + ((byte[5] & 255)<<7) + ((byte[6] & 254)>>1)
                    fineT = ((byte[6] & 1)<<3) + ((byte[7] & 224)>>5)
                    tdcT = coarseT * (1/320.0e6) + fineT*260.0e-12
                    final_tdc = tdcT

                    #Syntax to grab time difference between consecutive tdc's
                    try:
                        tdc1RL_dif.append((tdcT-new)*1e6)
                    except:
                        pass
                    new = tdcT
                    
                    triggerType = byte[0] & 15 #15 = 1111. Get trigger Type.
                    if triggerType==15: tdc1RL.append(tdcT)
                    elif triggerType==10: pass #print('tdc1Fal')
                    elif triggerType==14: pass #print('tdc2Ris')
                    elif triggerType==11: pass #print('tdc2Fal')
            index+=8 #Goes back to next header
                
finish = time.time()
print(f'Total time is {finish-start} with {i} events. Events per second is {sum(i) / (finish-start)}. Final tdc is {final_tdc}')

fig, ax = plt.subplots(3, 1)
ax[0].hist2d(xL, yL, bins=256, range=([0, 1024], [0, 256]))
ax[1].plot(globalTL, label='Electrons')
ax[1].plot(tdc1RL, color='red', label='TDC1Ris')
ax[2].hist(tdc1RL_dif, bins=100)
                
ax[1].legend()
ax[0].set_ylabel('Y')
ax[0].set_xlabel('X')

ax[1].set_ylabel('Time Elapsed (s)')
ax[1].set_xlabel('Counts')

ax[2].set_ylabel('Counts')
ax[2].set_xlabel('Time Difference (us)')
plt.show()


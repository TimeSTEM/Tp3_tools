import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
import time
import os

xL = list() #x position
yL = list() #Y position
Tafter = list() #Global Time
last_laser = [0.0, 0.0, 0.0, 0.0, 0.0] #Rising Edge tdc1 Time

i = [0, 0] #Counter. First index is electron event and second is tdc event.
final_tdc = 0 #Last tdc time received
start = time.time() 

FOLDER = '../TCPFiletoStream/GainRawTP3/25-53-25262(132)'
#FOLDER = '../TCPFiletoStream/gain_data'
WIDTH = 100000e-9
DELAY = 0e-9
HOR = 250
MIN_HOR = 0

def check_if_in(ele_time, tdc_time_list):
    for val in tdc_time_list:
        if ele_time > val + DELAY and ele_time < val + DELAY + WIDTH:
            return (True, val)
    return (False, 0)


for data in os.listdir(FOLDER):# in datas:
    print(f'Looping over file {data}.')
    if i[0] != 0: break
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
                    
                    toa = ((byte[2] & 15)<<10) | ((byte[3] & 255)<<2) | ((byte[4] & 192)>>6)
                    #tot = ((byte[4] & 63)<<4) | ((byte[5] & 240)>>4)
                    ftoa = (byte[5] & 15)
                    spidr = ((byte[6] & 255)<<8) + ((byte[7] & 255))
                    ctoa = toa<<4 | ~ftoa & 15
                    
                    spidrT = spidr * 25.0 * 16384.0
                    #toa_ns = toa * 25.0
                    #tot_ns = tot * 25.0
                    global_time = (spidrT + ctoa * 25.0/16.0)/1e9

                    

                    #if global_time > last_laser + DELAY and global_time < last_laser + DELAY + WIDTH:
                    res = check_if_in(global_time, last_laser)
                    if res[0]:
                        tdc_ref = res[1]

                        dcol = ((byte[0] & 15)<<4) | ((byte[1] & 224)>>4)
                        spix = ((byte[1] & 31)<<3) | ((byte[2] & 128)>>5)
                        pix = (byte[2] & 112)>>4

                        x = dcol | pix>>4
                        y = int(spix | (pix & 3))

    
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
                        if x<HOR and x>=MIN_HOR:
                            Tafter.append((global_time - tdc_ref)*1e6)

                
                elif id==6: #6 = 0xb. This is a tdc event.
                    i[1]+=1 #Increment tdc event.
                    triggerType = byte[0] & 15 #15 = 1111. Get trigger Type.
                    if triggerType==11:
                        coarseT = ((byte[2] & 15)<<31) + ((byte[3] & 255)<<23) + ((byte[4] & 255)<<15) + ((byte[5] & 255)<<7) + ((byte[6] & 254)>>1)
                        fineT = ((byte[6] & 1)<<3) + ((byte[7] & 224)>>5)
                        tdcT = coarseT * (1/320e6) + fineT*260e-12
                        last_laser.pop(0)
                        last_laser.append(tdcT)
                else: 
                    pass
            index+=8 #Goes back to next header
                
finish = time.time()
print(f'Total time is {finish-start} with {i} events. Last laser is at {last_laser}')

fig, ax = plt.subplots(3, 1, dpi=160)
ax[0].hist2d(xL, yL, bins=100, range=([0, 250], [0, 256]), norm=mcolors.PowerNorm(0.3))
ax[0].axvline(x=HOR, color='white')
if MIN_HOR>0: ax[0].axvline(x=MIN_HOR, color='red')
ax[1].hist2d(xL, yL, bins=49, range=([0, HOR], [0, 256]), norm=mcolors.PowerNorm(0.3))
ax[2].hist(Tafter, bins=200, density=True, range=(DELAY*1e6, (DELAY+WIDTH)*1e6))
                
ax[0].set_ylabel('Y')
ax[0].set_xlabel('X')

ax[1].set_ylabel('Y')
ax[1].set_xlabel('X')

ax[2].set_ylabel('Event counts')
ax[2].set_xlabel('Time Elapsed from TDC (us)')

plt.savefig(FOLDER + '.png')
numpy.save(FOLDER + '.npy', Tafter)
plt.show()



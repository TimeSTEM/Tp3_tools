import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from scipy.special import factorial
from scipy.optimize import curve_fit
import time
import os

xL = list() #x position
yL = list() #Y position
T = list() #Global Time

A = 6.242*10**(18) #C/s
cur = 0.26*10**(-12) #C/s
ele = cur*A
events = 10**-6 * ele


t = numpy.arange(0.0, 10.0, 0.1)
d = numpy.exp(-events) * numpy.power(events, t)/factorial(t)

#plt.plot(t, d, label="l = "+str(events) )
#plt.legend()
#plt.ylabel('Probability')
#plt.xlabel('K (us)')
#plt.show()

def exp_decay(x, a, l):
    return a*numpy.exp(-x*l)


i = [0, 0] #Counter. First index is electron event and second is tdc event.
final_tdc = 0 #Last tdc time received
start = time.time() 

FOLDER = '../TCPFiletoStream/18-03-2021'

MIN_HOR = 100 #100
MAX_HOR = 190 #190

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
                    
                    toa = ((byte[2] & 15)<<10) | ((byte[3] & 255)<<2) | ((byte[4] & 192)>>6)
                    tot = ((byte[4] & 63)<<4) | ((byte[5] & 240)>>4)
                    ftoa = (byte[5] & 15)
                    spidr = ((byte[6] & 255)<<8) + ((byte[7] & 255))
                    ctoa = toa<<4 | ~ftoa & 15
                    
                    spidrT = spidr * 25.0 * 16384.0
                    #toa_ns = toa * 25.0
                    tot_time= tot * 25.0 / 1e9
                    global_time = (spidrT + ctoa * 25.0/16.0)/1e9

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
                    
                    if x>MIN_HOR and x<MAX_HOR:
                        xL.append(x)
                        yL.append(y)
                        #T.append(global_time*1e9)
                        T.append(int(global_time*1e6))

                
                elif id==6: #6 = 0xb. This is a tdc event.
                    i[1]+=1 #Increment tdc event.
                
                else: 
                    pass

            index+=8 #Goes back to next header
                
finish = time.time()
print(f'Total time is {finish-start} with {i} events. Number of positional electrons are {len(xL)}. Number of temporal electrons are {len(T)}.')


T.sort()
T = [T[x+1]-T[x] for x in range(len(T)-1)]

uni, cts = numpy.unique(T, return_counts = True)
uni = uni[0:10]
cts = cts[0:10]

fig, ax = plt.subplots(1, 1, dpi=200)
popt, pcov = curve_fit(exp_decay, uni, cts)
print(popt, pcov)
ax.scatter(uni, cts, label='Data', color='red')
ax.plot(uni, exp_decay(uni, *popt), label='Exponential Decay Fitting', linewidth=2.0, linestyle='--')

ax.set_ylabel('Events', fontsize=8)
ax.set_xlabel('Time interval (us)', fontsize=8)
ax.set_yscale('log')
ax.legend(fontsize=10)

plt.show()

"""
fig, ax = plt.subplots(3, 1, dpi=160)
ax[0].hist2d(xL, yL, bins=100, range=([0, 1024], [0, 256]), norm=mcolors.PowerNorm(0.3))
ax[0].axvline(x=MIN_HOR, color='red')
ax[0].axvline(x=MAX_HOR, color='white')
ax[1].hist(Tafter, bins=200, density = True, range=(DELAY*1e6, (DELAY+WIDTH)*1e6))
ax[2].hist(Tdif, bins=200, density = True)
                
ax[0].set_ylabel('Y')
ax[0].set_xlabel('X')

ax[1].set_ylabel('Event counts')
ax[1].set_xlabel('Time Elapsed from TDC (us)')

ax[2].set_ylabel('Event counts')
ax[2].set_xlabel('TDC time distribution (us)')

#plt.show()
"""

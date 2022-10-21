
# coding = utf-8 


import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from scipy.optimize import curve_fit

off = 0
disp = 1
SPIM_PIXELS = 1225
TIME_DELAY = 525
TIME_WIDTH = 25

def gaussian(x, mean, amplitude, sigma, offset):
    return offset + amplitude * numpy.exp( -(x-mean)**2 / (2*sigma ** 2))

class TimepixData:
    def __init__(self):
        self.t = numpy.zeros(0, dtype='int64')
        self.tabs = numpy.zeros(0, dtype='uint64')
        self.g2t = numpy.zeros(0, dtype='int64')
        self.xT = numpy.zeros(SPIM_PIXELS)
        self.x = numpy.zeros(SPIM_PIXELS)
        self.xH = numpy.zeros(0, dtype='uint32')
        self.yH = numpy.zeros(0, dtype='uint32')
        self.tot = numpy.zeros(0, dtype='uint16')
        self.channel = numpy.zeros(0, dtype='uint32')
        
        self.active_index_list = None
        self.index_chip1 = None
        self.index_chip2 = None
        self.index_chip3 = None
        self.index_chip4 = None

        self.cle = numpy.zeros(SPIM_PIXELS)
        self.cle_g2 = numpy.zeros(SPIM_PIXELS)

    def __get_attribute_type(self, attr):
        if attr == 'tH.txt':
            attr_type = 'int64'
        elif attr == 'tabsH.txt':
            attr_type = 'uint64'
        elif attr == 'g2tH.txt':
            attr_type = 'int64'
        elif attr == 'xH.txt':
            attr_type = 'uint32'
        elif attr == 'yH.txt':
            attr_type = 'uint32'
        elif attr == 'tot.txt':
            attr_type = 'uint16'
        elif attr == 'channel.txt':
            attr_type = 'uint32'
        return attr_type

    def __set_attribute_object(self, attr, temp):
        if attr == 'tH.txt':
            self.t = numpy.append(self.t, temp)
        elif attr == 'tabsH.txt':
            self.tabs = numpy.append(self.tabs, temp)
        elif attr == 'g2tH.txt':
            self.g2t = numpy.append(self.g2t, temp)
        elif attr == 'xH.txt':
            self.xH = numpy.append(self.xH, temp)
        elif attr == 'yH.txt':
            self.yH = numpy.append(self.yH, temp)
        elif attr == 'tot.txt':
            self.tot = numpy.append(self.tot, temp)
        elif attr == 'channel.txt':
            self.channel = numpy.append(self.channel, temp)

    def __add_attribute(self, path, attr):
        
        attr_type = self.__get_attribute_type(attr)
        temp = numpy.fromfile(path+attr, dtype=attr_type)
        self.__set_attribute_object(attr, temp)
        self.active_index_list = numpy.arange(0, len(temp), 1)

        
        if attr == 'tH.txt':
            self.__get_time_limits()

    def update_indexes(self):
        self.indexes_ch1 = numpy.where((self.channel == 0))
        self.indexes_ch2 = numpy.where((self.channel == 12))
        self.index_chip1 = numpy.where((self.xH < 256))
        self.index_chip2 = numpy.where((self.xH < 512) & (self.xH > 256))
        self.index_chip3 = numpy.where((self.xH < 768) & (self.xH > 512))
        self.index_chip4 = numpy.where((self.xH > 768))

    def get_cle(self):
        indexes_cle = numpy.where((numpy.abs(self.t + TIME_DELAY) < TIME_WIDTH))
        for val in self.xH[indexes_cle]:
            self.cle[val] += 1

    def get_cle_g2(self):
        pass

    def add_relative_time(self, path):
        self.__add_attribute(path, "tH.txt")
    
    def add_absolute_time(self, path):
        self.__add_attribute(path, "tabsH.txt")

    def add_correlation_time(self, path):
        self.__add_attribute(path, "g2tH.txt")
    
    def add_xposition(self, path):
        self.__add_attribute(path, "xH.txt")
    
    def add_yposition(self, path):
        self.__add_attribute(path, "yH.txt")

    def add_tot(self, path):
        self.__add_attribute(path, "tot.txt")

    def add_channel(self, path):
        self.__add_attribute(path, "tot.txt")

    def add_total_spec(self, path):
        self.xT += numpy.loadtxt(path+"spec.txt", delimiter=',')
    
    def add_spec(self, path):
        self.x += numpy.loadtxt(path+"cspec.txt", delimiter=',')

    def set_active_list_by_fixed_tot(self, tot):
        self.active_index_list = numpy.where((self.tot == tot))
    
    def set_active_list_by_window_tot(self, tot_min, tot_max):
        self.active_index_list = numpy.where((self.tot > tot_min) & (self.tot < tot_max))

    def correct_time_delay(self, div, save=False):
        fig, ax = plt.subplots(nrows=2, ncols=2)
        divy = int(div / 4)
        delay_array = numpy.zeros((div, divy))
        fwhm_array = numpy.zeros((div, divy))
        for i in range(div):
            for iy in range(divy):
                indexes_chip = numpy.where((self.xH[self.active_index_list] < (1024/div)*(i+1)) & (self.xH[self.active_index_list] > (1024/div)*i) & (self.yH[self.active_index_list] < (256/divy) * (iy + 1)) & (self.yH[self.active_index_list] > (256/divy)*iy))
                try:
                    val = self.__plot_histogram(indexes_chip, 'correction', ax[0, 1])
                except:
                    print('***NOTE***: Problem during fitting. Using standard values.')
                    val = [-TIME_DELAY, 0.01, 5., 0.005]
                delay_array[i, iy] = -(numpy.round(val[0]) + TIME_DELAY)
                fwhm_array[i, iy] = val[2]
        delay = ax[0, 0].imshow(numpy.transpose(delay_array), aspect='equal', origin='upper')
        fig.colorbar(delay, ax = ax[0, 0])
        fwhm = ax[1, 0].imshow(numpy.transpose(fwhm_array), aspect='equal', origin='upper')
        fig.colorbar(fwhm, ax = ax[1, 0])
        h = ax[1, 1].hist2d(self.t[self.active_index_list], self.tot[self.active_index_list], range=[[self.tmin, self.tmax], [0, 100]], bins=[self.tbin, 100])
        fig.colorbar(h[3], ax = ax[1, 1])
        if save: numpy.save('delay_array_'+str(div), delay_array)
        plt.show()

    def __get_time_limits(self):
        self.tmax = int(numpy.max(self.t)) 
        self.tmin = int(numpy.min(self.t))
        self.tbin = int((self.tmax - self.tmin)) + 1
        #tbin = int(tbin/6)


    def __plot_histogram(self, indexes, label, ax):
        bin_heights, bin_borders, _ = ax.hist(self.t[self.active_index_list][indexes], density = True, bins=self.tbin, range=(self.tmin, self.tmax), alpha=0.2, label=label)
        bin_centers = bin_borders[:-1] + numpy.diff(bin_borders) / 2
        popt, _ = curve_fit(gaussian, bin_centers, bin_heights, p0 = [-TIME_DELAY, 0.01, 5., 0.005], bounds=([-numpy.inf, 0, 0, 0], [numpy.inf, 0.1, 20, 0.01]))
        x_interval_for_fit = numpy.linspace(bin_borders[0], bin_borders[-1], 10000)
        print('Plotting for chip ' + label + f'. The values are {popt} and {numpy.round(popt[0], 0) + TIME_DELAY}')
        ax.plot(x_interval_for_fit, gaussian(x_interval_for_fit, *popt))
        return popt


#data = TimepixData()
#path = "backup_tot60\\"
#path = ['Y:\\VG Lumiere\\TP3\Data\\23-09-2022\\raw7-isi323\\', 'Y:\\VG Lumiere\\TP3\Data\\23-09-2022\\raw3-isi320\\', 
#        'Y:\\VG Lumiere\\TP3\Data\\23-09-2022\\raw4-isi319\\', 'Y:\\VG Lumiere\\TP3\Data\\23-09-2022\\raw5-isi321\\', 
#        'Y:\\VG Lumiere\\TP3\Data\\23-09-2022\\raw2\\', 'Y:\\VG Lumiere\\TP3\Data\\30-09-2022\\raw3-isi334\\',
#        'Y:\\VG Lumiere\\TP3\Data\\30-09-2022\\raw4-isi335\\', 'Y:\\VG Lumiere\\TP3\Data\\30-09-2022\\raw5-isi325\\', 
#        'Y:\\VG Lumiere\\TP3\Data\\30-09-2022\\raw5b-isi336\\', 'Y:\\VG Lumiere\\TP3\Data\\30-09-2022\\raw6-isi337\\',
#        'Y:\\VG Lumiere\\TP3\Data\\30-09-2022\\raw7-isi339\\']
#path = ['Y:\\VG Lumiere\\TP3\Data\\23-09-2022\\raw7-isi323\\']
#path = [""]

#for p in path:
#    data.add_xposition(p)
#    data.add_yposition(p)
#    data.add_tot(p)
#    data.add_relative_time(p)
#data.set_active_list_by_fixed_tot(60)
#data.set_active_list_by_window_tot(40, 100)
#data.correct_time_delay(16, True)

#"""
def correct_time(pos_array, time_array, div, corrections):
    assert len(corrections) == div
    for i in range(div):
        indexes_chip = numpy.where((pos_array < (1024/div)*(i+1)) & (pos_array > (1024/div)*i))
        time_array[indexes_chip] += int(corrections[i])

def plot_histogram(indexes, label, axis):
    bin_heights, bin_borders, _ = ax[axis, 1].hist(t[indexes], density = True, bins=tbin, range=(tmin, tmax), alpha=0.2, label=label)
    bin_centers = bin_borders[:-1] + numpy.diff(bin_borders) / 2
    try:
        popt, _ = curve_fit(gaussian, bin_centers, bin_heights, p0 = [-TIME_DELAY, 0.01, 5., 0.005], bounds=([-numpy.inf, 0, 0, 0], [numpy.inf, 0.1, 20, 0.01]))
        x_interval_for_fit = numpy.linspace(bin_borders[0], bin_borders[-1], 10000)
        print('Plotting for chip ' + label + f'. The values are {popt} and {numpy.round(popt[0], 0) + TIME_DELAY}')
        ax[axis, 1].plot(x_interval_for_fit, gaussian(x_interval_for_fit, *popt))
        return popt
    except:
        return [-TIME_DELAY, 0.01, 5., 0.005]

def correct_time2d(xpos_array, ypos_array, time_array, div, delay_array):
    divy = int(div / 4)
    for i in range(div):
        for iy in range(divy):
            indexes_chip = numpy.where((xpos_array < (1024/div)*(i+1)) & (xpos_array > (1024/div)*i) & (ypos_array < (256/divy) * (iy + 1)) & (ypos_array > (256/divy)*iy))
            time_array[indexes_chip] += int(delay_array[i, iy])

disparray = numpy.linspace(off, disp*SPIM_PIXELS, SPIM_PIXELS)
t = numpy.fromfile("tH.txt", dtype='int64')
double_t = numpy.fromfile("double_tH.txt", dtype='int64')
tabs = numpy.fromfile("tabsH.txt", dtype='uint64')
g2t = numpy.fromfile("g2tH.txt", dtype='int64')
g2_total = numpy.fromfile("isi_g2.txt", dtype='int64')
xT = numpy.loadtxt("spec.txt", delimiter=',')
x = numpy.loadtxt("cspec.txt", delimiter=',')
xH = numpy.fromfile("xH.txt", dtype='uint32')
yH = numpy.fromfile("yH.txt", dtype='uint32')
tot = numpy.fromfile("tot.txt", dtype='uint16')
channel = numpy.fromfile("channel.txt", dtype='uint32')

print(xH.shape)

indexes2 = numpy.where((channel == 0))
indexes12 = numpy.where((channel == 12))
indexes_tot = numpy.where((tot == 70))
indexes_chip1 = numpy.where((xH < 256))
indexes_chip2 = numpy.where((xH < 512) & (xH > 256))
indexes_chip3 = numpy.where((xH < 768) & (xH > 512))
indexes_chip4 = numpy.where((xH > 768))

factor = 10
indexes_begin_time = numpy.arange(0, int(len(tabs)/factor), 1)
indexes_middle_time = numpy.arange(int((factor/2-1)*len(tabs)/factor), int((factor/2)*len(tabs)/factor), 1)
indexes_end_time = numpy.arange(int((factor-1)*len(tabs)/factor), int(len(tabs)), 1)

#indexes_begin_time = numpy.where(tabs < 1*(max(tabs)+min(tabs))/20)
#indexes_middle_time = numpy.where((tabs < 10*(max(tabs)+min(tabs))/20))
#indexes_end_time = numpy.where(tabs > 19*(max(tabs)+min(tabs))/20)
#indexes_extra = numpy.where(xH > 70)

#Getting CLE
indexes_cle = numpy.where((numpy.abs(t + TIME_DELAY) < TIME_WIDTH))
cle = numpy.zeros(SPIM_PIXELS)
for val in xH[indexes_cle]:
    cle[val] += 1


#Getting g2
indexes_g2 = numpy.where( (numpy.abs(g2t) < 50))
indexes_g2_correlated = numpy.where( (numpy.abs(g2t) < 50) & (numpy.abs(t+TIME_DELAY) < TIME_WIDTH))
indexes_g2_chip1 = numpy.where( (numpy.abs(g2t) < 25) & (xH < 256))
indexes_g2_chip2 = numpy.where( (numpy.abs(g2t) < 25) & (xH < 512) & (xH > 256))
indexes_g2_chip3 = numpy.where( (numpy.abs(g2t) < 25) & (xH < 768) & (xH > 512))
indexes_g2_chip4 = numpy.where( (numpy.abs(g2t) < 25) & (xH > 768))
g2 = numpy.zeros(SPIM_PIXELS)
for val in xH[indexes_g2_correlated]:
    g2[val] += 1
unique, counts = numpy.unique(g2t, return_counts = True)
total = numpy.sum(counts)
g2_total_photons = numpy.sum(counts[1:])
g2_photons = numpy.sum(g2)
print(f'Total number of correlations is {total}. From these, {g2_total_photons} fall into the g2 statistics, and only {g2_photons} fall into the g2 curve, or {g2_photons * 100 /total}%.')

#Performing the ratios
cRatio = numpy.divide(cle, xT)
cRatio2 = numpy.divide(g2, xT)

#Bins
tmax = int(numpy.max(t)) 
tmin = int(numpy.min(t))
tbin = int((tmax - tmin)) + 1
#tbin = int(tbin/6)
print(tmax, tmin, tbin)

#Perform ToT chip correction
tot_correction = False
div = 4
if tot_correction:
    fig, ax = plt.subplots(nrows=2, ncols=2)
    divy = int(div / 4)
    tot_delay_array = numpy.zeros((4, 100))
    tot_fwhm_array = numpy.zeros((4, 100))
    for tot_val in range(100):
        for i in range(div):
            #for iy in range(divy):
                #indexes_chip = numpy.where((xH < (1024/div)*(i+1)) & (xH > (1024/div)*i) & (yH < (256/divy) * (iy + 1)) & (yH > (256/divy)*iy) & (tot == tot_val))
                indexes_chip = numpy.where((xH < (1024/div)*(i+1)) & (xH > (1024/div)*i) & (tot == tot_val))
                try:
                    val = plot_histogram(indexes_chip, 'correction', 0)
                    tot_delay_array[i, tot_val] = -(numpy.round(val[0]) + TIME_DELAY)
                    tot_fwhm_array[i, tot_val] = val[2]
                except ValueError:
                    pass
                
    print(tot_delay_array.shape)
    print(tot_delay_array)
    ax[0, 0].plot(tot_delay_array[0])
    ax[0, 0].plot(tot_delay_array[1])
    ax[0, 0].plot(tot_delay_array[2])
    ax[0, 0].plot(tot_delay_array[3])
    ax[1, 0].plot(tot_fwhm_array[0])
    ax[1, 0].plot(tot_fwhm_array[1])
    ax[1, 0].plot(tot_fwhm_array[2])
    ax[1, 0].plot(tot_fwhm_array[3])
    #ax[0, 0].imshow(numpy.transpose(tot_delay_array), aspect='equal', origin='upper')
    #ax[1, 0].imshow(numpy.transpose(tot_fwhm_array), aspect='equal', origin='upper')
    #numpy.save('delay_array_'+str(div), delay_array)
    plt.show()


correction = False
#Plot chip correction
div = 16
if correction:
    fig, ax = plt.subplots(nrows=2, ncols=2)
    divy = int(div / 4)
    delay_array = numpy.zeros((div, divy))
    fwhm_array = numpy.zeros((div, divy))
    for i in range(div):
        for iy in range(divy):
            #print( (1024/div)*(i+1), (256/divy)*(iy+1) )
            #indexes_chip = numpy.where((xH < (1024/div)*(i+1)) & (xH > (1024/div)*i) & (yH < (1024/div)*(iy+1)) & (yH > (1024/div)*iy) )
            indexes_chip = numpy.where((xH < (1024/div)*(i+1)) & (xH > (1024/div)*i) & (yH < (256/divy) * (iy + 1)) & (yH > (256/divy)*iy))
            val = plot_histogram(indexes_chip, 'correction', 0)
            delay_array[i, iy] = -(numpy.round(val[0]) + TIME_DELAY)
            fwhm_array[i, iy] = val[2]
    ax[0, 0].imshow(numpy.transpose(delay_array), aspect='equal', origin='upper')
    ax[1, 0].imshow(numpy.transpose(fwhm_array), aspect='equal', origin='upper')
    numpy.save('delay_array_'+str(div), delay_array)

#delay_array = numpy.load('delay_array_'+str(div)+'.npy')
#correct_time2d(xH, yH, t, div, delay_array)
#fig, ax = plt.subplots()
#delay = ax.imshow(numpy.transpose(delay_array), aspect='equal', origin='upper')
#fig.colorbar(delay, ax = ax)


#Plot of the ratios
fig, ax = plt.subplots(nrows=2)
ax[0].plot(disparray, cle/max(cle), label='Correlated Data')
ax[0].plot(disparray, g2/max(g2), label='Correlated Data (g2)')
ax[0].plot(disparray, xT/max(xT), alpha=0.8, ls='-', color='red', lw=1, label='Total Data')
ax[1].plot(disparray, cRatio/max(cRatio), alpha=0.8, ls='-', color='green', lw=1, label='Ratio CLE')
ax[1].plot(disparray, cRatio2/max(cRatio2), alpha=0.8, ls='--', color='red', lw=1, label='Ratio g2')
ax[0].set_xlabel('Energy (eV)')
ax[0].set_ylabel('$\Gamma^{Loss}$ (A.U.)')
ax[1].set_ylabel('Coincidence Ratio', c='green')
ax[0].legend(loc='upper center')
ax[1].legend()
#ax[0].set_yscale("log")
#ax[1].set_yscale("log")
plt.tight_layout()


#Plot of the histograms
fig, ax = plt.subplots(nrows=3, ncols=2, sharex=False, figsize=(10, 10))
ax[0, 0].hist(t[indexes2], density=False, bins=tbin, range=(tmin, tmax), alpha=0.2, color='red', label='Channel 0')
ax[0, 0].hist(t[indexes12], density = False, bins=tbin, range=(tmin, tmax), alpha=0.2, color='blue', label='Channel 12')

ax[2, 1].hist(double_t, density = False, bins=tbin, range=(tmin, tmax), alpha=0.8, color='blue', label='Double electrons')

#ax[2, 0].hist(t[indexes_middle_time], density=False, bins=tbin, range=(tmin, tmax), alpha=0.2, color='green', label='Middle')
ax[2, 0].hist(t[indexes_begin_time], density=False, bins=tbin, range=(tmin, tmax), alpha=1.0, color='red', label='Beginning')
ax[2, 0].hist(t[indexes_end_time], density=False, bins=tbin, range=(tmin, tmax), alpha=0.6, color='blue', label='Ending')

plot_histogram(indexes12, 'channel2', 0)
#plot_histogram(indexes_tot, 'tot', 0)
plot_histogram(indexes_chip1, 'chip1', 0)
plot_histogram(indexes_chip2, 'chip2', 0)
plot_histogram(indexes_chip3, 'chip3', 0)
plot_histogram(indexes_chip4, 'chip4', 0)

ax[1, 0].hist(t[indexes_g2], bins=tbin, range=(tmin, tmax), alpha=0.2, color='green', label='g2')
ax[1, 0].hist(t[indexes_g2_correlated], bins=tbin, range=(tmin, tmax), alpha=0.2, color='magenta', label='g2_eff')

plot_histogram(indexes_g2_chip1, 'g2_chip1', 1)
plot_histogram(indexes_g2_chip2, 'g2_chip2', 1)
plot_histogram(indexes_g2_chip3, 'g2_chip3', 1)
plot_histogram(indexes_g2_chip4, 'g2_chip4', 1)

ax[0, 0].legend()
ax[0, 1].legend()
ax[1, 0].legend()
ax[1, 1].legend()
ax[2, 0].legend()
ax[1, 0].set_xlabel('Time delay (units of 260 ps)')

#Plot of the ToTs
fig, ax = plt.subplots(ncols = 2)
ax[0].hist(tot, bins=101, range=(0, 100))
ax[1].hist2d(t, tot, range=[[tmin, tmax], [0, 100]], bins=[tbin, 100])
ax[0].set_xlabel('Time over threshold (units of 1.5615 ns)')

#Histogram of the g2
#fig, ax = plt.subplots()
#ax.hist2d(xH, g2t, bins=[int(SPIM_PIXELS/2), 1001], range=[[0, SPIM_PIXELS], [-500, 500]])
#ax.set_xlabel('Energy (pixels)')
#ax.set_ylabel('Photon correlation delay (units of IsiBox 120 ps)')

#Histogram of the correlated/non-correlated g2
fig, ax = plt.subplots()
ax.hist(g2t[indexes_g2], bins=2001, range=(-1000, 1000), label='Correlated_region', alpha=0.2)
ax.hist(g2t, bins=2001, range=(-1000, 1000), label='Correlated', alpha=0.2)
ax.hist(g2_total, bins=2001, range=(-1000, 1000), label='Total', alpha=0.2)
ax.legend()
ax.set_xlabel('Photon correlation delay (units of IsiBox 120 ps)')

#Histogram of the CLE
fig, ax = plt.subplots(ncols=2, sharey=True)
ax[0].hist2d(xH, t, bins=[SPIM_PIXELS, tbin], range = [[0, SPIM_PIXELS], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.2))
#ax[1].hist2d(xH[indexes12], t[indexes12], bins=[SPIM_PIXELS, tbin], range = [[0, SPIM_PIXELS], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.2))
ax[1].hist2d(xH[indexes_g2], t[indexes_g2], bins=[SPIM_PIXELS, tbin], range = [[0, SPIM_PIXELS], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.2))
ax[0].set_xlabel('Energy (pixels)')
ax[1].set_xlabel('Energy (pixels)')
ax[0].set_ylabel('Time delay (units of 260 ps)')
plt.tight_layout()
plt.show()
#"""

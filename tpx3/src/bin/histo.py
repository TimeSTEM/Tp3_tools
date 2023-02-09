
# coding = utf-8 


import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from scipy.optimize import curve_fit

off = 0
disp = 1
SPIM_PIXELS = 1225
TIME_DELAY = 443 #Peak value of e-photon correlation
TIME_WIDTH = 25 #Peak interval of e-photon correlation
PP_FILTER = 500 #Max time between two photons (units of 120 ps)

def gaussian(x, mean, amplitude, sigma, offset):
    return offset + amplitude * numpy.exp( -(x-mean)**2 / (2*sigma ** 2))

disparray = numpy.linspace(off, disp*SPIM_PIXELS, SPIM_PIXELS)
t = numpy.fromfile("tH.txt", dtype='int16')
double_t = numpy.fromfile("double_tH.txt", dtype='int16')
tabs = numpy.fromfile("tabsH.txt", dtype='uint64')
g2t = numpy.fromfile("g2tH.txt", dtype='int16') #Photon-Photon time difference
g2_total = numpy.fromfile("isi_g2.txt", dtype='int64')
xT = numpy.loadtxt("spec.txt", delimiter=',')
x = numpy.loadtxt("cspec.txt", delimiter=',')
xH = numpy.fromfile("xH.txt", dtype='uint16')
yH = numpy.fromfile("yH.txt", dtype='uint16')
tot = numpy.fromfile("tot.txt", dtype='uint16')
channel = numpy.fromfile("channel.txt", dtype='uint8')

indexes2 = numpy.where((channel == 0))
indexes12 = numpy.where((channel == 12))
indexes_tot = numpy.where((tot >= 0))
indexes_chip1 = numpy.where((xH < 256))
indexes_chip2 = numpy.where((xH < 512) & (xH > 256))
indexes_chip3 = numpy.where((xH < 768) & (xH > 512))
indexes_chip4 = numpy.where((xH > 768))

factor = 10
indexes_begin_time = numpy.arange(0, int(len(tabs)/factor), 1)
indexes_middle_time = numpy.arange(int((factor/2-1)*len(tabs)/factor), int((factor/2)*len(tabs)/factor), 1)
indexes_end_time = numpy.arange(int((factor-1)*len(tabs)/factor), int(len(tabs)), 1)

#Getting CLE
indexes_cle = numpy.where((numpy.abs(t + TIME_DELAY) < TIME_WIDTH))
cle = numpy.zeros(SPIM_PIXELS)
for val in xH[indexes_cle]:
    cle[val] += 1


#Getting g2
indexes_g2 = numpy.where( (numpy.abs(g2t) < PP_FILTER))
indexes_g2_correlated = numpy.where( (numpy.abs(g2t) < PP_FILTER) & (numpy.abs(t+TIME_DELAY) < TIME_WIDTH))
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
plt.tight_layout()


#Plot of the histograms
fig, ax = plt.subplots(nrows=2, ncols=2, sharex=False, figsize=(8, 8))
ax[0, 0].hist(t[indexes2], density=False, bins=tbin, range=(tmin, tmax), alpha=0.2, color='red', label='Channel 0')
ax[0, 0].hist(t[indexes12], density = False, bins=tbin, range=(tmin, tmax), alpha=0.2, color='blue', label='Channel 12')

ax[1, 0].hist(t[indexes_g2], bins=tbin, range=(tmin, tmax), alpha=0.2, color='green', label='g2')
ax[1, 0].hist(t[indexes_g2_correlated], bins=tbin, range=(tmin, tmax), alpha=0.2, color='magenta', label='g2_eff')

ax[0, 1].hist(t[indexes_begin_time], density=False, bins=tbin, range=(tmin, tmax), alpha=1.0, color='red', label='Beginning')
ax[0, 1].hist(t[indexes_end_time], density=False, bins=tbin, range=(tmin, tmax), alpha=0.6, color='blue', label='Ending')

#2, 1
ax[1, 1].hist(double_t, density = False, bins=tbin, range=(tmin, tmax), alpha=0.8, color='blue', label='Double electrons')

ax[0, 0].legend()
ax[0, 1].legend()
ax[1, 0].legend()
ax[1, 1].legend()
ax[1, 0].set_xlabel('Time delay (units of 260 ps)')

#Plot of the ToTs
fig, ax = plt.subplots(ncols = 1, sharex = True, sharey = True)
#ax[0].hist(tot, bins=101, range=(0, 100))
ax.hist2d(t, tot, range=[[tmin, tmax], [0, 100]], bins=[tbin, 100])
ax.set_xlabel('Time over threshold (units of 1.5615 ns)')

#Histogram of the correlated/non-correlated g2
fig, ax = plt.subplots()
ax.hist(g2t[indexes_g2], bins=2001, range=(-1000, 1000), label='Correlated_region', alpha=0.2)
ax.hist(g2t, bins=2001, range=(-1000, 1000), label='Correlated', alpha=0.2)
ax.hist(g2_total, bins=2001, range=(-1000, 1000), label='Total', alpha=0.2)
ax.legend()
ax.set_xlabel('Photon correlation delay (units of IsiBox 120 ps)')

#Histogram of the CLE
fig, ax = plt.subplots(ncols=1, sharey=True)
ax.hist2d(xH, t, bins=[SPIM_PIXELS, tbin], range = [[0, SPIM_PIXELS], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.2))
#ax[1].hist2d(xH[indexes_g2], t[indexes_g2], bins=[SPIM_PIXELS, tbin], range = [[0, SPIM_PIXELS], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.2))
ax.set_xlabel('Energy (pixels)')
ax.set_ylabel('Time delay (units of 260 ps)')
plt.tight_layout()
plt.show()
#"""

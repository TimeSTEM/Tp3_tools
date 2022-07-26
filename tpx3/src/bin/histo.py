import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from matplotlib.widgets import Slider

off = 0
disp = 1
SPIM_PIXELS = 1041

disparray = numpy.linspace(off, disp*SPIM_PIXELS, SPIM_PIXELS)
t = numpy.fromfile("tH.txt", dtype='int64')
tabs = numpy.fromfile("tabsH.txt", dtype='uint64')
g2t = numpy.fromfile("g2tH.txt", dtype='int64')
xT = numpy.loadtxt("spec.txt", delimiter=',')
x = numpy.loadtxt("cspec.txt", delimiter=',')
xH = numpy.fromfile("xH.txt", dtype='uint32')
yH = numpy.fromfile("yH.txt", dtype='uint32')
tot = numpy.fromfile("tot.txt", dtype='uint16')
channel = numpy.fromfile("channel.txt", dtype='uint32')

indexes2 = numpy.where((channel == 2))
indexes12 = numpy.where((channel == 12))
indexes_position = numpy.where(xH < 256)
indexes_time = numpy.where(tabs < (max(tabs)+min(tabs))/2)

#yH = numpy.loadtxt("yH.txt", delimiter=',')
#indexes = numpy.where(xH<1025)[0]
#xH = xH[indexes]
#t = t[indexes]

#Getting CLE
indexes_cle = numpy.where((numpy.abs(t + 500) < 25))
cle = numpy.zeros(1041)
for val in xH[indexes_cle]:
    cle[val] += 1


#Getting g2
indexes_g2 = numpy.where( (numpy.abs(g2t) < 25))
#indexes_g2 = numpy.where( (numpy.abs(g2t) < 25) & (numpy.abs(t+500) < 100))
g2 = numpy.zeros(1041)
for val in xH[indexes_g2]:
    g2[val] += 1
unique, counts = numpy.unique(g2t, return_counts = True)
total = numpy.sum(counts)
g2_photons = numpy.sum(counts[1:])
print(f'Total number of correlations is {total}. From these, {g2_photons} fall into the g2 statistics, or {g2_photons * 100 /total}%.')

#cRatio = numpy.divide(x, xT)
cRatio = numpy.divide(cle, xT)
cRatio2 = numpy.divide(g2, xT)

#Bins
tmax = int(numpy.max(t)) 
tmin = int(numpy.min(t))
tbin = int((tmax - tmin)) + 1
tbin = int(tbin/6)
print(tmax, tmin, tbin)

fig, ax = plt.subplots()
ax2 = ax.twinx()
ax.plot(disparray, x, label='Correlated Data')
ax.plot(disparray, xT, alpha=0.8, ls='-', color='red', lw=1, label='Total Data')
ax2.plot(disparray, cRatio, alpha=0.8, ls='-', color='green', lw=1, label='Ratio CLE')
ax2.plot(disparray, cRatio2, alpha=0.8, ls='--', color='red', lw=1, label='Ratio g2')
ax.set_xlabel('Energy (eV)')
ax.set_ylabel('$\Gamma^{Loss}$ (A.U.)')
ax2.set_ylabel('Coincidence Ratio', c='green')
ax.legend(loc='upper center')
ax2.legend()
#ax2.set_yscale("log")
plt.tight_layout()

fig, ax = plt.subplots(nrows=3, sharex=True)
ax[0].hist(t[indexes2], bins=tbin, range=(tmin, tmax))
ax[1].hist(t[indexes12], bins=tbin, range=(tmin, tmax))
ax[2].hist(t[indexes_g2], bins=tbin, range=(tmin, tmax))
ax[1].set_xlabel('Time delay (units of 260 ps)')

fig, ax = plt.subplots()
ax.hist(tot, bins=25)
ax.set_xlabel('Time over threshold (units of 1.5615 ns)')

fig, ax = plt.subplots()
ax.hist2d(xH, g2t, bins=[int(SPIM_PIXELS/16), 1001], range=[[0, SPIM_PIXELS], [-500, 500]])
#ax.hist(g2t[indexes_position], bins=1000)
ax.set_xlabel('Energy (pixels)')
ax.set_ylabel('Photon correlation delay (units of IsiBox 120 ps)')

fig, ax = plt.subplots()
ax.hist(g2t, bins=1001, range=(-500, 500))
ax.set_xlabel('Photon correlation delay (units of IsiBox 120 ps)')

fig, ax = plt.subplots(ncols=2, sharey=True)
ax[0].hist2d(xH[indexes2], t[indexes2], bins=[SPIM_PIXELS, tbin], range = [[0, SPIM_PIXELS], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.2))
ax[1].hist2d(xH[indexes12], t[indexes12], bins=[SPIM_PIXELS, tbin], range = [[0, SPIM_PIXELS], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.2))
ax[0].set_xlabel('Energy (pixels)')
ax[1].set_xlabel('Energy (pixels)')
ax[0].set_ylabel('Time delay (units of 260 ps)')
plt.tight_layout()
plt.show()

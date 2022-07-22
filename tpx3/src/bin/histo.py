import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from matplotlib.widgets import Slider

off = 11.921
disp = 0.07
SPIM_PIXELS = 1041

disparray = numpy.linspace(off, disp*SPIM_PIXELS, SPIM_PIXELS)
t = numpy.fromfile("tH.txt", dtype='int64')
xT = numpy.loadtxt("spec.txt", delimiter=',')
x = numpy.loadtxt("cspec.txt", delimiter=',')
xH = numpy.fromfile("xH.txt", dtype='uint32')
yH = numpy.fromfile("yH.txt", dtype='uint32')
tot = numpy.fromfile("tot.txt", dtype='uint16')
channel = numpy.fromfile("channel.txt", dtype='uint32')

indexes2 = numpy.where(channel == 2)
indexes12 = numpy.where(channel == 12)

#yH = numpy.loadtxt("yH.txt", delimiter=',')
#indexes = numpy.where(xH<1025)[0]
#xH = xH[indexes]
#t = t[indexes]

cRatio = numpy.divide(x, xT)

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
ax2.plot(disparray, cRatio, alpha=0.8, ls='-', color='green', lw=1, label='Ratio')
ax.set_xlabel('Energy (eV)')
ax.set_ylabel('$\Gamma^{Loss}$ (A.U.)')
ax2.set_ylabel('Coincidence Ratio', c='green')
ax.legend(loc='upper center')
ax2.legend()
#ax2.set_yscale("log")
plt.tight_layout()

fig, ax = plt.subplots(nrows=2, sharex=True)
ax[0].hist(t[indexes2], bins=tbin, range=(tmin, tmax))
ax[1].hist(t[indexes12], bins=tbin, range=(tmin, tmax))
ax[1].set_xlabel('Time delay (units of 260 ps)')

fig, ax = plt.subplots()
ax.hist(tot, bins=25)
ax.set_xlabel('Time over threshold (units of 1.5615 ns)')

fig, ax = plt.subplots(ncols=2, sharey=True)
ax[0].hist2d(xH[indexes2], t[indexes2], bins=[SPIM_PIXELS, tbin], range = [[0, SPIM_PIXELS], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.2))
ax[1].hist2d(xH[indexes12], t[indexes12], bins=[SPIM_PIXELS, tbin], range = [[0, SPIM_PIXELS], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.2))
ax[0].set_xlabel('Energy (pixels)')
ax[1].set_xlabel('Energy (pixels)')
ax[0].set_ylabel('Time delay (units of 260 ps)')
plt.tight_layout()
plt.show()

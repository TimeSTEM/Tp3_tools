import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from matplotlib.widgets import Slider

off = 11.921
disp = 0.07
SPIM_PIXELS = 1041

disparray = numpy.linspace(off, disp*SPIM_PIXELS, SPIM_PIXELS)
t = numpy.loadtxt("tH.txt", delimiter=',')
xT = numpy.loadtxt("spec.txt", delimiter=',')
x = numpy.loadtxt("cspec.txt", delimiter=',')
x = numpy.reshape(x, (256, SPIM_PIXELS))
x = numpy.sum(x, axis=0)
xH = numpy.loadtxt("xH.txt", delimiter=',')
tot = numpy.loadtxt("tot.txt", delimiter=',')
#yH = numpy.loadtxt("yH.txt", delimiter=',')
#indexes = numpy.where(xH<1025)[0]
#xH = xH[indexes]
#t = t[indexes]

cRatio = numpy.divide(x, xT)

#Bins
tmax = int(numpy.max(t)) 
tmin = int(numpy.min(t))
tbin = int((tmax - tmin)) + 1
#tbin = int(tbin/2)
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

fig, ax = plt.subplots()
ax.hist(t, bins=tbin, range=(tmin, tmax))
ax.set_xlabel('Time delay (units of 1.5615 ns)')

fig, ax = plt.subplots()
ax.hist(tot, bins=25)
ax.set_xlabel('Time over threshold (units of 1.5615 ns)')

fig, ax = plt.subplots()
ax.hist2d(xH, t, bins=[SPIM_PIXELS, tbin], range = [[0, SPIM_PIXELS], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.2))
#ax.hist2d(xH*disp-off, t, bins=[1024, tbin], range = [[0*disp-off, 1024*disp-off], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.3))
ax.set_xlabel('Energy (pixels)')
ax.set_ylabel('Time delay (units of 1.5615 ns)')
plt.tight_layout()
plt.show()

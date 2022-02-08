import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from matplotlib.widgets import Slider

off = 11.921
disp = 0.08
disparray = numpy.linspace(off, disp*1024, 1024)
t = numpy.loadtxt("tH.txt", delimiter=',')
xT = numpy.loadtxt("spec.txt", delimiter=',')
x = numpy.loadtxt("cspec.txt", delimiter=',')
x = numpy.reshape(x, (256, 1024))
x = numpy.sum(x, axis=0)
xH = numpy.loadtxt("xH.txt", delimiter=',')
#yH = numpy.loadtxt("yH.txt", delimiter=',')
#indexes = numpy.where(xH<1025)[0]
#xH = xH[indexes]
#t = t[indexes]

cRatio = numpy.divide(x, xT)

#Bins
maxratio = int(numpy.max(t) / 1.5625)
minratio = int(numpy.min(t) / 1.5625)
tmax = maxratio * 1.5625 #int(numpy.max(t)) 
tmin = minratio * 1.5625 #int(numpy.min(t)) 
tbin = int((tmax - tmin)/(2*1.5625))
print(tmax, tmin)

fig, ax = plt.subplots(1, 2, dpi=180, sharex = False)
ax2 = ax[0].twinx()
ax[0].plot(disparray, x, label='Correlated Data')
ax[0].plot(disparray, xT, alpha=0.8, ls='--', color='red', lw=2, label='Total Data')
ax2.scatter(disparray, cRatio, c='green', s=5, marker='x', label='Ratio')
ax[1].hist(t, bins=tbin, range=(tmin, tmax))
#ax[2].hist(xH, bins=1024, range = (0, 1024))

ax[0].set_xlabel('Energy (eV)')
ax[0].set_ylabel('$\Gamma^{Loss}$ (A.U.)')
ax2.set_ylabel('Coincidence Ratio', c='green')
ax[0].legend(loc='upper center')

plt.tight_layout()
plt.savefig('coinc.svg')
plt.show()

fig, ax = plt.subplots(1, 1, dpi=180, sharex = False)
ax.hist2d(xH*disp-off, t, bins=[1024, tbin], range = [[0*disp-off, 1024*disp-off], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.6))
#ax.hist2d(xH*disp-off, t, bins=[160, tbin], range = [[-10, 10], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.7))
ax.set_xlabel('Energy (eV)')
ax.set_ylabel('Time delay (ns)')
plt.tight_layout()
plt.show()

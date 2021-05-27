import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from matplotlib.widgets import Slider

t = numpy.loadtxt("tH.txt", delimiter=',')
x = numpy.loadtxt("xH.txt", delimiter=',')
#y = numpy.loadtxt("yH.txt", delimiter=',')
cs = numpy.loadtxt("cs.txt", delimiter=',')
stot = numpy.loadtxt("stot.txt", delimiter=',')
print(numpy.mean(cs))
#indexes = (x>=0) & (x<=1024)

fig, ax = plt.subplots(2, 2, dpi=180)
plt.subplots_adjust(left=0.25, bottom=0.25)
ax[0, 0].hist2d(stot, cs, range=[[0, 600], [1, 18]], bins=[10, 17], norm=mcolors.PowerNorm(0.5), cmap='inferno')
ax[0, 1].hist(stot, bins=50, range=[0, 600])

ax[1, 0].plot(x)
ax[1, 1].hist(t)

#ax[0].set_xlabel('ToT Sum (A.U.)')
#ax[0].set_xlabel('Cluster Radius std (pixels)')
#ax[0].set_ylabel('Cluster Size')

#ax[1].set_xlabel('ToT Sum (A.U.)')
##ax[1].set_xlabel('Cluster Radius std (pixels)')
#ax[1].set_ylabel('Frequency')

#ax[0].hist(t[indexes], bins=25)
#ax[1].hist(x[indexes], bins=1024, range=(0, 1024))
#ax[1].hist2d(x[indexes], y[indexes], bins=[101, 51], range=[[0, 1024], [50, 225]])
#l = ax[1].axvspan(10, 12, color='red', alpha=0.25)

""""
axcolor = 'lightgoldenrodyellow'
axc = plt.axes([0.25, 0.1, 0.6, 0.03], facecolor = axcolor)
axint = plt.axes([0.25, 0.15, 0.6, 0.03], facecolor = axcolor)
sc = Slider(axc, 'Center', 0, 1024, valinit=0, valstep=1)
sint = Slider(axint, 'Int.', 0, 1024, valinit=0, valstep=1)

def update(val):
    xmax = sc.val + sint.val
    xmin = sc.val - sint.val
    indexes = (x>=xmin) & (x<=xmax)
    ax[0].cla();
    ax[0].hist(t[indexes], bins=25)
    tempPol = (l.get_xy())
    tempPol[:, 0] = [xmin, xmin, xmax, xmax, xmin]
    l.set_xy(tempPol)


sc.on_changed(update)
sint.on_changed(update)
"""

plt.tight_layout()
plt.show()

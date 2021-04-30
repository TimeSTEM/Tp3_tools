import numpy
import matplotlib.pyplot as plt
from matplotlib.widgets import Slider

t = numpy.loadtxt("tH.txt", delimiter=',')
x = numpy.loadtxt("xH.txt", delimiter=',')
indexes = (x>=0) & (x<=1024)

fig, ax = plt.subplots(1, 2)
plt.subplots_adjust(left=0.25, bottom=0.25)
ax[0].hist(t[indexes], bins=25)
ax[1].hist(x[indexes], bins=1024, range=(0, 1024))
l = ax[1].axvspan(10, 12, color='red', alpha=0.25)

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

plt.show()

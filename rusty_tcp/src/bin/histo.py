import numpy
import matplotlib.pyplot as plt
from matplotlib.widgets import Slider

xmin = 0
xmax = 1024

t = numpy.loadtxt("tH.txt", delimiter=',')
x = numpy.loadtxt("xH.txt", delimiter=',')
indexes = (x>=xmin) & (x<=xmax)

fig, ax = plt.subplots(1, 1)
plt.subplots_adjust(left=0.25, bottom=0.25)
ax.hist(t[indexes], bins=25)

axcolor = 'lightgoldenrodyellow'
axc = plt.axes([0.25, 0.1, 0.6, 0.03], facecolor = axcolor)
axint = plt.axes([0.25, 0.15, 0.6, 0.03], facecolor = axcolor)
sc = Slider(axc, 'Center', 0, 1024, valinit=0, valstep=1)
sint = Slider(axint, 'Int.', 0, 1024, valinit=0, valstep=1)

def update(val):
    xmax = sc.val + sint.val
    xmin = sc.val - sint.val
    indexes = (x>=xmin) & (x<=xmax)
    print(xmin, xmax, len(t[indexes]))
    ax.cla()
    ax.hist(t[indexes], bins=25)

sc.on_changed(update)
sint.on_changed(update)


ax.set_ylabel('E-Counts')
ax.set_xlabel('Time (s)')
plt.show()

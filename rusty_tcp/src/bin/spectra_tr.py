import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from matplotlib.widgets import Slider

minimum = 3
maximum = 19
step = 1

spectra = [numpy.loadtxt("TimeSpectral\\" + str(val), delimiter=',') for val in numpy.arange(minimum, maximum, step)]

fig, ax = plt.subplots(1, 1, dpi=180, sharex=True)
[ax.plot(spectrum, label = str(index)) for (index, spectrum) in enumerate(spectra)]
plt.legend(fontsize=4)
plt.show()

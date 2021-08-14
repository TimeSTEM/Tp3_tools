import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from matplotlib.widgets import Slider

minimum = 30
maximum = 195
step = 1

filename = "TimeSpectral\\196_31_1024"

my_file = numpy.loadtxt(filename, delimiter=',')
print(f'Number of spectra is {len(my_file) / 1024}. Filename is {filename}.')
number_spectra = int(len(my_file)/1024)

try:
    assert maximum<number_spectra
    spectra = [my_file[i*1024:(i+1)*1024] for i in numpy.arange(minimum, maximum, step)]
except:
    print('Using the entire output.')
    spectra = [my_file[i*1024:(i+1)*1024] for i in range(int(number_spectra/step))]


fig, ax = plt.subplots(1, 1, dpi=180, sharex=True)
[ax.plot(spectrum, label = str(index)) for (index, spectrum) in enumerate(spectra)]
plt.legend(fontsize=4)
plt.show()

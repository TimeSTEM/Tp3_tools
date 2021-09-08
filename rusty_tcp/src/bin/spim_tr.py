import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from matplotlib.animation import FuncAnimation
import os

xspim = yspim = 32

directory = "SpimTimeSpectral"

"""
for filename in os.listdir(directory):
    filename = os.path.join(directory, filename)
    if "30_1024" in filename and "counter" in filename:
        my_file = numpy.loadtxt(filename, delimiter = ',')
        full_spectra = my_file
        print("Full spectra counter found. Will be used to normalized other counters.")
"""

for filename in os.listdir(directory):

    filename = os.path.join(directory, filename)
    my_file = numpy.loadtxt(filename, delimiter=',')
    print(f'Number of spectral images is {len(my_file) / (xspim*yspim)}. Filename is {filename}.')
    number_spectra = int(len(my_file)/(xspim*yspim))

    
    spectra = [numpy.reshape(my_file[i*xspim*yspim:(i+1)*xspim*yspim], (xspim, yspim)) for i in range(number_spectra)]

    """
    if "counter" in filename:
        print("Found counter in the current filename. Using different analysis.")
        spectra = [numpy.divide(my_file, full_spectra)]
    """

    vmax = numpy.max(spectra)

    fig, ax = plt.subplots(1, 1, dpi=180, sharex=True, figsize=(8, 8))
    im = ax.imshow(spectra[0], vmax=numpy.max(spectra), vmin=numpy.min(spectra))

    def animate_func(i):
        im.set_data(spectra[i])
        return [im]

    anim = FuncAnimation(fig, animate_func, frames=len(spectra)-1, interval=100)
    anim.save(filename+".gif", fps = 5)
    plt.show()

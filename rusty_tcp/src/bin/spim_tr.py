import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
import os

xspim = yspim = 67

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

    
    spectra = [my_file[i*xspim*yspim:(i+1)*xspim*yspim] for i in range(number_spectra)]
    new_spim = numpy.reshape(spectra[0], (xspim, yspim))

    """
    if "counter" in filename:
        print("Found counter in the current filename. Using different analysis.")
        spectra = [numpy.divide(my_file, full_spectra)]
    """

    fig, ax = plt.subplots(1, 1, dpi=180, sharex=True)
    ax.imshow(new_spim)
    #[ax.plot(spectrum, label = str(index)) for (index, spectrum) in enumerate(spectra)]
    #plt.legend(fontsize=4)
    plt.show()

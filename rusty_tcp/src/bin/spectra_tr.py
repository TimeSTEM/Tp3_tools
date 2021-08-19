import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
import os

minimum = 1
maximum = 330
step = 100

directory = "TimeSpectral"
directory = "SpimTimeSpectral02"

for filename in os.listdir(directory):
    filename = os.path.join(directory, filename)
    if "30_1024" in filename and "counter" in filename:
        my_file = numpy.loadtxt(filename, delimiter = ',')
        full_spectra = my_file
        print("Full spectra counter found. Will be used to normalized other counters.")


for filename in os.listdir(directory):

    filename = os.path.join(directory, filename)
    my_file = numpy.loadtxt(filename, delimiter=',')
    print(f'Number of spectra is {len(my_file) / 1024}. Filename is {filename}.')
    number_spectra = int(len(my_file)/1024)

    fig, ax = plt.subplots(1, 1, dpi=180, sharex=True)
    
    spectra = [my_file[i*1024:(i+1)*1024] for i in numpy.linspace(0, number_spectra-2, 3, dtype=int)]

    if "counter" in filename:
        print("Found counter in the current filename. Using different analysis.")
        try:
            spectra = [numpy.divide(my_file, full_spectra)]
        except NameError:
            spectra = [numpy.divide(my_file, 1)]

    [ax.plot(spectrum, label = str(index)) for (index, spectrum) in enumerate(spectra)]
    ax.set_xlabel("Energy (pixels)")
    ax.set_ylabel("Counts")
    ax.set_yticklabels([])
    plt.legend(fontsize=4)
    plt.tight_layout()
    plt.show()

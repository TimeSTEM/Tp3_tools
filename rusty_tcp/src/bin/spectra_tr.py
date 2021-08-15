import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
import os

minimum = 0
maximum = 165
step = 1

directory = "TimeSpectral"

for filename in os.listdir(directory):

    filename = os.path.join(directory, filename)
    my_file = numpy.loadtxt(filename, delimiter=',')
    print(f'Number of spectra is {len(my_file) / 1024}. Filename is {filename}.')
    number_spectra = int(len(my_file)/1024)

    fig, ax = plt.subplots(1, 1, dpi=180, sharex=True)
    
    try:
        assert maximum<number_spectra
        spectra = [my_file[i*1024:(i+1)*1024] for i in numpy.arange(minimum, maximum, step)]
    except:
        print('Using the entire output.')
        spectra = [my_file[i*1024:(i+1)*1024] for i in range(int(number_spectra/step))]

    if "counter" in filename:
        print("Found counter in the current filename. Using different analysis.")
        spectra = [my_file]

    [ax.plot(spectrum, label = str(index)) for (index, spectrum) in enumerate(spectra)]
    #plt.legend(fontsize=4)
    plt.show()

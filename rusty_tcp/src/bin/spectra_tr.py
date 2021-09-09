import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
import os

directory = "SpimTimeSpectral_position/first"
directory = "SpimTimeSpectral_position/second"
directory = "SpimTimeSpectral_position/carbon"
directory = "SpimTimeSpectral_position/calcium"

#directory = "SpimTimeSpectral_position/complete"

counter_spectra = list()
counter_spectra_label = list()

spectra = list()
spectra_label = list()

for filename in os.listdir(directory):
    filename = os.path.join(directory, filename)
    if "30_1024" in filename and "counter" in filename:
        my_file = numpy.loadtxt(filename, delimiter = ',')
        full_spectra = my_file
        print("Full spectra counter found. Will be used to normalized other counters.")


for fignumber, filename in enumerate(os.listdir(directory)):

    filename = os.path.join(directory, filename)
    my_file = numpy.loadtxt(filename, delimiter=',')
    print(f'Number of spectra is {len(my_file) / 1024}. Filename is {filename}.')
    number_spectra = int(len(my_file)/1024)

    
    if "counter" in filename:
        try:
            temp = numpy.divide(my_file, full_spectra)
            spectra = [temp]
            counter_spectra.append(temp)
            counter_spectra_label.append(filename)
        except NameError:
            temp = numpy.divide(my_file, 1)
            counter_spectra.append(temp)
            counter_spectra_label.append(filename)
    else:
        [spectra.append(my_file[i*1024:(i+1)*1024]) for i in numpy.linspace(0, number_spectra, 7, dtype=int)]
        [spectra_label.append(filename) for i in numpy.linspace(0, number_spectra, 7, dtype=int)]


#Counter Plots
fig, ax = plt.subplots(1, 1, dpi=180, sharex=True)
ax.set_ylabel("Counts")
ax.set_xlabel("Time (A.U.)")
[ax.plot(spectrum, label = counter_spectra_label[index]) for (index, spectrum) in enumerate(counter_spectra)]
plt.legend(fontsize=6)
plt.savefig(directory+'/counter.svg')
#plt.show()


#Spectra Plots
fig, ax = plt.subplots(1, 1, dpi=180, sharex=True)
ax.set_ylabel("Counts")
ax.set_xlabel("Energy (pixels)")
[ax.plot(spectrum, label = spectra_label[index]) for (index, spectrum) in enumerate(spectra)]
plt.legend(fontsize=6)
plt.savefig(directory+'/spec.svg')
if "complete" in directory: plt.show()


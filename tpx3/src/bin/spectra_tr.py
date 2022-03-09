import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
import os

def my_func(directory, norm = None):

    counter_spectra = list()
    counter_spectra_label = list()
    spectra = list()
    spectra_label = list()

    off = -17.48
    disp = 0.439
    xx = numpy.linspace(off, off + disp*1024, 1024)

    pixel_time = 4
    slice_time = 400000
    pixels = 32
    spim_time = pixels * pixels * pixel_time
    how_many = slice_time / spim_time #how many HDFs per slice
    #pixel_bin = 2
    #eff_pb = pixel_bin*pixel_bin - 1

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
            xt = numpy.divide(numpy.arange(0, len(my_file)*pixel_time*how_many*1**2, pixel_time*how_many*1**2), 1000)
            try:
                temp = numpy.divide(my_file, norm)
                spectra = [temp]
                counter_spectra.append(temp)
                counter_spectra_label.append(filename)
            except TypeError:
                temp = numpy.divide(my_file, 1)
                counter_spectra.append(temp)
                counter_spectra_label.append(filename)
        else:
            [spectra.append(my_file[i*1024:(i+1)*1024]) for i in numpy.linspace(0, number_spectra, number_spectra, dtype=int)]
            [spectra_label.append(filename) for i in numpy.linspace(0, number_spectra, number_spectra, dtype=int)]


    #Counter Plots
    fig, ax = plt.subplots(1, 1, dpi=180, sharex=True)
    ax.set_ylabel("Intensity (A.U.)")
    ax.set_xlabel("Accumulated Time  per pixel (ms/pixel)")
    [ax.plot(xt, spectrum, label = counter_spectra_label[index]) for (index, spectrum) in enumerate(counter_spectra)]


    ax.set_yticks([])
    ax.set_xlim([0, 90])
    #ax.set_ylim([0, 0.014]) #Fig01
    #ax.set_ylim([0, 0.018]) #Fig02
    #plt.legend(fontsize=6)
    #plt.savefig(directory+'/counter.svg')
    plt.show()


    #Spectra Plots
    #fig, ax = plt.subplots(1, 1, dpi=180, sharex=True)
    #ax.set_ylabel("Normalized Intensity")
    #ax.set_xlabel("Energy (eV)")
    
    #[ax.plot(spectrum, label = spectra_label[index]) for (index, spectrum) in enumerate(spectra)]
    #for x in [0, 1, 2, 3, 4, 5, 6, 7, 17]:
    #    d = 0.00004
    #    if x>10:
    #        ax.plot(xx, spectra[x]/sum(spectra[x]) + d*8, lw=2, color='black', ls='dashed')
    #    else:
    #        ax.plot(xx, spectra[x]/sum(spectra[x]) + d*x, lw=2)
    
    #ax.set_xlim([-0.1, 80])
    #ax.set_xlim([280, 360])
    
    #ax.set_ylim([-0.0001, 0.018])
    #ax.set_ylim([-0.00001, 0.00043])
    
    #ax.set_yticks([])
    #plt.tight_layout()
    
    #plt.legend(fontsize=6)
    #plt.savefig(directory+'/spec.svg')
    #if "complete" in directory: plt.show()


dir_list = ['SpimTimeSpectral_position/first', 'SpimTimeSpectral_position/second', 'SpimTimeSpectral_position/carbon', 'SpimTimeSpectral_position/calcium']
dir_list2 = ['SpimTimeSpectral_position/complete']

for my_dir in dir_list2:
    for filename in os.listdir(my_dir):
        filename = os.path.join(my_dir, filename)
        if "counter" in filename:
            total_counter = numpy.loadtxt(filename, delimiter = ',')


for my_dir in dir_list:
    #my_func(my_dir, total_counter)
    my_func(my_dir, None)

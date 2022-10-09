import numpy
import matplotlib.pyplot as plt
import matplotlib.colors as mcolors
from scipy.optimize import curve_fit

off = 0
disp = 1
SPIM_PIXELS = 1041
TIME_DELAY = 625
TIME_WIDTH = 25

def gaussian(x, mean, amplitude, sigma, offset):
    return offset + amplitude * numpy.exp( -(x-mean)**2 / (2*sigma ** 2))

def correct_time(pos_array, time_array, div, corrections):
    assert len(corrections) == div
    for i in range(div):
        indexes_chip = numpy.where((pos_array < (1024/div)*(i+1)) & (pos_array > (1024/div)*i))
        time_array[indexes_chip] += int(corrections[i])

disparray = numpy.linspace(off, disp*SPIM_PIXELS, SPIM_PIXELS)
t = numpy.fromfile("tH.txt", dtype='int64')
tabs = numpy.fromfile("tabsH.txt", dtype='uint64')
g2t = numpy.fromfile("g2tH.txt", dtype='int64')
g2_total = numpy.fromfile("isi_g2.txt", dtype='int64')
xT = numpy.loadtxt("spec.txt", delimiter=',')
x = numpy.loadtxt("cspec.txt", delimiter=',')
xH = numpy.fromfile("xH.txt", dtype='uint32')
yH = numpy.fromfile("yH.txt", dtype='uint32')
tot = numpy.fromfile("tot.txt", dtype='uint16')
channel = numpy.fromfile("channel.txt", dtype='uint32')

#correct_time(xH, t, 16, numpy.asarray([0, -1, 0, 1, -4, -2, -2, -4, -1, 1, 2, 2, -5, -2, -1, -4], dtype='int64'))
#correct_time(xH, t, 32, numpy.asarray([0, 0, -1, -1, 0, 0, 1, 0, -6, -3, -3, -2, -2, -1, -2, -5, -2, 0, 1, 1, 2, 2, 2, 1, -7, -4, -2, -1, -1, -1, -2, -8], dtype='int64'))
#correct_time(xH, t, 64, numpy.asarray([8.0, 9.0, 9.0, 9.0, 8.0, 8.0, 8.0, 9.0, 8.0, 9.0, 9.0, 10.0, 10.0, 10.0, 10.0, 9.0, 0.0, 5.0, 5.0, 7.0, 6.0, 6.0, 6.0, 7.0, 7.0, 8.0, 7.0, 8.0, 7.0, 6.0, 4.0, 2.0, 3.0, 9.0, 9.0, 10.0, 11.0, 9.0, 9.0, 11.0, 12.0, 11.0, 11.0, 12.0, 12.0, 11.0, 12.0, 6.0, -2.0, 3.0, 5.0, 5.0, 7.0, 7.0, 8.0, 8.0, 8.0, 8.0, 8.0, 8.0, 7.0, 8.0, 3.0, -3.0], dtype='int64'))



indexes2 = numpy.where((channel == 0))
indexes12 = numpy.where((channel == 12))
indexes_chip1 = numpy.where((xH < 256))
indexes_chip2 = numpy.where((xH < 512) & (xH > 256))
indexes_chip3 = numpy.where((xH < 768) & (xH > 512))
indexes_chip4 = numpy.where((xH > 768))
#indexes_position = numpy.where(xH < 256)
#indexes_time = numpy.where(tabs > 3*(max(tabs)+min(tabs))/4)
#indexes_extra = numpy.where(xH > 70)


#Getting CLE
indexes_cle = numpy.where((numpy.abs(t + TIME_DELAY) < TIME_WIDTH))
cle = numpy.zeros(SPIM_PIXELS)
for val in xH[indexes_cle]:
    cle[val] += 1


#Getting g2
indexes_g2 = numpy.where( (numpy.abs(g2t) < TIME_WIDTH))
indexes_g2_correlated = numpy.where( (numpy.abs(g2t) < 25) & (numpy.abs(t+TIME_DELAY) < TIME_WIDTH))
indexes_g2_chip1 = numpy.where( (numpy.abs(g2t) < 25) & (xH < 256))
indexes_g2_chip2 = numpy.where( (numpy.abs(g2t) < 25) & (xH < 512) & (xH > 256))
indexes_g2_chip3 = numpy.where( (numpy.abs(g2t) < 25) & (xH < 768) & (xH > 512))
indexes_g2_chip4 = numpy.where( (numpy.abs(g2t) < 25) & (xH > 768))
g2 = numpy.zeros(1041)
for val in xH[indexes_g2_correlated]:
    g2[val] += 1
unique, counts = numpy.unique(g2t, return_counts = True)
total = numpy.sum(counts)
g2_total_photons = numpy.sum(counts[1:])
g2_photons = numpy.sum(g2)
print(f'Total number of correlations is {total}. From these, {g2_total_photons} fall into the g2 statistics, and only {g2_photons} fall into the g2 curve, or {g2_photons * 100 /total}%.')

#Performing the ratios
cRatio = numpy.divide(cle, xT)
cRatio2 = numpy.divide(g2, xT)

#Bins
tmax = int(numpy.max(t)) 
tmin = int(numpy.min(t))
tbin = int((tmax - tmin)) + 1
#tbin = int(tbin/6)
print(tmax, tmin, tbin)

#Plot of the ratios
fig, ax = plt.subplots(nrows=2)
ax[0].plot(disparray, cle/max(cle), label='Correlated Data')
ax[0].plot(disparray, g2/max(g2), label='Correlated Data (g2)')
ax[0].plot(disparray, xT/max(xT), alpha=0.8, ls='-', color='red', lw=1, label='Total Data')
ax[1].plot(disparray, cRatio/max(cRatio), alpha=0.8, ls='-', color='green', lw=1, label='Ratio CLE')
ax[1].plot(disparray, cRatio2/max(cRatio2), alpha=0.8, ls='--', color='red', lw=1, label='Ratio g2')
ax[0].set_xlabel('Energy (eV)')
ax[0].set_ylabel('$\Gamma^{Loss}$ (A.U.)')
ax[1].set_ylabel('Coincidence Ratio', c='green')
ax[0].legend(loc='upper center')
ax[1].legend()
#ax[0].set_yscale("log")
#ax[1].set_yscale("log")
plt.tight_layout()

def plot_histogram(indexes, label, axis):
    bin_heights, bin_borders, _ = ax[axis, 1].hist(t[indexes], density = True, bins=tbin, range=(tmin, tmax), alpha=0.2, label=label)
    bin_centers = bin_borders[:-1] + numpy.diff(bin_borders) / 2
    popt, _ = curve_fit(gaussian, bin_centers, bin_heights, p0 = [-633.0, 0.01, 5., 0.005], bounds=([-numpy.inf, 0, 0, 0], [numpy.inf, 0.1, 20, 0.01]))
    x_interval_for_fit = numpy.linspace(bin_borders[0], bin_borders[-1], 10000)
    print('Plotting for chip ' + label + f'. The values are {popt} and {numpy.round(popt[0], 0) + TIME_DELAY}')
    ax[axis, 1].plot(x_interval_for_fit, gaussian(x_interval_for_fit, *popt))
    #return numpy.round(popt[0]) + TIME_DELAY
    #ax[axis, 1].clear()
    return popt

#Plot of the histograms
fig, ax = plt.subplots(nrows=2, ncols=3, sharex=False)
ax[0, 0].hist(t[indexes2], bins=tbin, range=(tmin, tmax), alpha=0.2, color='red', label='Channel 0')
ax[0, 0].hist(t[indexes12], bins=tbin, range=(tmin, tmax), alpha=0.2, color='blue', label='Channel 12')

#plot_histogram(indexes_chip1, 'chip1', 0)
#plot_histogram(indexes_chip2, 'chip2', 0)
#plot_histogram(indexes_chip3, 'chip3', 0)
#plot_histogram(indexes_chip4, 'chip4', 0)

#temp_correction = list()
div = 64
divy = int(div / 4)
delay_array = numpy.zeros((div, divy))
fwhm_array = numpy.zeros((div, divy))
for i in range(div):
    for iy in range(divy):
        #print( (1024/div)*(i+1), (256/divy)*(iy+1) )
        #indexes_chip = numpy.where((xH < (1024/div)*(i+1)) & (xH > (1024/div)*i) & (yH < (1024/div)*(iy+1)) & (yH > (1024/div)*iy) )
        indexes_chip = numpy.where((xH < (1024/div)*(i+1)) & (xH > (1024/div)*i) & (yH < (256/divy) * (iy + 1)) & (yH > (256/divy)*iy))
        val = plot_histogram(indexes_chip, 'chip1', 0)
        #temp_correction.append(-val)
        delay_array[i, iy] = -(numpy.round(val[0]) + TIME_DELAY)
        fwhm_array[i, iy] = val[2]

ax[0, 2].imshow(numpy.transpose(delay_array), aspect='equal', origin='upper')
ax[1, 2].imshow(numpy.transpose(fwhm_array), aspect='equal', origin='upper')
#print(temp_correction)


ax[1, 0].hist(t[indexes_g2], bins=tbin, range=(tmin, tmax), alpha=0.2, color='green', label='g2')
ax[1, 0].hist(t[indexes_g2_correlated], bins=tbin, range=(tmin, tmax), alpha=0.2, color='magenta', label='g2_eff')

plot_histogram(indexes_g2_chip1, 'g2_chip1', 1)
plot_histogram(indexes_g2_chip2, 'g2_chip2', 1)
plot_histogram(indexes_g2_chip3, 'g2_chip3', 1)
plot_histogram(indexes_g2_chip4, 'g2_chip4', 1)

ax[0, 0].legend()
#ax[0, 1].legend()
ax[1, 0].legend()
ax[1, 1].legend()
ax[1, 0].set_xlabel('Time delay (units of 260 ps)')

#Plot of the ToTs
fig, ax = plt.subplots()
ax.hist(tot, bins=101, range=(0, 100))
ax.set_xlabel('Time over threshold (units of 1.5615 ns)')

#Histogram of the g2
#fig, ax = plt.subplots()
#ax.hist2d(xH, g2t, bins=[int(SPIM_PIXELS/2), 1001], range=[[0, SPIM_PIXELS], [-500, 500]])
#ax.set_xlabel('Energy (pixels)')
#ax.set_ylabel('Photon correlation delay (units of IsiBox 120 ps)')

#Histogram of the correlated/non-correlated g2
fig, ax = plt.subplots()
ax.hist(g2t[indexes_g2], bins=2001, range=(-1000, 1000), label='Correlated', alpha=0.2)
ax.hist(g2t, bins=2001, range=(-1000, 1000), label='Total', alpha=0.2)
ax.legend()
ax.set_xlabel('Photon correlation delay (units of IsiBox 120 ps)')

#Histogram of the CLE
fig, ax = plt.subplots(ncols=2, sharey=True)
ax[0].hist2d(xH, t, bins=[SPIM_PIXELS, tbin], range = [[0, SPIM_PIXELS], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.2))
#ax[1].hist2d(xH[indexes12], t[indexes12], bins=[SPIM_PIXELS, tbin], range = [[0, SPIM_PIXELS], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.2))
ax[1].hist2d(xH[indexes_g2], t[indexes_g2], bins=[SPIM_PIXELS, tbin], range = [[0, SPIM_PIXELS], [tmin, tmax]], cmap = 'viridis', norm = mcolors.PowerNorm(0.2))
ax[0].set_xlabel('Energy (pixels)')
ax[1].set_xlabel('Energy (pixels)')
ax[0].set_ylabel('Time delay (units of 260 ps)')
plt.tight_layout()
plt.show()

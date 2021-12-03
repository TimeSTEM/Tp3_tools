import numpy
import matplotlib.pyplot as plt
from matplotlib.pyplot import cm
from scipy.optimize import curve_fit

files = ['22-83-1694(95).npy', '22-99-2609(100).npy', '23-33-4108(100).npy', '23-58-5940(100).npy', '23-92-8001(100).npy', '24-29-10483(132).npy', '24-63-13792(132).npy', '24-92-16116(132).npy', '25-17-20115(132).npy', '25-53-25262(132).npy']
delays = [numpy.load(val) for val in files]
curs = [22.83, 22.99, 23.33, 23.58, 23.92, 24.29, 24.63, 24.92, 25.17, 25.53]
powers = [1.69, 2.60, 4.10, 5.94, 8.00, 10.48, 13.79, 16.11, 20.11, 25.26]
colors = ['b', 'g', 'r', 'c', 'm', 'y', 'k', 'b', 'b', 'b']
colors = iter(cm.rainbow(numpy.linspace(0, 1, 10)))

ms = list()
ss = list()
time = numpy.linspace(1800e-3, 2000e-3, 128)

def func(x, mu, sigma):
    return 1/(sigma*numpy.sqrt(2*numpy.pi)) * numpy.exp(-0.5*( (x-mu)/sigma )**2)

fig, ax = plt.subplots(1, 1)
for i, delay in enumerate(delays):
    c = next(colors)
    n, bins, patches = ax.hist(delay, bins=128, density = True, stacked=True, alpha=0.2, color = c)
    popt, pcov = curve_fit(func, time, n)
    yfit = func(time, *popt)
    ms.append(popt[0]); ss.append(popt[1])
    ax.plot(time, yfit+0*i, color= c, label=str(curs[i]) + ' A (' + str(powers[i]) + ' mw)')
    ax.axvline(popt[0], alpha=0.1, linestyle='--', color=c)

    ax.set_ylabel('Normalized event counts')
ax.set_xlabel('Time Elapsed from TDC ($\mu$s)')
ax.set_xlim(1.8, 2.01)
plt.legend()
plt.savefig('p1.pdf')
plt.savefig('p1.svg')
ss = numpy.multiply(ss, 1e3)

fig, ax = plt.subplots(1, 1)
ax2 = ax.twinx()
ax.plot(curs, ms, color='blue')
ax.scatter(curs, ms, color='blue')

ax2.plot(curs, 2.355*ss, color='red')
ax2.scatter(curs, 2.355*ss, color='red')

ax.set_ylabel('Mean ($\mu$s)', color='blue')
ax2.set_ylabel('Full width at half maximum (ns)', color='red')
ax.set_xlabel('Laser Current (A)')
ax2.set_xlabel('Laser Current (A)')

#plt.show()
plt.savefig('p2.pdf')
plt.savefig('p2.svg')





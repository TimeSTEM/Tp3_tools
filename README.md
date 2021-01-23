# Tp3_tools
Repository containing support scripts and tools to aid development.

## RawAnalysis
This contain raw data analysis scripts. *open_raw.py* describes opens a set or a single .tpx3 file; *tpx2txt.cpp* is the counterpart in C/C++. Executable can be used in Powershell followed by file name. *temp.tpx3* is a temporary file that can be created using the TCPDummyServer. Results from python and C/C++ give us same results. Folder **FromTP3** is actual data from timepix3 containing a zero loss peak. Python scripts seems to perform better than the C/C++.

## TCPDummyServer
This is a TCPDummyServer. Executing in localhost puts it in hanging until a connection is stablished. You can either created a client python file or a simple telnet client (such as [PuTTY](https://www.chiark.greenend.org.uk/~sgtatham/putty/latest.html), for example), can do the trick. Comment file writing to prevent huge data files being created in *temp.tpx3*.

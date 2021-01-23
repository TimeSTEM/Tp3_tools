/****************************************************************

   Example C++ script to convert a raw data .tpx3 to text file
   it allows to convert all .tpx3 files in the same folder

		copyright JL@ Amsterdam Scientific Instruments B.V.
					  www.amscins.com
						03-07-2020
****************************************************************/

#include <stdio.h>
#include <io.h>
#include <iostream>
#include <fstream>
#include <iomanip>
#include <vector>
#include <cstring>

using namespace std;

void pathConvert_Single2Double(string& s){
	string::size_type pos = 0;
	while ((pos = s.find('\\', pos)) != string::npos){
		s.insert(pos, "\\");
		pos = pos + 2;
	}
}


int main(int argc, char *argv[])
{  
    
	char* userinput;
		
	string ext(".tpx3");				
	vector <string> filepaths;
	string filepath;
		
    if (argc == 1) {
        cout << "Usage: tpx3 raw data with FULL file path!!" << endl;
        return 0;
    }
    else {
        userinput = argv[1];
		
		string usertype(userinput);
		size_t found = usertype.find(ext);
		if(found != string::npos) {
			cout<<"Single file: "<< usertype << "  to be converted!" <<endl;
			filepaths.push_back(usertype);
		}else{
			
		_finddata_t fileDir;
		string dir = argv[1];
		pathConvert_Single2Double(dir);
		dir = dir.substr(0,dir.size()-1) +"\\";
		filepath = dir;
		dir = dir + "\\*.tpx3";
		//cout<<dir<<endl;
		
		long lfDir;

		if ((lfDir = _findfirst(dir.c_str(), &fileDir)) == -1l){
			printf("No file is found\n");
		return 0;
		}
		else {
			printf("List of files to be converted:\n");
			do {
				string filename(fileDir.name);
				filepaths.push_back("\\" + filename);
				//printf("%s\n", filepath); //fileDir.name
				cout<<filepaths.at(filepaths.size()-1)<<endl;
			} while (_findnext(lfDir, &fileDir) == 0);
		}
		_findclose(lfDir);				
				
		}
		
        
    }

	cout << "Your input: " << userinput << endl;
	system("PAUSE");
    
	
	for (int f = 0; f < filepaths.size(); f++){	
	string TPX_name = filepath + filepaths.at(f);
	string ofile = string(TPX_name);	
		
	ofile = ofile.substr(0,ofile.size()-5)+".txt";

    ofstream xy_file(ofile.c_str());   //Converted and saved txt file
    streampos begin, end;
    ifstream myfile(TPX_name.c_str(), ios::binary);
    unsigned short xpix, ypix, TOT, TOA, spidrTime;
    char chipnr, FTOA;
    int frameNr;
    int CTOA;
    int mode;
    unsigned long Timer_LSB32 = 0;
    unsigned long Timer_MSB16 = 0;
	unsigned long numofTDC=0;

    if (!myfile) {
        cout << "This file is not found!" << endl;
    }
    else {
        myfile.seekg(0, myfile.end);
        unsigned long long fileLength = myfile.tellg();
        myfile.seekg(0, myfile.beg);
        unsigned long long temp64;
        cout << "filesize: " << fileLength/(1024*1024) <<"MB" << endl;
        unsigned long long NumofPacket = fileLength / 8;
        unsigned long long* datapacket = new unsigned long long [NumofPacket];
        myfile.read((char*) datapacket, fileLength);
        myfile.close();
        char* HeaderBuffer = new char[8];
        unsigned long long temp;
		
        for (unsigned long long i = 0; i < NumofPacket; i++) {
            memcpy(HeaderBuffer, &datapacket[i], 8);  			
            if (HeaderBuffer[0] == 'T' && HeaderBuffer[1] == 'P' && HeaderBuffer[2] == 'X') {
                int size = ((0xff & HeaderBuffer[7]) << 8) | (0xff & HeaderBuffer[6]);
                chipnr = HeaderBuffer[4];
                mode = HeaderBuffer[5];
                for (int j = 0; j < size / 8; j++) {
                    temp = datapacket[i + j + 1];
                    int hdr = (int)(temp >> 56);
                    int packet = temp >> 60;
                    double coarsetime;
                    unsigned long tmpfine;
                    unsigned long trigtime_fine;
                    double time_unit, global_timestamp;
                    int trigger_counter;
                    unsigned long long int timemaster;
                    int heartbeatL, heartbeatM;
                    double TDC_timestamp;
                    double spidrTimens;
                    int x, y;
                    double TOTns;
                    double TOAns;
                    long dcol;
                    long spix;
                    long pix;
					
                    switch (packet)
                    {
                    case 0x6:      //TDC timestamp packet header

                        coarsetime = (temp >> 9) & 0x1FFFFFFFF; 
						coarsetime = coarsetime * (1/320e6);
                        tmpfine = (temp >> 5) & 0xF;						
                        TDC_timestamp = coarsetime + tmpfine*260e-12;					
                  
						//uncomment below to save TDC timestamps into the txt file                        
                        //xy_file << setprecision(15) << TDC_timestamp << endl;  
                        //cout<< "TDC timestamp: " << setprecision(15) << TDC_timestamp << endl;
						numofTDC=numofTDC+1;
                       
						if ((temp >> 56) == 0x6f) {
							//cout << "tdc1 rising edge is working" << endl; 
							xy_file << "t1r: "<<setprecision(15) << TDC_timestamp << endl;
						}
						if ((temp >> 56) == 0x6a) {
							//cout << "tdc1 falling edge is working" << endl;
							xy_file << "t1f: "<<setprecision(15) << TDC_timestamp << endl;
						}
						if ((temp >> 56) == 0x6e) {
							//cout << "tdc2 rising edge is working" << endl;
							xy_file << "t2r: "<<setprecision(15) << TDC_timestamp << endl;
						}
						if ((temp >> 56) == 0x6b) {
							//cout << "tdc2 falling edge is working" << endl;
							xy_file << "t2f: "<<setprecision(15) << TDC_timestamp << endl;							
						}
						
                        break;

                    case 0xb:		//Chip data: ToA and ToT timestamp packet, x, y

                        spidrTime = (unsigned short)(temp & 0xffff);
                        dcol = (temp & 0x0FE0000000000000L) >> 52;                                                                  
                        spix = (temp & 0x001F800000000000L) >> 45;                                                                    
                        pix = (temp & 0x0000700000000000L) >> 44;
                        x = (int)(dcol + pix / 4);
                        y = (int)(spix + (pix & 0x3));
                        TOA = (unsigned short)((temp >> (16 + 14)) & 0x3fff);   
                        TOT = (unsigned short)((temp >> (16 + 4)) & 0x3ff);	
                        FTOA = (unsigned char)((temp >> 16) & 0xf);
                        CTOA = (TOA << 4) | (~FTOA & 0xf);
                        spidrTimens = spidrTime * 25.0 * 16384.0;
                        TOAns = TOA * 25.0;
                        TOTns = TOT * 25.0;	
                        global_timestamp = spidrTimens + CTOA * (25.0 / 16);
						
						/************************************************************ 
						Condition is different for single Timepix3 chip or quad chips:						
						Single chip, using "int (Chipnr) +3"						
						Quad chips, using "int (Chipnr)"						
						************************************************************/						
                        switch (int (chipnr)) // for single chip, e.g. TPX3Cam;
                        {

                        case 0:
                            x += 260;
                            y = y;
                            break;

                        case 1:
                            x = 255 - x + 260;
                            y = 255 - y + 260;
                            break;

                        case 2:
                            x = 255 - x;
                            y = 255 - y + 260;
                            break;

                        case 3:
                            break;

                        default:
                            break;
							
                        }

						//uncomment below to save the chip data into the text file;
						xy_file << "c: "<<setprecision(15) << global_timestamp / 1E9 << "  " << TOTns << "  " << x << "  " << y << endl;   //toa, tot, x and y data can be saved into txt data
                        //cout<< "Chip-ToA: " << setprecision(15) << global_timestamp / 1E9 << "  ToT: " << TOTns << " x: " << x << " y: " << y << endl;
					    
                        break;

                    case 0x4:		//the global timestamps.

                        if (((temp >> 56) & 0xF) == 0x4) {
                            Timer_LSB32 = (temp >> 16) & 0xFFFFFFFF;
                        }
                        else if (((temp >> 56) & 0xF) == 0x5)
                        {
                            Timer_MSB16 = (temp >> 16) & 0xFFFF;
                            unsigned long long int timemaster;
                            timemaster = Timer_MSB16;
                            timemaster = (timemaster << 32) & 0xFFFF00000000;
                            timemaster = timemaster | Timer_LSB32;
                            int diff = (spidrTime >> 14) - ((Timer_LSB32 >> 28) & 0x3);

                            if ((spidrTime >> 14) == ((Timer_LSB32 >> 28) & 0x3))
                            { 						
                            }
                            else {                               
                                Timer_MSB16 = Timer_MSB16 - diff;
                            }  
							//uncomment below to save the global timestamps into the text file;
                         //xy_file << " Global time: " << setprecision(15) << timemaster * 25e-9 << endl;  //global timestamps can be saved into text file
                        }

                        break;

                    default:
                        break;
                    }

                }
                i += (size / 8);
                printf("i : %lld\r", i);
            }
        }

        delete  [] HeaderBuffer;
        delete [] datapacket;
    }
	cout<<"the number of TDCs: "<<numofTDC<<endl;
    xy_file.close();
    cout << filepaths.at(f) <<": converted! " << endl;
	
	}

    return 0;
}

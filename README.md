[![DOI](https://zenodo.org/badge/DOI/10.5281/zenodo.6346261.svg)](https://doi.org/10.5281/zenodo.6346261)

# Tp3_tools
Repository containing the development of Timepix3 (CheeTah solution) for live measurements of EELS, including images, spectra, Chrono (sequency of spectra) and hyperspectral EELS. The crate is coded in [Rust](https://www.rust-lang.org/tools/install). An API has been developed in Python and used within [NionSwift](https://github.com/nion-software/nionswift). Please this is still under strong development so missing documentation could be an issue. Nonetheless, using

`cargo doc --open`

can explain most of the backbones of the crate, such as the available libraries and the data structures used. 

Within proper setup, running Timepix3 can be as simply as setting up the TDCs and the desired acquisition mode. In 3 lines:

```
let frame_tdc = PeriodicTdcRef::new(TdcType::TdcOneRisingEdge, &mut pack, None)?;
let np_tdc = NonPeriodicTdcRef::new(TdcType::TdcTwoRisingEdge, &mut pack, None)?;
speclib::run_spectrum(pack, ns, my_settings, frame_tdc, np_tdc, speclib::Live1D)?;
```

## Note
Fell free to contact us in order to help set up Timepix3 under live conditions or to simply ask questions.

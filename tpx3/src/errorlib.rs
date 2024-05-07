//!`errorlib` is a simply enumeration to control error handling and logging.
#[derive(Debug)]
pub enum Tp3ErrorKind {
    //From settings
    SetBin,
    SetByteDepth,
    SetCumul,
    SetMode,
    SetXSize,
    SetYSize,
    SetNoReadFile,
    SetNoWriteFile,

    //From TDC
    TdcNoReceived,
    TdcBadPeriod,
    TdcBadHighTime,
    TdcNotAscendingOrder,
    TdcZeroBytes,

    //Mode implementation
    MiscModeNotImplemented(u8),

    //From IO-based, such as external libraries (like json parser)
    IOGeneralError,
    IOCouldNotCreateFile,
    SerdeGeneralError,
    Utf8GeneralError,

    //Coincidence-related
    CoincidenceFolderAlreadyCreated,
    CoincidenceCantReadFile,

    //Read-packet related
    TimepixReadLoop,
    TimepixReadOver,

    //IsiBox
    IsiBoxAttempt(u8),
    IsiBoxCouldNotConnect,
    IsiBoxCouldNotSetParameters,
    IsiBoxCouldNotConfigure,
    IsiBoxCouldNotSync,

    //4D from mask
    STEM4DCouldNotSetMask,

    //Frame based
    FrameBasedModeHasNoTdc,

    //Time-resolved post-processing
    TROutOfBounds,
    TRFolderDoesNotExist,
    TRFolderNotCreated,
    TRScanOutofBounds,
    TRMinGreaterThanMax,
}

impl From<std::io::Error> for Tp3ErrorKind {
    fn from(e: std::io::Error) -> Tp3ErrorKind {
        match e.kind() {
            std::io::ErrorKind::NotFound => Tp3ErrorKind::IOCouldNotCreateFile,
            _ => Tp3ErrorKind::IOGeneralError,
        }
    }
}

impl From<std::str::Utf8Error> for Tp3ErrorKind {
    fn from(_: std::str::Utf8Error) -> Tp3ErrorKind {
        Tp3ErrorKind::Utf8GeneralError
    }
}

impl From<serde_json::Error> for Tp3ErrorKind {
    fn from(error: serde_json::Error) -> Tp3ErrorKind {
        println!("***Errorlib***: Serde general error is {:?}", error); 
        Tp3ErrorKind::SerdeGeneralError
    }
}


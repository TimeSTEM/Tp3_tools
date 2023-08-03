//!`errorlib` is a simply enumeration to control error handling and logging.
#[derive(Debug)]
pub enum Tp3ErrorKind {
    SetBin,
    SetByteDepth,
    SetCumul,
    SetMode,
    SetXSize,
    SetYSize,
    SetNoReadFile,
    SetNoWriteFile,

    TdcNoReceived,
    TdcBadPeriod,
    TdcBadHighTime,
    TdcNotAscendingOrder,
    TdcZeroBytes,

    MiscModeNotImplemented(u8),

    IOGeneralError,
    SerdeGeneralError,

    CoincidenceFolderAlreadyCreated,
    CoincidenceCantReadFile,

    TimepixReadLoop,
    TimepixReadOver,

    IsiBoxAttempt(u8),
    IsiBoxCouldNotConnect,
    IsiBoxCouldNotSetParameters,
    IsiBoxCouldNotConfigure,
    IsiBoxCouldNotSync,

    STEM4DCouldNotSetMask,

    FrameBasedModeHasNoTdc,
}

impl From<std::io::Error> for Tp3ErrorKind {
    fn from(_: std::io::Error) -> Tp3ErrorKind {
        Tp3ErrorKind::IOGeneralError
    }
}

impl From<serde_json::Error> for Tp3ErrorKind {
    fn from(_: serde_json::Error) -> Tp3ErrorKind {
        Tp3ErrorKind::SerdeGeneralError
    }
}


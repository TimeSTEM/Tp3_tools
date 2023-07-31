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

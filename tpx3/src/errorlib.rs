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

    TimepixReadLoop,
    TimepixReadOver,

    IsiBoxAttempt(u8),
    IsiBoxCouldNotConnect,
    IsiBoxCouldNotSync,
}

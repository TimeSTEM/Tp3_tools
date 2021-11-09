#[derive(Debug)]
pub enum Tp3ErrorKind {
    SetBin,
    SetByteDepth,
    SetCumul,
    SetMode,
    SetXSize,
    SetYSize,
    SetNbSockets,
    SetNoReadFile,
    SetNoWriteFile,

    TdcNoReceived,
    TdcBadPeriod,
    TdcNotAscendingOrder,
    TdcZeroBytes,

    MiscModeNotImplemented(u8),

    TimepixReadLoop,
    TimepixReadOver,
}

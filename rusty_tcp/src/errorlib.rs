#[derive(Debug)]
pub enum Tp3ErrorKind {
    SetBin,
    SetByteDepth,
    SetCumul,
    SetMode,
    SetXSize,
    SetYSize,
    SetNbSockets,
    TdcNoReceived,
    TdcBadPeriod,
    TdcNotAscendingOrder,
    TdcZeroBytes,
}

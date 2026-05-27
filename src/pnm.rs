use std::io::BufRead;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PnmError {
    #[error("invalid kind: {0}")]
    InvalidKind(String),
    #[error("invalid data: {0}")]
    InvalidData(String),
}

pub struct PnmBuf<T: PnmKindTrait> {
    pub width: usize,
    pub height: usize,
    pub comments: Vec<String>,
    pub data: Vec<T::DataType>,
}

impl<T: PnmKindTrait> PnmBuf<T> {
    pub fn new(width: usize, height: usize, comments: Vec<String>, data: Vec<T::DataType>) -> Self {
        Self {
            width,
            height,
            comments,
            data,
        }
    }
}

pub enum Pnm {
    AsciiPbm(AsciiPbmBuf),
    AsciiPgm(AsciiPgmBuf),
    AsciiPpm(AsciiPpmBuf),
    BinaryPbm(BinaryPbmBuf),
    BinaryPgm(BinaryPgmBuf),
    BinaryPpm(BinaryPpmBuf),
}

impl<T: BufRead> TryFrom<T> for Pnm {
    type Error = PnmError;

    fn try_from(value: T) -> Result<Self, Self::Error> {
        let mut lines = value.lines();
        let mut comments = Vec::new();

        let mut if_comments_or_empty = |line: &str| -> bool {
            if line.is_empty() {
                true
            } else if line.starts_with("#") {
                comments.push(line[1..].trim().to_string());
                true
            } else {
                false
            }
        };

        let pnm_kind = loop {
            let line = match lines.next() {
                Some(Ok(line)) => line,
                Some(Err(e)) => {
                    return Err(PnmError::InvalidKind(format!(
                        "failed to read kind line: {}",
                        e
                    )));
                }
                None => return Err(PnmError::InvalidKind("no kind line".to_string())),
            };
            let trimmed = line.trim();

            if if_comments_or_empty(trimmed) {
                continue;
            }
            use std::str::FromStr;
            let pnm_kind = match PnmKind::from_str(trimmed) {
                Ok(kind) => kind,
                Err(e) => {
                    return Err(PnmError::InvalidKind(format!(
                        "failed to parse kind: {:?}",
                        e
                    )));
                }
            };
            break pnm_kind;
        };

        let (width, height) = loop {
            let line = match lines.next() {
                Some(Ok(line)) => line,
                Some(Err(e)) => {
                    return Err(PnmError::InvalidKind(format!(
                        "failed to read size line: {}",
                        e
                    )));
                }
                None => return Err(PnmError::InvalidKind("no size line".to_string())),
            };
            let trimmed = line.trim();

            if if_comments_or_empty(trimmed) {
                continue;
            }

            let [width, height]: [usize; 2] = trimmed
                .split_whitespace()
                .map(str::parse)
                .collect::<Result<Vec<_>, _>>()
                .ok()
                .and_then(|v| v.try_into().ok())
                .ok_or_else(|| PnmError::InvalidKind(format!("invalid size line: {}", trimmed)))?;
            break (width, height);
        };

        match pnm_kind {
            PnmKind::P1 => {
                let mut data = Vec::with_capacity(width * height);
                while let Some(Ok(line)) = lines.next() {
                    let trimmed = line.trim();
                    if if_comments_or_empty(trimmed) {
                        continue;
                    }
                    let chars = trimmed.split_whitespace();
                    for char in chars {
                        let Ok(bit) = char.parse::<u8>() else {
                            return Err(PnmError::InvalidData(format!(
                                "invalid data line: {}",
                                trimmed
                            )));
                        };
                        let bit: bool = match bit {
                            0 => false,
                            1 => true,
                            _ => {
                                return Err(PnmError::InvalidData(format!(
                                    "invalid data line: {}",
                                    trimmed
                                )));
                            }
                        };
                        data.push(bit);
                    }
                }
                Ok(Pnm::AsciiPbm(AsciiPbmBuf::new(
                    width, height, comments, data,
                )))
            }
            _ => {
                unimplemented!()
            }
        }
    }
}

// aliases
/// Pnm<PnmKind::P1>
/// ASCII PBM
pub type AsciiPbmBuf = PnmBuf<P1>;
/// Pnm<PnmKind::P2>
/// ASCII PGM
pub type AsciiPgmBuf = PnmBuf<P2>;
/// Pnm<PnmKind::P3>
/// ASCII PPM
pub type AsciiPpmBuf = PnmBuf<P3>;
/// Pnm<PnmKind::P4>
/// Binary PBM
pub type BinaryPbmBuf = PnmBuf<P4>;
/// Pnm<PnmKind::P5>
/// Binary PGM
pub type BinaryPgmBuf = PnmBuf<P5>;
/// Pnm<PnmKind::P6>
/// Binary PPM
pub type BinaryPpmBuf = PnmBuf<P6>;

pub trait PnmKindTrait {
    type DataType;
    const KIND: PnmKind;
}

pub struct P1 {}
impl PnmKindTrait for P1 {
    type DataType = bool;
    const KIND: PnmKind = PnmKind::P1;
}

pub struct P2 {}
impl PnmKindTrait for P2 {
    type DataType = u8;
    const KIND: PnmKind = PnmKind::P2;
}

pub struct P3 {}
impl PnmKindTrait for P3 {
    type DataType = crate::vec::Vec3<u8>;
    const KIND: PnmKind = PnmKind::P3;
}

pub struct P4 {}

impl PnmKindTrait for P4 {
    type DataType = bool;
    const KIND: PnmKind = PnmKind::P4;
}

pub struct P5 {}

impl PnmKindTrait for P5 {
    type DataType = u8;
    const KIND: PnmKind = PnmKind::P5;
}

pub struct P6 {}

impl PnmKindTrait for P6 {
    type DataType = crate::vec::Vec3<u8>;
    const KIND: PnmKind = PnmKind::P6;
}

/// https://www.mm2d.net/main/prog/c/image_io-01.html
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PnmKind {
    /// ASCII PNM
    P1,
    /// ASCII PNM
    P2,
    /// ASCII PNM
    P3,
    /// Binary PNM
    P4,
    /// Binary PNM
    P5,
    /// Binary PNM
    P6,
}

impl std::str::FromStr for PnmKind {
    type Err = PnmError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "P1" => Ok(PnmKind::P1),
            "P2" => Ok(PnmKind::P2),
            "P3" => Ok(PnmKind::P3),
            "P4" => Ok(PnmKind::P4),
            "P5" => Ok(PnmKind::P5),
            "P6" => Ok(PnmKind::P6),
            other => Err(PnmError::InvalidKind(format!(
                "invalid PNM kind: {}",
                other
            ))),
        }
    }
}

impl TryFrom<&str> for PnmKind {
    type Error = PnmError;

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        s.parse()
    }
}

impl TryFrom<String> for PnmKind {
    type Error = PnmError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.as_str().parse()
    }
}

impl std::fmt::Display for PnmKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "P{}", *self as u8 - PnmKind::P1 as u8 + 1)
    }
}

impl PnmKind {
    /// Returns `true` if the PNM kind is ASCII.
    pub const fn is_ascii(&self) -> bool {
        matches!(self, PnmKind::P1 | PnmKind::P2 | PnmKind::P3)
    }

    /// Returns `true` if the PNM kind is binary.
    pub const fn is_binary(&self) -> bool {
        matches!(self, PnmKind::P4 | PnmKind::P5 | PnmKind::P6)
    }

    /// Returns `true` if the PNM kind is PBM.
    pub const fn is_pbm(&self) -> bool {
        matches!(self, PnmKind::P1 | PnmKind::P4)
    }

    /// Returns `true` if the PNM kind is PGM.
    pub const fn is_pgm(&self) -> bool {
        matches!(self, PnmKind::P2 | PnmKind::P5)
    }

    /// Returns `true` if the PNM kind is PPM.
    pub const fn is_ppm(&self) -> bool {
        matches!(self, PnmKind::P3 | PnmKind::P6)
    }
}

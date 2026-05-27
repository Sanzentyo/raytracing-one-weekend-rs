use bitvec::prelude::{BitVec, Msb0};
use std::fmt::Debug;
use std::io::{BufRead, Read, Write};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PnmError {
    /// "P7" など P1〜P6 以外のマジックバイト
    #[error("unknown PNM kind: {0}")]
    UnknownKind(String),
    /// ヘッダ読み込み中に予期せず EOF
    #[error("unexpected EOF while reading header")]
    UnexpectedEof,
    /// width/height 行のパース失敗
    #[error("invalid header: {0}")]
    InvalidHeader(String),
    /// データ部に不正なピクセル値
    #[error("invalid pixel value: {0}")]
    InvalidPixel(String),
    /// データ部に # コメント（許容しない）
    #[error("comments are not allowed in the data section")]
    CommentInData,
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub type PnmResult<T> = Result<T, PnmError>;

/// データ形式(PBM/PGM/PPM)ごとのI/Oを集約するトレイト
pub trait PnmContent {
    type DataType: Debug;

    fn read_ascii<R: BufRead>(r: R, width: usize, height: usize) -> PnmResult<Self::DataType>;
    fn read_binary<R: Read>(r: R, width: usize, height: usize) -> PnmResult<Self::DataType>;
    fn write_ascii<W: Write>(
        data: &Self::DataType,
        w: &mut W,
        width: usize,
        height: usize,
    ) -> PnmResult<()>;
    fn write_binary<W: Write>(
        data: &Self::DataType,
        w: &mut W,
        width: usize,
        height: usize,
    ) -> PnmResult<()>;
}

/// Portable bitmap format (PBM)
/// 0: black, 1: white
pub struct Pbm;
/// Portable graymap format (PGM)
/// 0: black, 255: white
pub struct Pgm;
/// Portable pixmap format (PPM)
/// Red, Green, Blue: 0-255
pub struct Ppm;

impl PnmContent for Pbm {
    type DataType = BitVec<u8, Msb0>;

    /// Read a PBM file from ASCII format(P1)
    fn read_ascii<R: BufRead>(r: R, width: usize, height: usize) -> PnmResult<Self::DataType> {
        let mut data = BitVec::with_capacity(width * height);
        for line in r.lines() {
            let line = line?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('#') {
                return Err(PnmError::CommentInData);
            }
            for token in trimmed.split_whitespace() {
                match token {
                    "0" => data.push(false),
                    "1" => data.push(true),
                    other => {
                        return Err(PnmError::InvalidPixel(other.to_string()));
                    }
                }
            }
        }
        Ok(data)
    }

    /// Read a PBM file from binary format(P4)
    fn read_binary<R: Read>(mut r: R, width: usize, height: usize) -> PnmResult<Self::DataType> {
        let row_bytes = width.div_ceil(u8::BITS as usize);
        let mut data = BitVec::with_capacity(width * height);
        let mut buf = vec![0u8; row_bytes];
        for _ in 0..height {
            r.read_exact(&mut buf)?;
            for byte in &buf {
                for i in (0..8).rev() {
                    if data.len() < width * height {
                        data.push((byte >> i) & 1 == 1);
                    }
                }
            }
        }
        Ok(data)
    }

    /// Write a PBM file in ASCII format(P1)
    fn write_ascii<W: Write>(
        data: &Self::DataType,
        w: &mut W,
        width: usize,
        height: usize,
    ) -> PnmResult<()> {
        debug_assert!(
            data.len() == width * height,
            "data length must be equal to width * height"
        );
        for (i, bit) in data.iter().enumerate() {
            if i > 0 && i % width == 0 {
                writeln!(w)?;
            } else if i > 0 {
                write!(w, " ")?;
            }
            write!(w, "{}", u8::from(*bit))?;
        }
        writeln!(w)?;
        Ok(())
    }

    /// Write a PBM file in binary format(P4)
    fn write_binary<W: Write>(
        data: &Self::DataType,
        w: &mut W,
        width: usize,
        height: usize,
    ) -> PnmResult<()> {
        // P4: 行ごとにバイト境界までパディング
        let row_bytes = width.div_ceil(u8::BITS as usize);
        for row in 0..height {
            let row_bits = &data[row * width..(row * width + width).min(data.len())];
            let mut byte = 0u8;
            let mut bit_count = 0usize;
            for bit in row_bits {
                byte |= (u8::from(*bit)) << (7 - bit_count);
                bit_count += 1;
                if bit_count == 8 {
                    w.write_all(&[byte])?;
                    byte = 0;
                    bit_count = 0;
                }
            }
            // 行末パディング
            let written_bytes = width / 8;
            for _ in written_bytes..row_bytes {
                w.write_all(&[byte])?;
            }
        }
        Ok(())
    }
}

impl PnmContent for Pgm {
    type DataType = Vec<u8>;

    fn read_ascii<R: BufRead>(_r: R, _width: usize, _height: usize) -> PnmResult<Self::DataType> {
        unimplemented!()
    }

    fn read_binary<R: Read>(_r: R, _width: usize, _height: usize) -> PnmResult<Self::DataType> {
        unimplemented!()
    }

    fn write_ascii<W: Write>(
        _data: &Self::DataType,
        _w: &mut W,
        _width: usize,
        _height: usize,
    ) -> PnmResult<()> {
        unimplemented!()
    }

    fn write_binary<W: Write>(
        _data: &Self::DataType,
        _w: &mut W,
        _width: usize,
        _height: usize,
    ) -> PnmResult<()> {
        unimplemented!()
    }
}

impl PnmContent for Ppm {
    type DataType = Vec<crate::vec::Vec3<u8>>;

    fn read_ascii<R: BufRead>(_r: R, _width: usize, _height: usize) -> PnmResult<Self::DataType> {
        unimplemented!()
    }

    fn read_binary<R: Read>(_r: R, _width: usize, _height: usize) -> PnmResult<Self::DataType> {
        unimplemented!()
    }

    fn write_ascii<W: Write>(
        _data: &Self::DataType,
        _w: &mut W,
        _width: usize,
        _height: usize,
    ) -> PnmResult<()> {
        unimplemented!()
    }

    fn write_binary<W: Write>(
        _data: &Self::DataType,
        _w: &mut W,
        _width: usize,
        _height: usize,
    ) -> PnmResult<()> {
        unimplemented!()
    }
}

// ---------------------------------------------------------------------------
// PnmKindTrait: Content × Encoding の組み合わせ
// read_data / write_data は KIND.is_ascii() で振り分けるデフォルト実装
// ---------------------------------------------------------------------------

pub trait PnmKindTrait {
    type Content: PnmContent;
    const KIND: PnmKind;

    fn read_data<R: BufRead>(
        r: R,
        width: usize,
        height: usize,
    ) -> PnmResult<<Self::Content as PnmContent>::DataType> {
        if Self::KIND.is_ascii() {
            Self::Content::read_ascii(r, width, height)
        } else {
            Self::Content::read_binary(r, width, height)
        }
    }

    fn write_data<W: Write>(
        data: &<Self::Content as PnmContent>::DataType,
        w: &mut W,
        width: usize,
        height: usize,
    ) -> PnmResult<()> {
        if Self::KIND.is_ascii() {
            Self::Content::write_ascii(data, w, width, height)
        } else {
            Self::Content::write_binary(data, w, width, height)
        }
    }
}

// P1〜P6 は Content と KIND の宣言だけ
pub struct P1;
impl PnmKindTrait for P1 {
    type Content = Pbm;
    const KIND: PnmKind = PnmKind::P1;
}

pub struct P2;
impl PnmKindTrait for P2 {
    type Content = Pgm;
    const KIND: PnmKind = PnmKind::P2;
}

pub struct P3;
impl PnmKindTrait for P3 {
    type Content = Ppm;
    const KIND: PnmKind = PnmKind::P3;
}

pub struct P4;
impl PnmKindTrait for P4 {
    type Content = Pbm;
    const KIND: PnmKind = PnmKind::P4;
}

pub struct P5;
impl PnmKindTrait for P5 {
    type Content = Pgm;
    const KIND: PnmKind = PnmKind::P5;
}

pub struct P6;
impl PnmKindTrait for P6 {
    type Content = Ppm;
    const KIND: PnmKind = PnmKind::P6;
}

// ---------------------------------------------------------------------------
// PnmBuf と型エイリアス
// ---------------------------------------------------------------------------

pub struct PnmBuf<T: PnmKindTrait> {
    pub width: usize,
    pub height: usize,
    pub comments: Vec<String>,
    pub data: <T::Content as PnmContent>::DataType,
}

impl<T: PnmKindTrait> PnmBuf<T> {
    pub fn new(
        width: usize,
        height: usize,
        comments: Vec<String>,
        data: <T::Content as PnmContent>::DataType,
    ) -> Self {
        Self {
            width,
            height,
            comments,
            data,
        }
    }
}

/// ASCII PBM (P1)
pub type AsciiPbmBuf = PnmBuf<P1>;
/// ASCII PGM (P2)
pub type AsciiPgmBuf = PnmBuf<P2>;
/// ASCII PPM (P3)
pub type AsciiPpmBuf = PnmBuf<P3>;
/// Binary PBM (P4)
pub type BinaryPbmBuf = PnmBuf<P4>;
/// Binary PGM (P5)
pub type BinaryPgmBuf = PnmBuf<P5>;
/// Binary PPM (P6)
pub type BinaryPpmBuf = PnmBuf<P6>;

// ---------------------------------------------------------------------------
// Pnm: 実行時不定のラッパー enum
// ---------------------------------------------------------------------------

pub enum Pnm {
    AsciiPbm(AsciiPbmBuf),
    AsciiPgm(AsciiPgmBuf),
    AsciiPpm(AsciiPpmBuf),
    BinaryPbm(BinaryPbmBuf),
    BinaryPgm(BinaryPgmBuf),
    BinaryPpm(BinaryPpmBuf),
}

impl Pnm {
    pub fn from_reader<R: BufRead>(mut reader: R) -> PnmResult<Self> {
        let mut comments = Vec::new();

        let skip = |line: &str, comments: &mut Vec<String>| -> bool {
            if line.is_empty() {
                true
            } else if let Some(stripped) = line.strip_prefix('#') {
                comments.push(stripped.trim().to_string());
                true
            } else {
                false
            }
        };

        // magic bytes
        let pnm_kind = {
            let mut lines = (&mut reader).lines();
            loop {
                let line = lines.next().ok_or(PnmError::UnexpectedEof)??;
                let trimmed = line.trim().to_string();
                if skip(&trimmed, &mut comments) {
                    continue;
                }
                use std::str::FromStr;
                break PnmKind::from_str(&trimmed)?;
            }
        };

        // width height
        let (width, height) = {
            let mut lines = (&mut reader).lines();
            loop {
                let line = lines.next().ok_or(PnmError::UnexpectedEof)??;
                let trimmed = line.trim().to_string();
                if skip(&trimmed, &mut comments) {
                    continue;
                }
                let [w, h]: [usize; 2] = trimmed
                    .split_whitespace()
                    .map(str::parse)
                    .collect::<Result<Vec<_>, _>>()
                    .ok()
                    .and_then(|v| v.try_into().ok())
                    .ok_or_else(|| PnmError::InvalidHeader(format!("invalid size: {}", trimmed)))?;
                break (w, h);
            }
        };

        // データ読み込みは各 Kind に委譲
        match pnm_kind {
            PnmKind::P1 => Ok(Pnm::AsciiPbm(PnmBuf::new(
                width,
                height,
                comments,
                P1::read_data(&mut reader, width, height)?,
            ))),
            PnmKind::P2 => Ok(Pnm::AsciiPgm(PnmBuf::new(
                width,
                height,
                comments,
                P2::read_data(&mut reader, width, height)?,
            ))),
            PnmKind::P3 => Ok(Pnm::AsciiPpm(PnmBuf::new(
                width,
                height,
                comments,
                P3::read_data(&mut reader, width, height)?,
            ))),
            PnmKind::P4 => Ok(Pnm::BinaryPbm(PnmBuf::new(
                width,
                height,
                comments,
                P4::read_data(&mut reader, width, height)?,
            ))),
            PnmKind::P5 => Ok(Pnm::BinaryPgm(PnmBuf::new(
                width,
                height,
                comments,
                P5::read_data(&mut reader, width, height)?,
            ))),
            PnmKind::P6 => Ok(Pnm::BinaryPpm(PnmBuf::new(
                width,
                height,
                comments,
                P6::read_data(&mut reader, width, height)?,
            ))),
        }
    }
}

// ---------------------------------------------------------------------------
// PnmKind enum
// ---------------------------------------------------------------------------

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
            other => Err(PnmError::UnknownKind(other.to_string())),
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

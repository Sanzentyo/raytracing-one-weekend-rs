use bitvec::prelude::{BitVec, Msb0};
use byteorder::{BigEndian, ReadBytesExt, WriteBytesExt as _};
use std::fmt::Debug;
use std::io::{BufRead, Read, Write};
use std::iter::FusedIterator;
use std::num::NonZeroU16;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum EofAt {
    #[error("kind")]
    Kind,
    #[error("size")]
    Size,
    #[error("max value")]
    MaxVal,
}

#[derive(Error, Debug)]
pub enum PnmError {
    /// "P7" など P1〜P6 以外のマジックバイト
    #[error("unknown PNM kind: {0}")]
    UnknownKind(String),
    /// ヘッダ読み込み中に予期せず EOF
    #[error("unexpected EOF while reading {0}")]
    UnexpectedEof(EofAt),
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
    #[error("parse error: {0}")]
    ParseError(#[from] std::num::ParseIntError),
}

pub type PnmResult<T> = Result<T, PnmError>;

/// データ形式(PBM/PGM/PPM)ごとのI/Oを集約するトレイト
pub trait PnmContent {
    type DataType: Debug;
    type PixelType: Debug + Copy;
    type MaxVal: MaxValTrait;
    const EXTENSION: &'static str;

    fn read_ascii<R: BufRead>(
        r: R,
        maxval: Self::MaxVal,
        width: usize,
        height: usize,
    ) -> PnmResult<Self::DataType>;
    fn read_binary<R: Read>(
        r: R,
        maxval: Self::MaxVal,
        width: usize,
        height: usize,
    ) -> PnmResult<Self::DataType>;
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
/// 0: black, 255: white (8-bit)
/// 0: black, 65535: white (16-bit)
pub struct Pgm;
/// Portable pixmap format (PPM)
/// Red, Green, Blue: 0-255 (8-bit)
/// Red, Green, Blue: 0-65535 (16-bit)
pub struct Ppm;

/// Gray data enum for PGM files
#[derive(Debug, Clone, PartialEq)]
pub enum GrayData {
    U8(Vec<u8>),
    U16(Vec<u16>),
}

impl GrayData {
    pub fn len(&self) -> usize {
        match self {
            GrayData::U8(v) => v.len(),
            GrayData::U16(v) => v.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
/// RGB data for PPM files
#[derive(Debug, Clone, PartialEq)]
pub enum RgbData {
    U8(Vec<[u8; 3]>),
    U16(Vec<[u16; 3]>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PnmPixel {
    Bit(bool),
    Gray(u16),
    Rgb([u16; 3]),
}

impl From<bool> for PnmPixel {
    fn from(pixel: bool) -> Self {
        Self::Bit(pixel)
    }
}

impl From<u8> for PnmPixel {
    fn from(pixel: u8) -> Self {
        Self::Gray(u16::from(pixel))
    }
}

impl From<u16> for PnmPixel {
    fn from(pixel: u16) -> Self {
        Self::Gray(pixel)
    }
}

impl From<[u8; 3]> for PnmPixel {
    fn from([r, g, b]: [u8; 3]) -> Self {
        Self::Rgb([u16::from(r), u16::from(g), u16::from(b)])
    }
}

impl From<[u16; 3]> for PnmPixel {
    fn from(pixel: [u16; 3]) -> Self {
        Self::Rgb(pixel)
    }
}

#[derive(Debug, Clone)]
pub struct PbmPixels<'a> {
    inner: bitvec::slice::Iter<'a, u8, Msb0>,
}

impl Iterator for PbmPixels<'_> {
    type Item = bool;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|bit| *bit)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for PbmPixels<'_> {
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl FusedIterator for PbmPixels<'_> {}

#[derive(Debug, Clone)]
pub struct GrayPixels<'a> {
    inner: GrayPixelsInner<'a>,
}

#[derive(Debug, Clone)]
enum GrayPixelsInner<'a> {
    U8(std::slice::Iter<'a, u8>),
    U16(std::slice::Iter<'a, u16>),
}

impl Iterator for GrayPixels<'_> {
    type Item = u16;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            GrayPixelsInner::U8(iter) => iter.next().map(|&pixel| u16::from(pixel)),
            GrayPixelsInner::U16(iter) => iter.next().copied(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            GrayPixelsInner::U8(iter) => iter.size_hint(),
            GrayPixelsInner::U16(iter) => iter.size_hint(),
        }
    }
}

impl ExactSizeIterator for GrayPixels<'_> {
    fn len(&self) -> usize {
        match &self.inner {
            GrayPixelsInner::U8(iter) => iter.len(),
            GrayPixelsInner::U16(iter) => iter.len(),
        }
    }
}

impl FusedIterator for GrayPixels<'_> {}

#[derive(Debug, Clone)]
pub struct RgbPixels<'a> {
    inner: RgbPixelsInner<'a>,
}

#[derive(Debug, Clone)]
enum RgbPixelsInner<'a> {
    U8(std::slice::Iter<'a, [u8; 3]>),
    U16(std::slice::Iter<'a, [u16; 3]>),
}

impl Iterator for RgbPixels<'_> {
    type Item = [u16; 3];

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            RgbPixelsInner::U8(iter) => iter
                .next()
                .map(|&[r, g, b]| [u16::from(r), u16::from(g), u16::from(b)]),
            RgbPixelsInner::U16(iter) => iter.next().copied(),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            RgbPixelsInner::U8(iter) => iter.size_hint(),
            RgbPixelsInner::U16(iter) => iter.size_hint(),
        }
    }
}

impl ExactSizeIterator for RgbPixels<'_> {
    fn len(&self) -> usize {
        match &self.inner {
            RgbPixelsInner::U8(iter) => iter.len(),
            RgbPixelsInner::U16(iter) => iter.len(),
        }
    }
}

impl FusedIterator for RgbPixels<'_> {}

#[derive(Debug, Clone)]
pub struct PnmPixels<'a> {
    inner: PnmPixelsInner<'a>,
}

#[derive(Debug, Clone)]
enum PnmPixelsInner<'a> {
    Pbm(PbmPixels<'a>),
    Pgm(GrayPixels<'a>),
    Ppm(RgbPixels<'a>),
}

impl Iterator for PnmPixels<'_> {
    type Item = PnmPixel;

    fn next(&mut self) -> Option<Self::Item> {
        match &mut self.inner {
            PnmPixelsInner::Pbm(iter) => iter.next().map(PnmPixel::Bit),
            PnmPixelsInner::Pgm(iter) => iter.next().map(PnmPixel::Gray),
            PnmPixelsInner::Ppm(iter) => iter.next().map(PnmPixel::Rgb),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.inner {
            PnmPixelsInner::Pbm(iter) => iter.size_hint(),
            PnmPixelsInner::Pgm(iter) => iter.size_hint(),
            PnmPixelsInner::Ppm(iter) => iter.size_hint(),
        }
    }
}

impl ExactSizeIterator for PnmPixels<'_> {
    fn len(&self) -> usize {
        match &self.inner {
            PnmPixelsInner::Pbm(iter) => iter.len(),
            PnmPixelsInner::Pgm(iter) => iter.len(),
            PnmPixelsInner::Ppm(iter) => iter.len(),
        }
    }
}

impl FusedIterator for PnmPixels<'_> {}

#[derive(Debug, Clone)]
pub struct EnumeratePixels<I> {
    width: usize,
    inner: std::iter::Enumerate<I>,
}

impl<I> Iterator for EnumeratePixels<I>
where
    I: Iterator,
{
    type Item = (usize, usize, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        if self.width == 0 {
            return None;
        }
        self.inner.next().map(|(index, pixel)| {
            let x = index % self.width;
            let y = index / self.width;
            (x, y, pixel)
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<I> ExactSizeIterator for EnumeratePixels<I>
where
    I: ExactSizeIterator,
{
    fn len(&self) -> usize {
        self.inner.len()
    }
}

impl<I> FusedIterator for EnumeratePixels<I> where I: FusedIterator {}

pub trait PnmPixelData {
    type Pixel: Debug + Copy + PartialEq;

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    fn pixel_at(&self, index: usize) -> Option<Self::Pixel>;
    fn set_pixel_at(&mut self, index: usize, pixel: Self::Pixel) -> PnmResult<()>;

    fn validate_pixel(pixel: Self::Pixel, max_val: Option<NonZeroU16>) -> PnmResult<()>;
}

fn invalid_pixel(message: impl Into<String>) -> PnmError {
    PnmError::InvalidPixel(message.into())
}

fn validate_max_val(value: u16, max_val: Option<NonZeroU16>) -> PnmResult<()> {
    if let Some(max_val) = max_val
        && value > max_val.get()
    {
        return Err(invalid_pixel(format!(
            "pixel value {} exceeds max value {}",
            value, max_val
        )));
    }
    Ok(())
}

impl PnmPixelData for BitVec<u8, Msb0> {
    type Pixel = bool;

    fn len(&self) -> usize {
        self.len()
    }

    fn pixel_at(&self, index: usize) -> Option<Self::Pixel> {
        self.as_bitslice().get(index).map(|bit| *bit)
    }

    fn set_pixel_at(&mut self, index: usize, pixel: Self::Pixel) -> PnmResult<()> {
        if index >= self.len() {
            return Err(invalid_pixel(format!(
                "pixel index {} out of bounds",
                index
            )));
        }
        self.set(index, pixel);
        Ok(())
    }

    fn validate_pixel(_pixel: Self::Pixel, _max_val: Option<NonZeroU16>) -> PnmResult<()> {
        Ok(())
    }
}

impl GrayData {
    pub fn as_u8_slice(&self) -> Option<&[u8]> {
        match self {
            GrayData::U8(data) => Some(data),
            GrayData::U16(_) => None,
        }
    }

    pub fn as_u16_slice(&self) -> Option<&[u16]> {
        match self {
            GrayData::U8(_) => None,
            GrayData::U16(data) => Some(data),
        }
    }

    pub fn pixel_at(&self, index: usize) -> Option<u16> {
        <Self as PnmPixelData>::pixel_at(self, index)
    }

    pub fn set_pixel_at(&mut self, index: usize, pixel: u16) -> PnmResult<()> {
        <Self as PnmPixelData>::set_pixel_at(self, index, pixel)
    }

    pub fn pixels_u8(&self) -> Option<std::iter::Copied<std::slice::Iter<'_, u8>>> {
        self.as_u8_slice().map(|data| data.iter().copied())
    }

    pub fn pixels_u16(&self) -> Option<std::iter::Copied<std::slice::Iter<'_, u16>>> {
        self.as_u16_slice().map(|data| data.iter().copied())
    }

    pub fn normalized_pixels(&self) -> GrayPixels<'_> {
        GrayPixels {
            inner: match self {
                GrayData::U8(data) => GrayPixelsInner::U8(data.iter()),
                GrayData::U16(data) => GrayPixelsInner::U16(data.iter()),
            },
        }
    }
}

impl PnmPixelData for GrayData {
    type Pixel = u16;

    fn len(&self) -> usize {
        self.len()
    }

    fn pixel_at(&self, index: usize) -> Option<Self::Pixel> {
        match self {
            GrayData::U8(data) => data.get(index).map(|&pixel| u16::from(pixel)),
            GrayData::U16(data) => data.get(index).copied(),
        }
    }

    fn set_pixel_at(&mut self, index: usize, pixel: Self::Pixel) -> PnmResult<()> {
        match self {
            GrayData::U8(data) => {
                if pixel > u16::from(u8::MAX) {
                    return Err(invalid_pixel(format!(
                        "pixel value {} exceeds u8 range",
                        pixel
                    )));
                }
                let Some(slot) = data.get_mut(index) else {
                    return Err(invalid_pixel(format!(
                        "pixel index {} out of bounds",
                        index
                    )));
                };
                *slot = pixel as u8;
            }
            GrayData::U16(data) => {
                let Some(slot) = data.get_mut(index) else {
                    return Err(invalid_pixel(format!(
                        "pixel index {} out of bounds",
                        index
                    )));
                };
                *slot = pixel;
            }
        }
        Ok(())
    }

    fn validate_pixel(pixel: Self::Pixel, max_val: Option<NonZeroU16>) -> PnmResult<()> {
        validate_max_val(pixel, max_val)
    }
}

impl RgbData {
    pub fn len(&self) -> usize {
        match self {
            RgbData::U8(v) => v.len(),
            RgbData::U16(v) => v.len(),
        }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_u8_slice(&self) -> Option<&[[u8; 3]]> {
        match self {
            RgbData::U8(data) => Some(data),
            RgbData::U16(_) => None,
        }
    }

    pub fn as_u16_slice(&self) -> Option<&[[u16; 3]]> {
        match self {
            RgbData::U8(_) => None,
            RgbData::U16(data) => Some(data),
        }
    }

    pub fn pixel_at(&self, index: usize) -> Option<[u16; 3]> {
        <Self as PnmPixelData>::pixel_at(self, index)
    }

    pub fn set_pixel_at(&mut self, index: usize, pixel: [u16; 3]) -> PnmResult<()> {
        <Self as PnmPixelData>::set_pixel_at(self, index, pixel)
    }

    pub fn pixels_u8(&self) -> Option<std::iter::Copied<std::slice::Iter<'_, [u8; 3]>>> {
        self.as_u8_slice().map(|data| data.iter().copied())
    }

    pub fn pixels_u16(&self) -> Option<std::iter::Copied<std::slice::Iter<'_, [u16; 3]>>> {
        self.as_u16_slice().map(|data| data.iter().copied())
    }

    pub fn normalized_pixels(&self) -> RgbPixels<'_> {
        RgbPixels {
            inner: match self {
                RgbData::U8(data) => RgbPixelsInner::U8(data.iter()),
                RgbData::U16(data) => RgbPixelsInner::U16(data.iter()),
            },
        }
    }
}

impl PnmPixelData for RgbData {
    type Pixel = [u16; 3];

    fn len(&self) -> usize {
        self.len()
    }

    fn pixel_at(&self, index: usize) -> Option<Self::Pixel> {
        match self {
            RgbData::U8(data) => data
                .get(index)
                .map(|&[r, g, b]| [u16::from(r), u16::from(g), u16::from(b)]),
            RgbData::U16(data) => data.get(index).copied(),
        }
    }

    fn set_pixel_at(&mut self, index: usize, pixel: Self::Pixel) -> PnmResult<()> {
        match self {
            RgbData::U8(data) => {
                if let Some(channel) = pixel.iter().find(|&&channel| channel > u16::from(u8::MAX)) {
                    return Err(invalid_pixel(format!(
                        "pixel value {} exceeds u8 range",
                        channel
                    )));
                }
                let Some(slot) = data.get_mut(index) else {
                    return Err(invalid_pixel(format!(
                        "pixel index {} out of bounds",
                        index
                    )));
                };
                *slot = [pixel[0] as u8, pixel[1] as u8, pixel[2] as u8];
            }
            RgbData::U16(data) => {
                let Some(slot) = data.get_mut(index) else {
                    return Err(invalid_pixel(format!(
                        "pixel index {} out of bounds",
                        index
                    )));
                };
                *slot = pixel;
            }
        }
        Ok(())
    }

    fn validate_pixel(pixel: Self::Pixel, max_val: Option<NonZeroU16>) -> PnmResult<()> {
        pixel
            .into_iter()
            .try_for_each(|channel| validate_max_val(channel, max_val))
    }
}

impl PnmContent for Pbm {
    type DataType = BitVec<u8, Msb0>;
    type PixelType = bool;
    type MaxVal = ();
    const EXTENSION: &'static str = "pbm";

    /// Read a PBM file from ASCII format(P1)
    fn read_ascii<R: BufRead>(
        r: R,
        _maxval: Self::MaxVal,
        width: usize,
        height: usize,
    ) -> PnmResult<Self::DataType> {
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
        if data.len() != width * height {
            return Err(PnmError::InvalidPixel("data length mismatch".to_string()));
        }
        Ok(data)
    }

    /// Read a PBM file from binary format(P4)
    fn read_binary<R: Read>(
        mut r: R,
        _maxval: Self::MaxVal,
        width: usize,
        height: usize,
    ) -> PnmResult<Self::DataType> {
        let row_bytes = width.div_ceil(u8::BITS as usize);
        let mut data = BitVec::with_capacity(width * height);
        let mut buf = vec![0u8; row_bytes];
        for _ in 0..height {
            r.read_exact(&mut buf)?;
            for byte in &buf {
                for i in (0..u8::BITS).rev() {
                    // パディング分を無視
                    if data.len() < width * height {
                        data.push((byte >> i) & 1 == 1);
                    }
                }
            }
        }
        if data.len() != width * height {
            return Err(PnmError::InvalidPixel(format!(
                "data length mismatch: expected {}, got {}",
                width * height,
                data.len()
            )));
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
                byte |= (u8::from(*bit)) << (u8::BITS as usize - 1 - bit_count);
                bit_count += 1;
                if bit_count == u8::BITS as usize {
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
    type DataType = GrayData;
    type PixelType = u16;
    type MaxVal = NonZeroU16;
    const EXTENSION: &'static str = "pgm";

    /// Read a PGM file in ASCII format.(P5)
    fn read_ascii<R: BufRead>(
        r: R,
        maxval: Self::MaxVal,
        width: usize,
        height: usize,
    ) -> PnmResult<Self::DataType> {
        let data = match maxval.get() {
            1..=255 => {
                let mut data = Vec::with_capacity(width * height);
                for line in r.lines() {
                    let line = line?;

                    for number_str in line.split_whitespace() {
                        let num = number_str.parse::<u8>()?;
                        data.push(num);
                    }
                }
                GrayData::U8(data)
            }
            256..=65535 => {
                let mut data = Vec::with_capacity(width * height);
                for line in r.lines() {
                    let line = line?;
                    for number_str in line.split_whitespace() {
                        let num = number_str.parse::<u16>()?;
                        data.push(num);
                    }
                }
                GrayData::U16(data)
            }
            0 => unreachable!("maxval must be greater than 0"),
        };
        if data.len() != width * height {
            return Err(PnmError::InvalidPixel(format!(
                "data length mismatch: expected {}, got {}",
                width * height,
                data.len()
            )));
        }
        Ok(data)
    }

    /// Read a PGM file in binary format.(P2)
    fn read_binary<R: Read>(
        mut r: R,
        maxval: Self::MaxVal,
        width: usize,
        height: usize,
    ) -> PnmResult<Self::DataType> {
        let data = if maxval.get() <= u8::MAX as u16 {
            let mut data = Vec::with_capacity(width * height);
            r.read_to_end(&mut data)?;
            if data.len() != width * height {
                return Err(PnmError::InvalidPixel(format!(
                    "data length mismatch: expected {}, got {}",
                    width * height,
                    data.len()
                )));
            }
            GrayData::U8(data)
        } else {
            let mut data = Vec::with_capacity(width * height);
            r.read_u16_into::<BigEndian>(&mut data)?;
            if data.len() != width * height {
                return Err(PnmError::InvalidPixel(format!(
                    "data length mismatch: expected {}, got {}",
                    width * height,
                    data.len()
                )));
            }
            GrayData::U16(data)
        };
        Ok(data)
    }

    /// Write a PGM file in ASCII format.(P5)
    fn write_ascii<W: Write>(
        data: &Self::DataType,
        w: &mut W,
        width: usize,
        _height: usize,
    ) -> PnmResult<()> {
        match data {
            GrayData::U8(data) => {
                for (i, pixel) in data.iter().enumerate() {
                    if i % width == 0 {
                        writeln!(w)?;
                    }
                    write!(w, "{} ", pixel)?;
                }
                writeln!(w)?;
            }
            GrayData::U16(data) => {
                for (i, pixel) in data.iter().enumerate() {
                    if i % width == 0 {
                        writeln!(w)?;
                    }
                    write!(w, "{} ", pixel)?;
                }
                writeln!(w)?;
            }
        }
        Ok(())
    }

    /// Write a PGM file in binary format.(P2)
    fn write_binary<W: Write>(
        data: &Self::DataType,
        w: &mut W,
        _width: usize,
        _height: usize,
    ) -> PnmResult<()> {
        match data {
            GrayData::U8(data) => w.write_all(data)?,
            GrayData::U16(data) => data
                .iter()
                .try_for_each(|pixel| w.write_u16::<BigEndian>(*pixel))?,
        }
        Ok(())
    }
}

impl PnmContent for Ppm {
    type DataType = RgbData;
    type PixelType = [u16; 3];
    type MaxVal = NonZeroU16;
    const EXTENSION: &'static str = "ppm";

    /// Read a PPM file in ASCII format.(P3)
    fn read_ascii<R: BufRead>(
        mut r: R,
        maxval: Self::MaxVal,
        width: usize,
        height: usize,
    ) -> PnmResult<Self::DataType> {
        let data = if maxval.get() <= u8::MAX as u16 {
            let mut data = Vec::with_capacity(width * height);
            for _ in 0..width * height {
                let mut pixel = [0u8; 3];
                r.read_exact(&mut pixel)?;
                data.push(pixel);
            }
            if data.len() != width * height {
                return Err(PnmError::InvalidPixel(format!(
                    "data length mismatch: expected {}, got {}",
                    width * height,
                    data.len()
                )));
            }
            RgbData::U8(data)
        } else {
            let mut data = Vec::with_capacity(width * height);
            for _ in 0..width * height {
                let mut pixel = [0u16; 3];
                r.read_exact(bytemuck::cast_slice_mut(&mut pixel))?;
                data.push(pixel);
            }
            if data.len() != width * height {
                return Err(PnmError::InvalidPixel(format!(
                    "data length mismatch: expected {}, got {}",
                    width * height,
                    data.len()
                )));
            }
            RgbData::U16(data)
        };
        Ok(data)
    }

    fn read_binary<R: Read>(
        mut r: R,
        maxval: Self::MaxVal,
        width: usize,
        height: usize,
    ) -> PnmResult<Self::DataType> {
        let data = if maxval.get() <= u8::MAX as u16 {
            let mut data = vec![[0u8; 3]; width * height];
            r.read_exact(bytemuck::cast_slice_mut(&mut data))?;
            if data.len() != width * height {
                return Err(PnmError::InvalidPixel(format!(
                    "data length mismatch: expected {}, got {}",
                    width * height,
                    data.len()
                )));
            }
            RgbData::U8(data)
        } else {
            let mut data: Vec<[u16; 3]> = Vec::with_capacity(width * height);
            r.read_u16_into::<BigEndian>(bytemuck::cast_slice_mut(&mut data))?;
            if data.len() != width * height {
                return Err(PnmError::InvalidPixel(format!(
                    "data length mismatch: expected {}, got {}",
                    width * height,
                    data.len()
                )));
            }
            RgbData::U16(data)
        };
        Ok(data)
    }

    fn write_ascii<W: Write>(
        data: &Self::DataType,
        w: &mut W,
        width: usize,
        height: usize,
    ) -> PnmResult<()> {
        debug_assert!(data.len() == width * height);
        match data {
            RgbData::U8(data) => {
                for row in data.chunks(width) {
                    for (i, [r, g, b]) in row.iter().enumerate() {
                        write!(w, "{} {} {}", r, g, b)?;
                        if i < width - 1 {
                            write!(w, " ")?;
                        }
                    }
                    writeln!(w)?;
                }
            }
            RgbData::U16(data) => {
                for row in data.chunks(width) {
                    for (i, [r, g, b]) in row.iter().enumerate() {
                        write!(w, "{} {} {}", r, g, b)?;
                        if i < width - 1 {
                            write!(w, " ")?;
                        }
                    }
                    writeln!(w)?;
                }
            }
        }
        Ok(())
    }

    fn write_binary<W: Write>(
        data: &Self::DataType,
        w: &mut W,
        width: usize,
        height: usize,
    ) -> PnmResult<()> {
        debug_assert!(data.len() == width * height);
        match data {
            RgbData::U8(data) => w.write_all(bytemuck::cast_slice(data))?,
            RgbData::U16(data) => data
                .iter()
                .try_for_each(|p| w.write_all(bytemuck::cast_slice(p)))?,
        }
        Ok(())
    }
}
pub trait MaxValTrait {
    fn limit(&self) -> Option<NonZeroU16> {
        None
    }

    fn write_max_val(&self, w: &mut dyn Write) -> PnmResult<()>;
}
impl MaxValTrait for () {
    fn write_max_val(&self, _w: &mut dyn Write) -> PnmResult<()> {
        Ok(())
    }
}
impl MaxValTrait for NonZeroU16 {
    fn limit(&self) -> Option<NonZeroU16> {
        Some(*self)
    }

    fn write_max_val(&self, w: &mut dyn Write) -> PnmResult<()> {
        write!(w, " {}", self.get())?;
        Ok(())
    }
}

/// PnmKindTrait は Content × Encoding の組み合わせを表すトレイト
/// 具体的な Content と Encoding の組み合わせを表すために、PnmKind を使用する
pub trait PnmKindTrait {
    type Content: PnmContent;
    type MaxVal: MaxValTrait;

    const KIND: PnmKind;

    fn read_data<R: BufRead>(
        r: R,
        maxval: <Self::Content as PnmContent>::MaxVal,
        width: usize,
        height: usize,
    ) -> PnmResult<<Self::Content as PnmContent>::DataType> {
        if Self::KIND.is_ascii() {
            Self::Content::read_ascii(r, maxval, width, height)
        } else {
            Self::Content::read_binary(r, maxval, width, height)
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
    type MaxVal = ();
    const KIND: PnmKind = PnmKind::P1;
}

pub struct P2;
impl PnmKindTrait for P2 {
    type Content = Pgm;
    type MaxVal = NonZeroU16;
    const KIND: PnmKind = PnmKind::P2;
}

pub struct P3;
impl PnmKindTrait for P3 {
    type Content = Ppm;
    type MaxVal = NonZeroU16;
    const KIND: PnmKind = PnmKind::P3;
}

pub struct P4;
impl PnmKindTrait for P4 {
    type Content = Pbm;
    type MaxVal = ();
    const KIND: PnmKind = PnmKind::P4;
}

pub struct P5;
impl PnmKindTrait for P5 {
    type Content = Pgm;
    type MaxVal = NonZeroU16;
    const KIND: PnmKind = PnmKind::P5;
}

pub struct P6;
impl PnmKindTrait for P6 {
    type Content = Ppm;
    type MaxVal = NonZeroU16;
    const KIND: PnmKind = PnmKind::P6;
}

/// PNM バッファ
pub struct PnmBuf<T: PnmKindTrait> {
    pub width: usize,
    pub height: usize,
    pub max_val: T::MaxVal,
    pub comments: Vec<String>,
    pub data: <T::Content as PnmContent>::DataType,
}

impl<T: PnmKindTrait> PnmBuf<T> {
    pub fn new(
        width: usize,
        height: usize,
        max_val: T::MaxVal,
        comments: Vec<String>,
        data: <T::Content as PnmContent>::DataType,
    ) -> Self {
        Self {
            width,
            height,
            max_val,
            comments,
            data,
        }
    }

    pub const fn pixel_count(&self) -> usize {
        self.width * self.height
    }

    pub const fn contains_pixel(&self, x: usize, y: usize) -> bool {
        x < self.width && y < self.height
    }

    pub fn pixel_index(&self, x: usize, y: usize) -> Option<usize> {
        self.contains_pixel(x, y).then_some(y * self.width + x)
    }

    pub fn data(&self) -> &<T::Content as PnmContent>::DataType {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut <T::Content as PnmContent>::DataType {
        &mut self.data
    }
}

impl<T> PnmBuf<T>
where
    T: PnmKindTrait,
    <T::Content as PnmContent>::DataType: PnmPixelData,
{
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn pixel(
        &self,
        x: usize,
        y: usize,
    ) -> Option<<<T::Content as PnmContent>::DataType as PnmPixelData>::Pixel> {
        self.pixel_index(x, y)
            .and_then(|index| self.data.pixel_at(index))
    }

    pub fn set_pixel(
        &mut self,
        x: usize,
        y: usize,
        pixel: <<T::Content as PnmContent>::DataType as PnmPixelData>::Pixel,
    ) -> PnmResult<()> {
        let Some(index) = self.pixel_index(x, y) else {
            return Err(invalid_pixel(format!(
                "pixel coordinates ({}, {}) out of bounds for {}x{} image",
                x, y, self.width, self.height
            )));
        };
        <T::Content as PnmContent>::DataType::validate_pixel(pixel, self.max_val.limit())?;
        self.data.set_pixel_at(index, pixel)
    }
}

impl<T> PnmBuf<T>
where
    T: PnmKindTrait<Content = Pbm>,
{
    pub fn pixels(&self) -> PbmPixels<'_> {
        PbmPixels {
            inner: self.data.as_bitslice().iter(),
        }
    }

    pub fn enumerate_pixels(&self) -> EnumeratePixels<PbmPixels<'_>> {
        EnumeratePixels {
            width: self.width,
            inner: self.pixels().enumerate(),
        }
    }
}

impl<T> PnmBuf<T>
where
    T: PnmKindTrait<Content = Pgm>,
{
    pub fn gray_pixels_u8(&self) -> Option<std::iter::Copied<std::slice::Iter<'_, u8>>> {
        self.data.pixels_u8()
    }

    pub fn gray_pixels_u16(&self) -> Option<std::iter::Copied<std::slice::Iter<'_, u16>>> {
        self.data.pixels_u16()
    }

    pub fn normalized_gray_pixels(&self) -> GrayPixels<'_> {
        self.data.normalized_pixels()
    }

    pub fn enumerate_gray_pixels_u8(
        &self,
    ) -> Option<EnumeratePixels<std::iter::Copied<std::slice::Iter<'_, u8>>>> {
        self.gray_pixels_u8().map(|pixels| EnumeratePixels {
            width: self.width,
            inner: pixels.enumerate(),
        })
    }

    pub fn enumerate_gray_pixels_u16(
        &self,
    ) -> Option<EnumeratePixels<std::iter::Copied<std::slice::Iter<'_, u16>>>> {
        self.gray_pixels_u16().map(|pixels| EnumeratePixels {
            width: self.width,
            inner: pixels.enumerate(),
        })
    }

    pub fn enumerate_normalized_gray_pixels(&self) -> EnumeratePixels<GrayPixels<'_>> {
        EnumeratePixels {
            width: self.width,
            inner: self.normalized_gray_pixels().enumerate(),
        }
    }
}

impl<T> PnmBuf<T>
where
    T: PnmKindTrait<Content = Ppm>,
{
    pub fn rgb_pixels_u8(&self) -> Option<std::iter::Copied<std::slice::Iter<'_, [u8; 3]>>> {
        self.data.pixels_u8()
    }

    pub fn rgb_pixels_u16(&self) -> Option<std::iter::Copied<std::slice::Iter<'_, [u16; 3]>>> {
        self.data.pixels_u16()
    }

    pub fn normalized_rgb_pixels(&self) -> RgbPixels<'_> {
        self.data.normalized_pixels()
    }

    pub fn enumerate_rgb_pixels_u8(
        &self,
    ) -> Option<EnumeratePixels<std::iter::Copied<std::slice::Iter<'_, [u8; 3]>>>> {
        self.rgb_pixels_u8().map(|pixels| EnumeratePixels {
            width: self.width,
            inner: pixels.enumerate(),
        })
    }

    pub fn enumerate_rgb_pixels_u16(
        &self,
    ) -> Option<EnumeratePixels<std::iter::Copied<std::slice::Iter<'_, [u16; 3]>>>> {
        self.rgb_pixels_u16().map(|pixels| EnumeratePixels {
            width: self.width,
            inner: pixels.enumerate(),
        })
    }

    pub fn enumerate_normalized_rgb_pixels(&self) -> EnumeratePixels<RgbPixels<'_>> {
        EnumeratePixels {
            width: self.width,
            inner: self.normalized_rgb_pixels().enumerate(),
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

/// Portable AnyMap Format(PNM)
pub enum Pnm {
    AsciiPbm(AsciiPbmBuf),
    AsciiPgm(AsciiPgmBuf),
    AsciiPpm(AsciiPpmBuf),
    BinaryPbm(BinaryPbmBuf),
    BinaryPgm(BinaryPgmBuf),
    BinaryPpm(BinaryPpmBuf),
}

impl Pnm {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> PnmResult<Self> {
        let path = path.as_ref();
        let file = std::fs::File::open(path)?;
        let mut reader = std::io::BufReader::new(file);
        Self::from_reader(&mut reader)
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        let mut buf = Vec::new();
        self.write(&mut buf).unwrap();
        String::from_utf8(buf).unwrap()
    }

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> PnmResult<()> {
        let path = path.as_ref();
        let file = std::fs::File::create(path)?;
        let mut writer = std::io::BufWriter::new(file);
        self.write(&mut writer)?;
        Ok(())
    }

    /// pbm, pgm, ppm の拡張子を付けて保存する
    pub fn save_with_extension(&self, path: impl AsRef<std::path::Path>) -> PnmResult<()> {
        let path = path.as_ref();
        let file = std::fs::File::create(path.with_extension(self.extension()))?;
        let mut writer = std::io::BufWriter::new(file);
        self.write(&mut writer)?;
        Ok(())
    }

    pub fn from_reader<R: BufRead>(mut reader: R) -> PnmResult<Self> {
        let mut comments = Vec::new();

        let mut skip = |line: &str| -> bool {
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
                let line = lines.next().ok_or(PnmError::UnexpectedEof(EofAt::Kind))??;
                let trimmed = line.trim().to_string();
                if skip(&trimmed) {
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
                let line = lines.next().ok_or(PnmError::UnexpectedEof(EofAt::Size))??;
                let trimmed = line.trim().to_string();
                if skip(&trimmed) {
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

        let maxval = if pnm_kind.is_pbm() {
            None
        } else {
            let mut lines = (&mut reader).lines();
            loop {
                let line = lines
                    .next()
                    .ok_or(PnmError::UnexpectedEof(EofAt::MaxVal))??;
                if skip(line.trim()) {
                    continue;
                }
                if let Ok(value) = line.trim().parse::<NonZeroU16>() {
                    break Some(value);
                }
            }
        };

        // データ読み込みは各 Kind に委譲
        match pnm_kind {
            PnmKind::P1 => Ok(Pnm::AsciiPbm(PnmBuf::new(
                width,
                height,
                (),
                comments,
                P1::read_data(&mut reader, (), width, height)?,
            ))),
            PnmKind::P2 => Ok(Pnm::AsciiPgm(PnmBuf::new(
                width,
                height,
                maxval.unwrap(),
                comments,
                P2::read_data(&mut reader, maxval.unwrap(), width, height)?,
            ))),
            PnmKind::P3 => Ok(Pnm::AsciiPpm(PnmBuf::new(
                width,
                height,
                maxval.unwrap(),
                comments,
                P3::read_data(&mut reader, maxval.unwrap(), width, height)?,
            ))),
            PnmKind::P4 => Ok(Pnm::BinaryPbm(PnmBuf::new(
                width,
                height,
                (),
                comments,
                P4::read_data(&mut reader, (), width, height)?,
            ))),
            PnmKind::P5 => Ok(Pnm::BinaryPgm(PnmBuf::new(
                width,
                height,
                maxval.unwrap(),
                comments,
                P5::read_data(&mut reader, maxval.unwrap(), width, height)?,
            ))),
            PnmKind::P6 => Ok(Pnm::BinaryPpm(PnmBuf::new(
                width,
                height,
                maxval.unwrap(),
                comments,
                P6::read_data(&mut reader, maxval.unwrap(), width, height)?,
            ))),
        }
    }

    pub fn write<W: Write>(&self, w: &mut W) -> PnmResult<()> {
        // ヘッダーの書き込み
        writeln!(w, "{}", self.kind())?;
        // コメントの書き込み
        for comment in self.comments() {
            writeln!(w, "# {}", comment)?;
        }
        // width, heightの書き込み
        writeln!(w, "{} {}", self.width(), self.height())?;

        // max_valの書き込み
        self.writeln_max_val(w)?;

        // データの書き込み
        self.write_data(w)?;
        Ok(())
    }

    fn write_data<W: Write>(&self, w: &mut W) -> PnmResult<()> {
        match self {
            Pnm::AsciiPbm(buf) => P1::write_data(&buf.data, w, buf.width, buf.height),
            Pnm::AsciiPgm(buf) => P2::write_data(&buf.data, w, buf.width, buf.height),
            Pnm::AsciiPpm(buf) => P3::write_data(&buf.data, w, buf.width, buf.height),
            Pnm::BinaryPbm(buf) => P4::write_data(&buf.data, w, buf.width, buf.height),
            Pnm::BinaryPgm(buf) => P5::write_data(&buf.data, w, buf.width, buf.height),
            Pnm::BinaryPpm(buf) => P6::write_data(&buf.data, w, buf.width, buf.height),
        }
    }

    pub const fn kind(&self) -> PnmKind {
        match self {
            Pnm::AsciiPbm(_) => PnmKind::P1,
            Pnm::AsciiPgm(_) => PnmKind::P2,
            Pnm::AsciiPpm(_) => PnmKind::P3,
            Pnm::BinaryPbm(_) => PnmKind::P4,
            Pnm::BinaryPgm(_) => PnmKind::P5,
            Pnm::BinaryPpm(_) => PnmKind::P6,
        }
    }

    pub fn comments(&self) -> &[String] {
        match self {
            Pnm::AsciiPbm(buf) => &buf.comments,
            Pnm::AsciiPgm(buf) => &buf.comments,
            Pnm::AsciiPpm(buf) => &buf.comments,
            Pnm::BinaryPbm(buf) => &buf.comments,
            Pnm::BinaryPgm(buf) => &buf.comments,
            Pnm::BinaryPpm(buf) => &buf.comments,
        }
    }

    pub const fn width(&self) -> usize {
        match self {
            Pnm::AsciiPbm(buf) => buf.width,
            Pnm::AsciiPgm(buf) => buf.width,
            Pnm::AsciiPpm(buf) => buf.width,
            Pnm::BinaryPbm(buf) => buf.width,
            Pnm::BinaryPgm(buf) => buf.width,
            Pnm::BinaryPpm(buf) => buf.width,
        }
    }

    pub const fn height(&self) -> usize {
        match self {
            Pnm::AsciiPbm(buf) => buf.height,
            Pnm::AsciiPgm(buf) => buf.height,
            Pnm::AsciiPpm(buf) => buf.height,
            Pnm::BinaryPbm(buf) => buf.height,
            Pnm::BinaryPgm(buf) => buf.height,
            Pnm::BinaryPpm(buf) => buf.height,
        }
    }

    pub const fn pixel_count(&self) -> usize {
        self.width() * self.height()
    }

    pub fn len(&self) -> usize {
        match self {
            Pnm::AsciiPbm(buf) => buf.len(),
            Pnm::AsciiPgm(buf) => buf.len(),
            Pnm::AsciiPpm(buf) => buf.len(),
            Pnm::BinaryPbm(buf) => buf.len(),
            Pnm::BinaryPgm(buf) => buf.len(),
            Pnm::BinaryPpm(buf) => buf.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub const fn contains_pixel(&self, x: usize, y: usize) -> bool {
        x < self.width() && y < self.height()
    }

    pub fn pixel_index(&self, x: usize, y: usize) -> Option<usize> {
        self.contains_pixel(x, y).then_some(y * self.width() + x)
    }

    pub fn pixel(&self, x: usize, y: usize) -> Option<PnmPixel> {
        match self {
            Pnm::AsciiPbm(buf) => buf.pixel(x, y).map(PnmPixel::Bit),
            Pnm::AsciiPgm(buf) => buf.pixel(x, y).map(PnmPixel::Gray),
            Pnm::AsciiPpm(buf) => buf.pixel(x, y).map(PnmPixel::Rgb),
            Pnm::BinaryPbm(buf) => buf.pixel(x, y).map(PnmPixel::Bit),
            Pnm::BinaryPgm(buf) => buf.pixel(x, y).map(PnmPixel::Gray),
            Pnm::BinaryPpm(buf) => buf.pixel(x, y).map(PnmPixel::Rgb),
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, pixel: impl Into<PnmPixel>) -> PnmResult<()> {
        let pixel = pixel.into();
        let kind = self.kind();
        match (self, pixel) {
            (Pnm::AsciiPbm(buf), PnmPixel::Bit(pixel)) => buf.set_pixel(x, y, pixel),
            (Pnm::BinaryPbm(buf), PnmPixel::Bit(pixel)) => buf.set_pixel(x, y, pixel),
            (Pnm::AsciiPgm(buf), PnmPixel::Gray(pixel)) => buf.set_pixel(x, y, pixel),
            (Pnm::BinaryPgm(buf), PnmPixel::Gray(pixel)) => buf.set_pixel(x, y, pixel),
            (Pnm::AsciiPpm(buf), PnmPixel::Rgb(pixel)) => buf.set_pixel(x, y, pixel),
            (Pnm::BinaryPpm(buf), PnmPixel::Rgb(pixel)) => buf.set_pixel(x, y, pixel),
            _ => Err(invalid_pixel(format!(
                "pixel kind mismatch for {} image: {:?}",
                kind, pixel
            ))),
        }
    }

    pub fn pixels(&self) -> PnmPixels<'_> {
        PnmPixels {
            inner: match self {
                Pnm::AsciiPbm(buf) => PnmPixelsInner::Pbm(buf.pixels()),
                Pnm::AsciiPgm(buf) => PnmPixelsInner::Pgm(buf.normalized_gray_pixels()),
                Pnm::AsciiPpm(buf) => PnmPixelsInner::Ppm(buf.normalized_rgb_pixels()),
                Pnm::BinaryPbm(buf) => PnmPixelsInner::Pbm(buf.pixels()),
                Pnm::BinaryPgm(buf) => PnmPixelsInner::Pgm(buf.normalized_gray_pixels()),
                Pnm::BinaryPpm(buf) => PnmPixelsInner::Ppm(buf.normalized_rgb_pixels()),
            },
        }
    }

    pub fn enumerate_pixels(&self) -> EnumeratePixels<PnmPixels<'_>> {
        EnumeratePixels {
            width: self.width(),
            inner: self.pixels().enumerate(),
        }
    }

    pub fn max_val(&self) -> Option<NonZeroU16> {
        match self {
            Pnm::AsciiPbm(_) => None,
            Pnm::AsciiPgm(buf) => Some(buf.max_val),
            Pnm::AsciiPpm(buf) => Some(buf.max_val),
            Pnm::BinaryPbm(_) => None,
            Pnm::BinaryPgm(buf) => Some(buf.max_val),
            Pnm::BinaryPpm(buf) => Some(buf.max_val),
        }
    }

    pub fn write_max_val(&self, w: &mut dyn Write) -> PnmResult<()> {
        match self {
            Pnm::AsciiPbm(_) => ().write_max_val(w),
            Pnm::AsciiPgm(buf) => buf.max_val.write_max_val(w),
            Pnm::AsciiPpm(buf) => buf.max_val.write_max_val(w),
            Pnm::BinaryPbm(_) => ().write_max_val(w),
            Pnm::BinaryPgm(buf) => buf.max_val.write_max_val(w),
            Pnm::BinaryPpm(buf) => buf.max_val.write_max_val(w),
        }
    }

    pub fn writeln_max_val(&self, w: &mut dyn Write) -> PnmResult<()> {
        self.write_max_val(w)?;
        writeln!(w)?;
        Ok(())
    }

    pub const fn extension(&self) -> &'static str {
        self.kind().extension()
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

    pub const fn extension(&self) -> &'static str {
        match self {
            PnmKind::P1 | PnmKind::P4 => Pbm::EXTENSION,
            PnmKind::P2 | PnmKind::P5 => Pgm::EXTENSION,
            PnmKind::P3 | PnmKind::P6 => Ppm::EXTENSION,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn max_val(value: u16) -> NonZeroU16 {
        NonZeroU16::new(value).unwrap()
    }

    #[test]
    fn gray_data_access_normalizes_to_u16() {
        let mut data = GrayData::U8(vec![1, 2, 3]);

        assert_eq!(data.pixel_at(1), Some(2));
        assert_eq!(data.pixels_u8().unwrap().collect::<Vec<_>>(), vec![1, 2, 3]);
        assert_eq!(data.normalized_pixels().collect::<Vec<_>>(), vec![1, 2, 3]);

        data.set_pixel_at(0, 255).unwrap();
        assert_eq!(data.as_u8_slice(), Some([255, 2, 3].as_slice()));
        assert!(data.set_pixel_at(0, 256).is_err());
    }

    #[test]
    fn pnm_buf_accesses_pixels_by_coordinates() {
        let mut buf = AsciiPgmBuf::new(2, 2, max_val(10), vec![], GrayData::U8(vec![0, 1, 2, 3]));

        assert_eq!(buf.pixel_count(), 4);
        assert_eq!(buf.len(), 4);
        assert_eq!(buf.pixel_index(1, 1), Some(3));
        assert_eq!(buf.pixel(1, 1), Some(3));
        assert_eq!(buf.pixel(2, 0), None);
        assert_eq!(
            buf.enumerate_gray_pixels_u8().unwrap().collect::<Vec<_>>(),
            vec![(0, 0, 0), (1, 0, 1), (0, 1, 2), (1, 1, 3)]
        );

        buf.set_pixel(0, 1, 9).unwrap();
        assert_eq!(buf.pixel(0, 1), Some(9));
        assert!(buf.set_pixel(1, 1, 11).is_err());
        assert!(buf.set_pixel(2, 0, 1).is_err());
    }

    #[test]
    fn pnm_enum_accesses_pixels_across_formats() {
        let mut pnm = Pnm::BinaryPpm(BinaryPpmBuf::new(
            2,
            1,
            max_val(255),
            vec![],
            RgbData::U8(vec![[1, 2, 3], [4, 5, 6]]),
        ));

        assert_eq!(pnm.max_val(), Some(max_val(255)));
        assert_eq!(pnm.pixel(1, 0), Some(PnmPixel::Rgb([4, 5, 6])));
        assert_eq!(
            pnm.pixels().collect::<Vec<_>>(),
            vec![PnmPixel::Rgb([1, 2, 3]), PnmPixel::Rgb([4, 5, 6])]
        );

        pnm.set_pixel(0, 0, [7u16, 8, 9]).unwrap();
        assert_eq!(pnm.pixel(0, 0), Some(PnmPixel::Rgb([7, 8, 9])));
        assert!(pnm.set_pixel(0, 0, 1u16).is_err());
        assert!(pnm.set_pixel(0, 0, [256u16, 0, 0]).is_err());
    }

    #[test]
    fn pnm_max_val_is_available_for_pgm_and_ppm() {
        let pgm = Pnm::AsciiPgm(AsciiPgmBuf::new(
            1,
            1,
            max_val(31),
            vec![],
            GrayData::U8(vec![0]),
        ));
        let ppm = Pnm::AsciiPpm(AsciiPpmBuf::new(
            1,
            1,
            max_val(63),
            vec![],
            RgbData::U8(vec![[0, 0, 0]]),
        ));

        assert_eq!(pgm.max_val(), Some(max_val(31)));
        assert_eq!(ppm.max_val(), Some(max_val(63)));
    }
}

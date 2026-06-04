use crate::pnm::{AsciiPpmBuf, BinaryPpmBuf, PnmResult};
use crate::vec::Vec3;

pub type Color = Vec3<f64>;

pub trait SetColor {
    fn set_color(&mut self, x: usize, y: usize, color: Color) -> PnmResult<()>;
}

fn color_to_rgb8(color: Color) -> [u8; 3] {
    [
        (color.x.clamp(0.0, 1.0) * 255.999) as u8,
        (color.y.clamp(0.0, 1.0) * 255.999) as u8,
        (color.z.clamp(0.0, 1.0) * 255.999) as u8,
    ]
}

fn set_ppm_color(buf: &mut impl PpmSetPixel, x: usize, y: usize, color: Color) -> PnmResult<()> {
    let [r, g, b] = color_to_rgb8(color);
    buf.set_ppm_pixel(x, y, [u16::from(r), u16::from(g), u16::from(b)])
}

trait PpmSetPixel {
    fn set_ppm_pixel(&mut self, x: usize, y: usize, pixel: [u16; 3]) -> PnmResult<()>;
}

impl PpmSetPixel for AsciiPpmBuf {
    fn set_ppm_pixel(&mut self, x: usize, y: usize, pixel: [u16; 3]) -> PnmResult<()> {
        self.set_pixel(x, y, pixel)
    }
}

impl PpmSetPixel for BinaryPpmBuf {
    fn set_ppm_pixel(&mut self, x: usize, y: usize, pixel: [u16; 3]) -> PnmResult<()> {
        self.set_pixel(x, y, pixel)
    }
}

impl SetColor for AsciiPpmBuf {
    fn set_color(&mut self, x: usize, y: usize, color: Color) -> PnmResult<()> {
        set_ppm_color(self, x, y, color)
    }
}

impl SetColor for BinaryPpmBuf {
    fn set_color(&mut self, x: usize, y: usize, color: Color) -> PnmResult<()> {
        set_ppm_color(self, x, y, color)
    }
}

use std::num::NonZeroU16;

use anyhow::Result;
use raytracing_one_weekend_rs::pnm::{AsciiPpmBuf, Pnm};

fn to_u8(v: f64) -> u8 {
    (v * 255.999).round() as u8
}

fn main() -> Result<()> {
    const WIDTH: usize = 256;
    const HEIGHT: usize = 256;
    const OUTPUT_DIR: &str = "output";

    let mut image = AsciiPpmBuf::new(
        WIDTH,
        HEIGHT,
        NonZeroU16::new(u8::MAX as u16).unwrap(),
        vec![],
    );

    image
        .enumerate_rgb_pixels_mut_u8()
        .unwrap()
        .for_each(|(x, y, pixel)| {
            let r = x as f64 / (WIDTH as f64 - 1.0);
            let g = y as f64 / (HEIGHT as f64 - 1.0);
            const B: f64 = 0.0;

            *pixel = [to_u8(r), to_u8(g), to_u8(B)];
        });

    let pnm = Pnm::AsciiPpm(image);

    std::fs::create_dir_all(OUTPUT_DIR)?;
    pnm.save_with_extension(std::path::Path::new(OUTPUT_DIR).join("image"))?;

    Ok(())
}

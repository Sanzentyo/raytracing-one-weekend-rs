use std::num::NonZeroU16;

use anyhow::Result;
use raytracing_one_weekend_rs::pnm::{AsciiPpmBuf, Pnm, RgbData};

fn to_u8(v: f64) -> u8 {
    (v * 255.999).round() as u8
}

fn main() -> Result<()> {
    const WIDTH: usize = 256;
    const HEIGHT: usize = 256;
    const OUTPUT_DIR: &str = "output";

    let mut image_data = vec![[[0u8; 3]; WIDTH]; HEIGHT];

    image_data.iter_mut().enumerate().for_each(|(j, row)| {
        row.iter_mut().enumerate().for_each(|(i, pixel)| {
            let r = i as f64 / (WIDTH as f64 - 1.0);
            let b = j as f64 / (HEIGHT as f64 - 1.0);
            const G: f64 = 0.0;

            *pixel = [to_u8(r), to_u8(G), to_u8(b)];
        });
    });

    let pnm = Pnm::AsciiPpm(AsciiPpmBuf::new(
        WIDTH,
        HEIGHT,
        NonZeroU16::new(u8::MAX as u16).unwrap(),
        vec![],
        RgbData::U8(
            image_data
                .into_iter()
                .flatten()
                .collect::<Vec<_>>(),
        ),
    ));

    std::fs::create_dir_all(OUTPUT_DIR)?;
    pnm.save_with_extension(std::path::Path::new(OUTPUT_DIR).join("image"))?;

    Ok(())
}

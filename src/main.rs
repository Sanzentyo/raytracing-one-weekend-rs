use std::{borrow::Cow, num::NonZeroU16};

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use raytracing_one_weekend_rs::pnm::{AsciiPpmBuf, Pnm};
use tracing::info;

const fn to_u8(v: f64) -> u8 {
    (v * 255.999).round() as u8
}

fn main() -> Result<()> {
    const WIDTH: usize = 256;
    const HEIGHT: usize = 256;
    const OUTPUT_DIR: &str = "output";

    tracing_subscriber::fmt::init();

    let spinner_style = ProgressStyle::default_bar();

    let deps = (WIDTH * HEIGHT) as u64;
    
    let pb = ProgressBar::new(deps);
    pb.set_style(spinner_style);


    info!("Rendering image...");
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
            pb.inc(1);
        });
    pb.with_finish(indicatif::ProgressFinish::WithMessage(Cow::Borrowed("Image rendered.")));

    let pnm = Pnm::AsciiPpm(image);

    info!("Saving image...");
    std::fs::create_dir_all(OUTPUT_DIR)?;
    pnm.save_with_extension(std::path::Path::new(OUTPUT_DIR).join("image"))?;

    info!("Image saved {OUTPUT_DIR}/image.ppm");
    Ok(())
}

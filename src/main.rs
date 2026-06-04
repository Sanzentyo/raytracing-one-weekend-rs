use std::{borrow::Cow, num::NonZeroU16};

use anyhow::Result;
use indicatif::{ProgressBar, ProgressStyle};
use raytracing_one_weekend_rs::pnm::{AsciiPpmBuf, Pnm};
use raytracing_one_weekend_rs::ray::Ray;
use raytracing_one_weekend_rs::vec::Vec3;

use tracing::info;

const fn to_u8(v: Float) -> u8 {
    (v * 255.999).round() as u8
}

type Float = f64;

pub fn ray_color(ray: &Ray<Float>) -> Vec3<Float> {
    let unit_dir = ray.dir.normalize();
    let a = 0.5 * (unit_dir.y + 1.0);
    Vec3::one() * (1.0 - a) + Vec3::new(0.5, 0.7, 1.0) * a
}

pub fn hit_sphere(ray: &Ray<Float>, center: Vec3<Float>, radius: Float) -> bool {
    let oc = center - ray.orig;
    let a = ray.dir.dot(ray.dir);
    let b = -2.0 * ray.dir.dot(oc);
    let c = oc.dot(oc) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;
    discriminant >= 0.0
}

fn main() -> Result<()> {
    // image
    const IMAGE_WIDTH: usize = 400;
    const ASPECT_RATIO: Float = 16. / 9.;
    const IMAGE_HEIGHT: usize = (IMAGE_WIDTH as Float / ASPECT_RATIO) as usize;
    const {
        assert!(IMAGE_HEIGHT > 0);
    }

    // camera
    const FOCAL_LENGTH: Float = 1.0;
    const VIEWPORT_HEIGHT: Float = 2.0;
    const VIEWPORT_WIDTH: Float = VIEWPORT_HEIGHT * (IMAGE_WIDTH as Float / IMAGE_HEIGHT as Float);
    const CAMERA_CENTER: Vec3<Float> = Vec3::new(0.0, 0.0, 0.0);

    const VIEWPORT_U: Vec3<Float> = Vec3::new(VIEWPORT_WIDTH, 0.0, 0.0);
    const VIEWPORT_V: Vec3<Float> = Vec3::new(0.0, -VIEWPORT_HEIGHT, 0.0); // Y軸が逆なので逆にする

    let pixel_delta_u = VIEWPORT_U / IMAGE_WIDTH as Float;
    let pixel_delta_v = VIEWPORT_V / IMAGE_HEIGHT as Float;

    let viewport_upper_left =
        CAMERA_CENTER - Vec3::new(0., 0., FOCAL_LENGTH) - VIEWPORT_U / 2.0 - VIEWPORT_V / 2.0;
    let pixel00_loc = viewport_upper_left + (pixel_delta_u + pixel_delta_v) * 0.5;

    const OUTPUT_DIR: &str = "output";

    tracing_subscriber::fmt::init();

    let spinner_style = ProgressStyle::default_bar();

    let deps = (IMAGE_WIDTH * IMAGE_HEIGHT) as u64;

    let pb = ProgressBar::new(deps);
    pb.set_style(spinner_style);

    info!("Rendering image...");
    let mut image = AsciiPpmBuf::new(
        IMAGE_WIDTH,
        IMAGE_HEIGHT,
        NonZeroU16::new(u8::MAX as u16).unwrap(),
        vec![],
    );

    image
        .enumerate_rgb_pixels_mut_u8()
        .unwrap()
        .for_each(|(x, y, pixel)| {
            if x == 0 {
                info!("Rendering row {y}...");
            }

            let pixel_center =
                pixel00_loc + pixel_delta_u * x as Float + pixel_delta_v * y as Float;
            let ray_dir = pixel_center - CAMERA_CENTER;

            let ray = Ray::new(CAMERA_CENTER, ray_dir);

            let color = if hit_sphere(&ray, Vec3::new(0.0, 0.0, -1.0), 0.5) {
                Vec3::new(1.0, 0.0, 0.0)
            } else {
                ray_color(&ray)
            };
            *pixel = [to_u8(color.x), to_u8(color.y), to_u8(color.z)];
            pb.inc(1);
        });
    pb.with_finish(indicatif::ProgressFinish::WithMessage(Cow::Borrowed(
        "Image rendered.",
    )));

    let pnm = Pnm::AsciiPpm(image);

    info!("Saving image...");
    std::fs::create_dir_all(OUTPUT_DIR)?;
    pnm.save_with_extension(std::path::Path::new(OUTPUT_DIR).join("image"))?;

    info!("Image saved {OUTPUT_DIR}/image.ppm");
    Ok(())
}

use indicatif::{ProgressBar, ProgressStyle};
use tracing::info;

use crate::Float;
use crate::hittable::Hittable;
use crate::pnm::BinaryPpmBuf;
use crate::ray::Ray;
use crate::vec::Vec3;
use std::borrow::Cow;
use std::num::NonZeroU16;
use std::range::Range;

// リテラルを一々変換するのが流石に面倒なので諦める

pub struct Camera {
    #[allow(dead_code)]
    aspect_ratio: Float,        // Aspect ratio of the rendered image
    image_width: usize,         // Rendered image width
    image_height: usize,        // Rendered image height
    center: Vec3<Float>,        // Camera center
    pixel00_loc: Vec3<Float>,   // Location of pixel 0, 0
    pixel_delta_u: Vec3<Float>, // Offset to pixel to the right
    pixel_delta_v: Vec3<Float>, // Offset to pixel below
}

impl Camera {
    pub fn new(aspect_ratio: Float, image_width: usize) -> Self {
        let image_height = (image_width as Float / aspect_ratio).round() as usize;
        assert!(
            image_width > 0 && image_height > 0,
            "image width and height must be positive"
        );

        let center = Vec3::new(0.0, 0.0, 0.0);

        let focal_length = 1.0;
        let viewport_height = 2.0;
        let viewport_width = viewport_height * image_width as Float / image_height as Float;

        let viewport_u = Vec3::new(viewport_width, 0.0, 0.0);
        let viewport_v = Vec3::new(0.0, -viewport_height, 0.0);
        let pixel_delta_u = viewport_u / image_width as Float;
        let pixel_delta_v = viewport_v / image_height as Float;
        let pixel00_loc =
            center - Vec3::new(0.0, 0.0, focal_length) - viewport_u / 2.0 - viewport_v / 2.0;

        Camera {
            aspect_ratio,
            image_width,
            image_height,
            center,
            pixel00_loc,
            pixel_delta_u,
            pixel_delta_v,
        }
    }

    pub fn render(&self, world: &impl Hittable<Float>) -> BinaryPpmBuf {
        info!("Rendering image...");

        let deps = (self.image_width * self.image_height) as u64;
        let pb = ProgressBar::new(deps);
        let spinner_style = ProgressStyle::default_bar();
        pb.set_style(spinner_style);

        let mut image = BinaryPpmBuf::new(
            self.image_width,
            self.image_height,
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
                    self.pixel00_loc + self.pixel_delta_u * x as Float + self.pixel_delta_v * y as Float;
                let ray_dir = pixel_center - self.center;

                let ray = Ray::new(self.center, ray_dir);

                let color = Self::ray_color(&ray, world);
                *pixel = [Self::to_u8(color.x), Self::to_u8(color.y), Self::to_u8(color.z)];
                pb.inc(1);
            });
        pb.with_finish(indicatif::ProgressFinish::WithMessage(Cow::Borrowed(
            "Image rendered.",
        )));

        image
    }

    fn ray_color(ray: &Ray<Float>, world: &impl Hittable<Float>) -> Vec3<Float> {
        if let Some(rec) = world.hit(
            ray,
            Range {
                start: 0.0,
                end: Float::INFINITY,
            },
        ) {
            return (rec.normal + Vec3::one()) * 0.5;
        }
        let unit_dir = ray.dir.normalize();
        let a = 0.5 * (unit_dir.y + 1.0);
        Vec3::one() * (1.0 - a) + Vec3::new(0.5, 0.7, 1.0) * a
    }

    #[inline(always)]
    const fn to_u8(v: Float) -> u8 {
        (v * 255.999).round() as u8
    }
}

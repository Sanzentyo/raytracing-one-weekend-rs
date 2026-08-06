use indicatif::{ProgressBar, ProgressStyle};
use rand::prelude::*;
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
    aspect_ratio: Float, // Aspect ratio of the rendered image
    image_width: usize,         // Rendered image width
    image_height: usize,        // Rendered image height
    samples_per_pixel: usize,   // Number of samples per pixel
    pixel_samples_scale: Float, // Color scale factor for a sum of pixel samples
    center: Vec3<Float>,        // Camera center
    pixel00_loc: Vec3<Float>,   // Location of pixel 0, 0
    pixel_delta_u: Vec3<Float>, // Offset to pixel to the right
    pixel_delta_v: Vec3<Float>, // Offset to pixel below
    thread_rng: ThreadRng,
}

impl Camera {
    pub fn new(aspect_ratio: Float, image_width: usize, samples_per_pixel: usize) -> Self {
        let image_height = (image_width as Float / aspect_ratio).round() as usize;
        assert!(
            image_width > 0 && image_height > 0,
            "image width and height must be positive"
        );

        let pixel_samples_scale = 1.0 / samples_per_pixel as Float;

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
            samples_per_pixel,
            pixel_samples_scale,
            center,
            pixel00_loc,
            pixel_delta_u,
            pixel_delta_v,
            thread_rng: rand::rng(),
        }
    }

    pub fn get_ray(&mut self, x: usize, y: usize) -> Ray<Float> {
        // Construct a camera ray originating from the origin and directed at randomly sampled
        // point around the pixel location x, y.
        let offset = self.sample_square();
        let pixel_sample = self.pixel00_loc
            + self.pixel_delta_u * (x as Float + offset.x)
            + self.pixel_delta_v * (y as Float + offset.y);
        Ray::new(self.center, pixel_sample - self.center)
    }

    pub fn render(&mut self, world: &impl Hittable<Float>) -> BinaryPpmBuf {
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

                let mut color = Vec3::zero();
                (0..self.samples_per_pixel).for_each(|_| {
                    let ray = self.get_ray(x, y);
                    color += Self::ray_color(&ray, world);
                });

                color *= self.pixel_samples_scale;

                *pixel = [
                    Self::to_u8(color.x),
                    Self::to_u8(color.y),
                    Self::to_u8(color.z),
                ];
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
        (v.clamp(0.0, 0.999) * 256.0).round() as u8
    }

    /// Returns the vector to a random point in the [-0.5, 0.5]
    #[inline(always)]
    fn sample_square(&mut self) -> Vec3<Float> {
        Vec3::new(
            self.thread_rng.random_range(0.0..1.0) - 0.5,
            self.thread_rng.random_range(0.0..1.0) - 0.5,
            0.0,
        )
    }
}

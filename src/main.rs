use anyhow::Result;
use raytracing_one_weekend_rs::Float;
use raytracing_one_weekend_rs::camera::Camera;
use raytracing_one_weekend_rs::hittable::HittableList;
use raytracing_one_weekend_rs::pnm::Pnm;
use raytracing_one_weekend_rs::sphere::Sphere;
use raytracing_one_weekend_rs::vec::Vec3;

use tracing::info;

fn main() -> Result<()> {
    // image
    const ASPECT_RATIO: Float = 16. / 9.;
    const IMAGE_WIDTH: usize = 400;

    const OUTPUT_DIR: &str = "output";

    tracing_subscriber::fmt::init();

    let cam = Camera::new(ASPECT_RATIO, IMAGE_WIDTH);

    let world = HittableList::new(vec![
        Box::new(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5)),
        Box::new(Sphere::new(Vec3::new(0.0, -100.5, -1.0), 100.0)),
    ]);

    let image = cam.render(&world);

    let pnm = Pnm::BinaryPpm(image);

    info!("Saving image...");
    std::fs::create_dir_all(OUTPUT_DIR)?;
    let saved_path = pnm.save_with_extension(std::path::Path::new(OUTPUT_DIR).join("image"))?;
    info!("Image saved {}", saved_path.display());
    Ok(())
}

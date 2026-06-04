use std::range::Range;

use crate::{
    hittable::{HitRecord, Hittable},
    ray::Ray,
    vec::{Vec3, VecElem},
};

pub struct Sphere<T: VecElem> {
    pub center: Vec3<T>,
    pub radius: T,
}

impl<T: VecElem> Sphere<T> {
    pub const fn new(center: Vec3<T>, radius: T) -> Self {
        Self { center, radius }
    }
}

impl<T: VecElem> Hittable<T> for Sphere<T> {
    fn hit(
        &self,
        ray: &Ray<T>,
        t_range: Range<T>,
    ) -> Option<HitRecord<T>> {
        let oc = self.center - ray.orig;
        let a = ray.dir.length_squared();
        let h = ray.dir.dot(oc);
        let c = oc.length_squared() - self.radius * self.radius;

        let discriminant = h * h - a * c;
        if discriminant < T::zero() {
            return None;
        }
        let sqrtd = discriminant.sqrt();

        // find the nearest root that lies in the acceptable range
        let mut root = (h - sqrtd) / a;
        if !(t_range.contains(&root)) {
            root = (h + sqrtd) / a;
            if !(t_range.contains(&root)) {
                return None;
            }
        }

        let t = root;
        let p = ray.at(t);
        let outward_normal = (p - self.center) / self.radius;
        let (normal, front_face) = HitRecord::calc_face_normal(ray, outward_normal);
        Some(HitRecord { p, normal, t, front_face })
    }
}

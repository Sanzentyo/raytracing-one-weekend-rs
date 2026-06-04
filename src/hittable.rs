use std::range::Range;

use crate::{
    ray::Ray,
    vec::{Vec3, VecElem},
};

pub trait Hittable<T: VecElem> {
    fn hit(
        &self,
        ray: &Ray<T>,
        t_range: Range<T>,
        rec: Option<HitRecord<T>>,
    ) -> Option<HitRecord<T>>;
}

pub struct HitRecord<T: VecElem> {
    pub p: Vec3<T>,
    pub normal: Vec3<T>,
    pub t: T,
}

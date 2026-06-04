use std::range::Range;

use crate::{
    ray::Ray,
    vec::{Vec3, VecElem},
};

pub trait Hittable<T: VecElem> {
    fn hit(&self, ray: &Ray<T>, t_range: Range<T>) -> Option<HitRecord<T>>;
}

pub struct HitRecord<T: VecElem> {
    pub p: Vec3<T>,
    pub normal: Vec3<T>,
    pub t: T,
    pub front_face: bool,
}

impl<T: VecElem> HitRecord<T> {
    pub fn calc_face_normal(ray: &Ray<T>, outward_normal: Vec3<T>) -> (Vec3<T>, bool) {
        let front_face = ray.front_face(outward_normal);
        let normal = if front_face {
            outward_normal
        } else {
            -outward_normal
        };
        (normal, front_face)
    }
}

pub struct HittableList<T: VecElem> {
    pub list: Vec<Box<dyn Hittable<T>>>,
}

impl<T: VecElem> HittableList<T> {
    pub fn new(list: Vec<Box<dyn Hittable<T>>>) -> Self {
        Self { list }
    }

    pub fn add(&mut self, item: Box<dyn Hittable<T>>) {
        self.list.push(item);
    }
}

impl<T: VecElem> Hittable<T> for HittableList<T> {
    fn hit(&self, ray: &Ray<T>, mut t_range: Range<T>) -> Option<HitRecord<T>> {
        self.list.iter().fold(None, |closest, item| {
            if let Some(rec) = item.hit(ray, t_range) {
                t_range = Range::<T> {
                    start: t_range.start,
                    end: rec.t,
                };
                Some(rec)
            } else {
                closest
            }
        })
    }
}

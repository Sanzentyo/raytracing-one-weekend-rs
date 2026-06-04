use crate::vec::{Vec3, VecElem};

pub struct Ray<T: VecElem> {
    pub orig: Vec3<T>,
    pub dir: Vec3<T>,
}

impl<T: VecElem> Ray<T> {
    pub fn new(orig: Vec3<T>, dir: Vec3<T>) -> Self {
        Self { orig, dir }
    }

    pub fn origin(&self) -> &Vec3<T> {
        &self.orig
    }

    pub fn direction(&self) -> &Vec3<T> {
        &self.dir
    }

    pub fn at(&self, t: T) -> Vec3<T> {
        self.orig + self.dir * t
    }
}

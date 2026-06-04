use num_traits::{Float, Num};
use std::fmt::Display;

pub trait VecElem: Copy + Num + Float + Default + Display {}
impl<T: Copy + Num + Float + Default + Display> VecElem for T {}

#[derive(Debug, Clone, Copy)]
#[repr(C, align(16))]
pub struct Vec3<T: VecElem> {
    pub x: T,
    pub y: T,
    pub z: T,
    _padding: T,
}

impl<T: VecElem> Default for Vec3<T> {
    fn default() -> Self {
        Self {
            x: T::zero(),
            y: T::zero(),
            z: T::zero(),
            _padding: T::zero(),
        }
    }
}

impl<T: VecElem> Vec3<T> {
    pub const fn new(x: T, y: T, z: T) -> Self {
        Self {
            x,
            y,
            z,
            _padding: z, // 適当な値でいいので、zを入れる(T::zero()だとconstにできない)
        }
    }

    pub fn zero() -> Self {
        Self::default()
    }

    pub fn one() -> Self {
        Self::new(T::one(), T::one(), T::one())
    }

    pub fn dot(self, rhs: Self) -> T {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn cross(self, rhs: Self) -> Self {
        Self::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    #[inline(always)]
    pub fn length_squared(self) -> T {
        self.dot(self)
    }

    #[inline(always)]
    pub fn length(self) -> T {
        self.length_squared().sqrt()
    }

    #[inline(always)]
    pub fn normalize(self) -> Self {
        self / self.length()
    }
}

impl<T: VecElem> Display for Vec3<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}, {}, {}", self.x, self.y, self.z)
    }
}

impl<T: VecElem> std::ops::Add<Vec3<T>> for Vec3<T> {
    type Output = Vec3<T>;

    fn add(self, rhs: Vec3<T>) -> Vec3<T> {
        Vec3 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
            _padding: T::zero(),
        }
    }
}
impl<T: VecElem> std::ops::Sub<Vec3<T>> for Vec3<T> {
    type Output = Vec3<T>;

    fn sub(self, rhs: Vec3<T>) -> Vec3<T> {
        Vec3 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
            _padding: T::zero(),
        }
    }
}

impl<T: VecElem> std::ops::Mul<Vec3<T>> for Vec3<T> {
    type Output = Vec3<T>;

    fn mul(self, rhs: Vec3<T>) -> Vec3<T> {
        Vec3 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
            _padding: T::zero(),
        }
    }
}

impl<T: VecElem> std::ops::Mul<T> for Vec3<T> {
    type Output = Vec3<T>;

    fn mul(self, rhs: T) -> Vec3<T> {
        Vec3 {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
            _padding: T::zero(),
        }
    }
}

impl<T: VecElem> std::ops::Div<T> for Vec3<T> {
    type Output = Vec3<T>;

    fn div(self, rhs: T) -> Vec3<T> {
        Vec3 {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
            _padding: T::zero(),
        }
    }
}

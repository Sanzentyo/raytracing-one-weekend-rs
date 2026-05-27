use num_traits::Num;

pub trait VecElem: Copy + Num {}
impl<T: Copy + Num> VecElem for T {}

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
    pub fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z, _padding: T::zero() }
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

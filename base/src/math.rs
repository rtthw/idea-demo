//! # Math

use std::ops::{Add, Mul, Sub};



#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Axis {
    Horizontal,
    Vertical,
}

impl Axis {
    pub const fn cross(&self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }

    pub fn pack_size(self, axis_value: f32, cross_value: f32) -> Size {
        match self {
            Self::Horizontal => Size::new(axis_value, cross_value),
            Self::Vertical => Size::new(cross_value, axis_value),
        }
    }

    pub fn pack_point(self, axis_value: f32, cross_value: f32) -> Point {
        match self {
            Self::Horizontal => Point::new(axis_value, cross_value),
            Self::Vertical => Point::new(cross_value, axis_value),
        }
    }
}



#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Area {
    pub position: Point,
    pub size: Size,
}

impl Area {
    pub const ZERO: Self = Self::new(Point::ZERO, Size::ZERO);

    #[inline]
    pub const fn new(position: Point, size: Size) -> Self {
        Self { position, size }
    }

    #[inline]
    pub const fn from_size(size: Size) -> Self {
        Self {
            position: Point::ZERO,
            size,
        }
    }

    #[inline]
    pub const fn from_min_max(min: Point, max: Point) -> Self {
        let size = min.size_up_to(max);
        Self {
            position: min,
            size,
        }
    }

    #[inline]
    pub const fn union(&self, other: Self) -> Self {
        let max = self.max_point();
        let other_max = other.max_point();
        Self::from_min_max(
            Point::new(
                self.position.x.min(other.position.x),
                self.position.y.min(other.position.y),
            ),
            Point::new(max.x.max(other_max.x), max.y.max(other_max.y)),
        )
    }

    #[inline]
    pub const fn contains(&self, point: Point) -> bool {
        let max = self.max_point();
        self.position.x <= point.x
            && self.position.y <= point.y
            && max.x > point.x
            && max.y > point.y
    }

    #[inline]
    pub const fn max_point(&self) -> Point {
        self.position.add_size(self.size)
    }
}



#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    #[inline]
    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    #[inline]
    pub const fn round(&self) -> Self {
        Self {
            width: self.width.round(),
            height: self.height.round(),
        }
    }

    pub const fn value_for_axis(&self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        }
    }
}

impl Add for Size {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            width: self.width + rhs.width,
            height: self.height + rhs.height,
        }
    }
}

impl Mul for Size {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: Self) -> Self::Output {
        Self {
            width: self.width * rhs.width,
            height: self.height * rhs.height,
        }
    }
}



#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self::new(0.0, 0.0);

    #[inline]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn round(&self) -> Self {
        Self {
            x: self.x.round(),
            y: self.y.round(),
        }
    }

    #[inline]
    pub const fn value_for_axis(&self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.x,
            Axis::Vertical => self.y,
        }
    }

    #[inline]
    pub const fn add_size(&self, size: Size) -> Self {
        Self {
            x: self.x + size.width,
            y: self.y + size.height,
        }
    }

    #[inline]
    pub const fn size_up_to(&self, max: Point) -> Size {
        Size {
            width: max.x - self.x,
            height: max.y - self.y,
        }
    }
}

impl Add for Point {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Sub for Point {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl Add<Size> for Point {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Size) -> Self::Output {
        Self {
            x: self.x + rhs.width,
            y: self.y + rhs.height,
        }
    }
}



/// An affine transformation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine([f32; 6]);

impl Affine {
    /// The identity transform (i.e. no transformation).
    pub const IDENTITY: Affine = Affine::scale(1.0);

    /// A transform that is flipped on the y-axis.
    pub const FLIP_Y: Affine = Affine::new([1.0, 0.0, 0.0, -1.0, 0.0, 0.0]);

    /// A transform that is flipped on the x-axis.
    pub const FLIP_X: Affine = Affine::new([-1.0, 0.0, 0.0, 1.0, 0.0, 0.0]);

    /// Construct an affine transform from coefficients.
    #[inline(always)]
    pub const fn new(c: [f32; 6]) -> Affine {
        Affine(c)
    }

    /// An affine transform representing uniform scaling.
    #[inline(always)]
    pub const fn scale(amount: f32) -> Affine {
        Affine([amount, 0.0, 0.0, amount, 0.0, 0.0])
    }

    #[inline(always)]
    pub const fn translation(self) -> Point {
        Point {
            x: self.0[4],
            y: self.0[5],
        }
    }

    #[inline]
    pub const fn with_translation(mut self, translation: Point) -> Self {
        self.0[4] += translation.x;
        self.0[5] += translation.y;
        self
    }

    pub const fn determinant(self) -> f32 {
        self.0[0] * self.0[3] - self.0[1] * self.0[2]
    }

    pub const fn inverse(self) -> Self {
        let inv_det = self.determinant().recip();

        Self([
            inv_det * self.0[3],
            -inv_det * self.0[1],
            -inv_det * self.0[2],
            inv_det * self.0[0],
            inv_det * (self.0[2] * self.0[5] - self.0[3] * self.0[4]),
            inv_det * (self.0[1] * self.0[4] - self.0[0] * self.0[5]),
        ])
    }

    pub const fn tranform_point(self, point: Point) -> Point {
        Point {
            x: self.0[0] * point.x + self.0[2] * point.y + self.0[4],
            y: self.0[1] * point.x + self.0[3] * point.y + self.0[5],
        }
    }

    pub const fn transform_area(self, area: Area) -> Area {
        let min = area.position;
        let max = area.max_point();

        let p00 = self.tranform_point(Point::new(min.x, min.y));
        let p01 = self.tranform_point(Point::new(min.x, max.y));
        let p10 = self.tranform_point(Point::new(max.x, min.y));
        let p11 = self.tranform_point(Point::new(max.x, max.y));

        Area::from_min_max(p00, p01).union(Area::from_min_max(p10, p11))
    }
}

impl Mul for Affine {
    type Output = Self;

    #[inline]
    fn mul(self, other: Self) -> Self {
        Self([
            self.0[0] * other.0[0] + self.0[2] * other.0[1],
            self.0[1] * other.0[0] + self.0[3] * other.0[1],
            self.0[0] * other.0[2] + self.0[2] * other.0[3],
            self.0[1] * other.0[2] + self.0[3] * other.0[3],
            self.0[0] * other.0[4] + self.0[2] * other.0[5] + self.0[4],
            self.0[1] * other.0[4] + self.0[3] * other.0[5] + self.0[5],
        ])
    }
}

impl Mul<Point> for Affine {
    type Output = Point;

    #[inline(always)]
    fn mul(self, point: Point) -> Point {
        self.tranform_point(point)
    }
}

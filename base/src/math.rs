//! # Math

use std::ops::{Add, Sub};



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

    pub const fn contains(&self, point: Point) -> bool {
        let max = self.position.add_size(self.size);
        self.position.x <= point.x
            && self.position.y <= point.y
            && max.x > point.x
            && max.y > point.y
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

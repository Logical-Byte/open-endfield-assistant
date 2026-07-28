use std::ops::{Add, Div, Mul, Sub};

use windows::Win32::Foundation::{POINT, RECT};

use crate::utils::cast::Cast;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point2D<T> {
    pub x: T,
    pub y: T,
}

impl<T> From<POINT> for Point2D<T>
where
    i32: Cast<T>,
{
    fn from(point: POINT) -> Self {
        Self {
            x: point.x.cast(),
            y: point.y.cast(),
        }
    }
}

impl<T> From<Point2D<T>> for POINT
where
    T: Cast<i32>,
{
    fn from(point: Point2D<T>) -> Self {
        Self {
            x: point.x.cast(),
            y: point.y.cast(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Region2D<T> {
    p0: Point2D<T>,
    p1: Point2D<T>,
}

impl<T> Region2D<T> {
    pub fn from_points(p0: Point2D<T>, p1: Point2D<T>) -> Self {
        Self { p0, p1 }
    }

    pub fn from_ltwh(x: T, y: T, width: T, height: T) -> Self
    where
        T: Copy + Add<Output = T>,
    {
        Self {
            p0: Point2D { x, y },
            p1: Point2D {
                x: x + width,
                y: y + height,
            },
        }
    }

    pub fn from_ltrb(left: T, top: T, right: T, bottom: T) -> Self {
        Self {
            p0: Point2D { x: left, y: top },
            p1: Point2D {
                x: right,
                y: bottom,
            },
        }
    }

    pub fn p0(&self) -> Point2D<T>
    where
        T: Copy,
    {
        self.p0
    }

    pub fn p1(&self) -> Point2D<T>
    where
        T: Copy,
    {
        self.p1
    }

    pub fn x0(&self) -> T
    where
        T: Copy,
    {
        self.p0.x
    }

    pub fn y0(&self) -> T
    where
        T: Copy,
    {
        self.p0.y
    }

    pub fn x1(&self) -> T
    where
        T: Copy,
    {
        self.p1.x
    }

    pub fn y1(&self) -> T
    where
        T: Copy,
    {
        self.p1.y
    }

    pub fn left(&self) -> T
    where
        T: Copy,
    {
        self.p0.x
    }

    pub fn top(&self) -> T
    where
        T: Copy,
    {
        self.p0.y
    }

    pub fn right(&self) -> T
    where
        T: Copy,
    {
        self.p1.x
    }

    pub fn bottom(&self) -> T
    where
        T: Copy,
    {
        self.p1.y
    }

    pub fn width(&self) -> T
    where
        T: Copy + Sub<Output = T>,
    {
        self.p1.x - self.p0.x
    }

    pub fn height(&self) -> T
    where
        T: Copy + Sub<Output = T>,
    {
        self.p1.y - self.p0.y
    }

    pub fn x_center(&self) -> T
    where
        T: Copy + Add<Output = T> + Div<Output = T>,
        u8: Cast<T>,
    {
        (self.p0.x + self.p1.x) / 2.cast()
    }

    pub fn y_center(&self) -> T
    where
        T: Copy + Add<Output = T> + Div<Output = T>,
        u8: Cast<T>,
    {
        (self.p0.y + self.p1.y) / 2.cast()
    }

    pub fn center(&self) -> Point2D<T>
    where
        T: Copy + Add<Output = T> + Div<Output = T>,
        u8: Cast<T>,
    {
        Point2D {
            x: self.x_center(),
            y: self.y_center(),
        }
    }

    pub fn area(&self) -> T
    where
        T: Copy + Sub<Output = T> + Mul<Output = T>,
    {
        self.width() * self.height()
    }

    pub fn apply_padding_ltrb(&self, pl: T, pt: T, pr: T, pb: T) -> Self
    where
        T: Copy + Add<Output = T> + Sub<Output = T>,
    {
        let left = self.left() - pl;
        let top = self.top() - pt;
        let right = self.right() + pr;
        let bottom = self.bottom() + pb;

        Self::from_ltrb(left, top, right, bottom)
    }

    pub fn apply_padding(&self, padding: T) -> Self
    where
        T: Copy + Add<Output = T> + Sub<Output = T>,
    {
        self.apply_padding_ltrb(padding, padding, padding, padding)
    }
}

impl<T> From<RECT> for Region2D<T>
where
    i32: Cast<T>,
{
    fn from(rect: RECT) -> Self {
        Self {
            p0: Point2D {
                x: rect.left.cast(),
                y: rect.top.cast(),
            },
            p1: Point2D {
                x: rect.right.cast(),
                y: rect.bottom.cast(),
            },
        }
    }
}

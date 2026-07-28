use std::ops::{Add, Div, Mul, Sub};

use num_traits::{AsPrimitive, One};
use windows::Win32::Foundation::RECT;

use crate::utils::point::Point2D;

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
        T: Copy + Add<Output = T> + Div<Output = T> + One,
    {
        let two = T::one() + T::one();
        (self.p0.x + self.p1.x) / two
    }

    pub fn y_center(&self) -> T
    where
        T: Copy + Add<Output = T> + Div<Output = T> + One,
    {
        let two: T = T::one() + T::one();
        (self.p0.y + self.p1.y) / two
    }

    pub fn center(&self) -> Point2D<T>
    where
        T: Copy + Add<Output = T> + Div<Output = T> + One,
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
        Self::from_ltrb(
            self.left() - pl,
            self.top() - pt,
            self.right() + pr,
            self.bottom() + pb,
        )
    }

    pub fn apply_padding(&self, padding: T) -> Self
    where
        T: Copy + Add<Output = T> + Sub<Output = T>,
    {
        self.apply_padding_ltrb(padding, padding, padding, padding)
    }
}

impl<T> Region2D<T> {
    /// 将 Region2D 转换为另一种数值类型。
    pub fn cast<U>(self) -> Region2D<U>
    where
        T: AsPrimitive<U>,
        U: 'static + Copy,
    {
        Region2D {
            p0: self.p0.cast(),
            p1: self.p1.cast(),
        }
    }
}

impl From<RECT> for Region2D<i32> {
    fn from(rect: RECT) -> Self {
        Self {
            p0: Point2D {
                x: rect.left,
                y: rect.top,
            },
            p1: Point2D {
                x: rect.right,
                y: rect.bottom,
            },
        }
    }
}

impl From<Region2D<i32>> for RECT {
    fn from(region: Region2D<i32>) -> Self {
        Self {
            left: region.p0.x,
            top: region.p0.y,
            right: region.p1.x,
            bottom: region.p1.y,
        }
    }
}

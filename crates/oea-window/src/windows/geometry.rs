//! 备选截图 implementation 使用的私有矩形类型。

use crate::Point;
use ::windows::Win32::Foundation::{POINT, RECT};

impl From<POINT> for Point {
    fn from(point: POINT) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

impl From<Point> for POINT {
    fn from(point: Point) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Region(RECT);

impl From<RECT> for Region {
    fn from(rect: RECT) -> Self {
        Self(rect)
    }
}

impl Region {
    pub(crate) fn from_points(start: Point, end: Point) -> Self {
        Self(RECT {
            left: start.x,
            top: start.y,
            right: end.x,
            bottom: end.y,
        })
    }
    pub(crate) fn p0(self) -> Point {
        Point {
            x: self.0.left,
            y: self.0.top,
        }
    }
    pub(crate) fn p1(self) -> Point {
        Point {
            x: self.0.right,
            y: self.0.bottom,
        }
    }
    pub(crate) fn x0(self) -> i32 {
        self.0.left
    }
    pub(crate) fn y0(self) -> i32 {
        self.0.top
    }
    pub(crate) fn x1(self) -> i32 {
        self.0.right
    }
    pub(crate) fn y1(self) -> i32 {
        self.0.bottom
    }
    pub(crate) fn width(self) -> i32 {
        self.0.right - self.0.left
    }
    pub(crate) fn height(self) -> i32 {
        self.0.bottom - self.0.top
    }
}

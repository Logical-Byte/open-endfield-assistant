use windows::Win32::Foundation::{POINT, RECT};

use crate::utils::{point::Point2D, region::Region2D};

impl From<POINT> for Point2D<i32> {
    fn from(point: POINT) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

impl From<Point2D<i32>> for POINT {
    fn from(point: Point2D<i32>) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

impl From<RECT> for Region2D<i32> {
    fn from(rect: RECT) -> Self {
        Self::from_ltrb(rect.left, rect.top, rect.right, rect.bottom)
    }
}

impl From<Region2D<i32>> for RECT {
    fn from(region: Region2D<i32>) -> Self {
        let p0 = region.p0();
        let p1 = region.p1();
        Self {
            left: p0.x,
            top: p0.y,
            right: p1.x,
            bottom: p1.y,
        }
    }
}

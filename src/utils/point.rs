use num_traits::AsPrimitive;
use windows::Win32::Foundation::POINT;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point2D<T> {
    pub x: T,
    pub y: T,
}

impl<T> Point2D<T> {
    /// 将 Point2D 转换为另一种数值类型。
    pub fn cast<U>(self) -> Point2D<U>
    where
        T: AsPrimitive<U>,
        U: 'static + Copy,
    {
        Point2D {
            x: self.x.as_(),
            y: self.y.as_(),
        }
    }
}

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

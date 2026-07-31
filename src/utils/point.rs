use num_traits::AsPrimitive;
use windows::Win32::Foundation::POINT;

/// 泛型二维点结构体，表示平面上的一个坐标 `(x, y)`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Point2D<T> {
    /// x 坐标
    pub x: T,
    /// y 坐标
    pub y: T,
}

impl<T> From<(T, T)> for Point2D<T> {
    /// 将一个元组 `(x, y): (T, T)` 转换为 [`Point2D<T>`]。
    fn from(tuple: (T, T)) -> Self {
        Self {
            x: tuple.0,
            y: tuple.1,
        }
    }
}

impl<T> From<Point2D<T>> for (T, T) {
    /// 将 [`Point2D<T>`] 转换为一个元组 `(x, y): (T, T)`。
    fn from(point: Point2D<T>) -> Self {
        (point.x, point.y)
    }
}

impl<T> Point2D<T> {
    /// 将 [`Point2D<T>`] 转换为 [`Point2D<U>`]，其中 `T` 可以转换为 `U`。
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
    /// 将 Windows API 的 [`POINT`] 结构转换为 [`Point2D<i32>`]。
    fn from(point: POINT) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

impl From<Point2D<i32>> for POINT {
    /// 将 [`Point2D<i32>`] 转换为 Windows API 的 [`POINT`] 结构。
    fn from(point: Point2D<i32>) -> Self {
        Self {
            x: point.x,
            y: point.y,
        }
    }
}

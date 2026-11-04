use az::{Cast, CheckedCast, OverflowingCast, SaturatingCast, StrictCast, WrappingCast};

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

impl<T, U> Cast<Point2D<U>> for Point2D<T>
where
    T: Cast<U>,
{
    /// 将 [`Point2D<T>`] 转换为 [`Point2D<U>`]，其中 `T` 可以转换为 `U`。
    fn cast(self) -> Point2D<U> {
        Point2D {
            x: self.x.cast(),
            y: self.y.cast(),
        }
    }
}

impl<T, U> CheckedCast<Point2D<U>> for Point2D<T>
where
    T: CheckedCast<U>,
{
    /// 尝试将 [`Point2D<T>`] 转换为 [`Point2D<U>`]，如果 `T` 无法转换为 `U`，则返回 `None`。
    fn checked_cast(self) -> Option<Point2D<U>> {
        Some(Point2D {
            x: self.x.checked_cast()?,
            y: self.y.checked_cast()?,
        })
    }
}

impl<T, U> StrictCast<Point2D<U>> for Point2D<T>
where
    T: StrictCast<U>,
{
    /// 将 [`Point2D<T>`] 严格转换为 [`Point2D<U>`]，如果 `T` 无法严格转换为 `U`，则会 panic。
    fn strict_cast(self) -> Point2D<U> {
        Point2D {
            x: self.x.strict_cast(),
            y: self.y.strict_cast(),
        }
    }
}

impl<T, U> SaturatingCast<Point2D<U>> for Point2D<T>
where
    T: SaturatingCast<U>,
{
    /// 将 [`Point2D<T>`] 饱和转换为 [`Point2D<U>`]，如果 `T` 超出 `U` 的范围，则会返回 `U` 的边界值。
    fn saturating_cast(self) -> Point2D<U> {
        Point2D {
            x: self.x.saturating_cast(),
            y: self.y.saturating_cast(),
        }
    }
}

impl<T, U> WrappingCast<Point2D<U>> for Point2D<T>
where
    T: WrappingCast<U>,
{
    /// 将 [`Point2D<T>`] 环绕转换为 [`Point2D<U>`]，如果 `T` 超出 `U` 的范围，则会环绕回 `U` 的范围内。
    fn wrapping_cast(self) -> Point2D<U> {
        Point2D {
            x: self.x.wrapping_cast(),
            y: self.y.wrapping_cast(),
        }
    }
}

impl<T, U> OverflowingCast<Point2D<U>> for Point2D<T>
where
    T: OverflowingCast<U>,
{
    /// 将 [`Point2D<T>`] 溢出转换为 [`Point2D<U>`]，如果 `T` 超出 `U` 的范围，则会返回溢出标志。
    fn overflowing_cast(self) -> (Point2D<U>, bool) {
        let (x, x_overflow) = self.x.overflowing_cast();
        let (y, y_overflow) = self.y.overflowing_cast();
        (Point2D { x, y }, x_overflow || y_overflow)
    }
}

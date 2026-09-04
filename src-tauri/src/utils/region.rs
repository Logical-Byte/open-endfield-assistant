use std::ops::{Add, Div, Mul, Sub};

use az::{Cast, CheckedCast, OverflowingCast, SaturatingCast, StrictCast, WrappingCast};

use crate::utils::point::Point2D;

/// 泛型二维矩形区域结构体，由左上角 `p0` 和右下角 `p1` 两个点定义。
///
/// 约定 `p0` 为区域左上角（left, top），`p1` 为区域右下角（right, bottom）。
/// 支持 ltrb（left/top/right/bottom）与 ltwh（left/top/width/height）两种构造方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Region2D<T> {
    /// 区域左上角坐标
    p0: Point2D<T>,
    /// 区域右下角坐标
    p1: Point2D<T>,
}

impl<T> Region2D<T> {
    /// 由两个点构造 [`Region2D<T>`]。
    ///
    /// `p0` 应为左上角，`p1` 应为右下角。
    pub fn from_points(p0: Point2D<T>, p1: Point2D<T>) -> Self {
        Self { p0, p1 }
    }

    /// 以 ltwh（left, top, width, height）格式构造 [`Region2D<T>`]。
    ///
    /// 内部转换为 ltrb 表示：`right = left + width`，`bottom = top + height`。
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

    /// 以 ltrb（left, top, right, bottom）格式构造 [`Region2D<T>`]。
    ///
    /// 此方法为 `const fn`，可用于编译期常量声明。
    pub const fn from_ltrb(left: T, top: T, right: T, bottom: T) -> Self {
        Self {
            p0: Point2D { x: left, y: top },
            p1: Point2D {
                x: right,
                y: bottom,
            },
        }
    }

    /// 返回区域左上角点 `p0`。
    pub fn p0(&self) -> Point2D<T>
    where
        T: Copy,
    {
        self.p0
    }

    /// 返回区域右下角点 `p1`。
    pub fn p1(&self) -> Point2D<T>
    where
        T: Copy,
    {
        self.p1
    }

    /// 返回左上角 x 坐标（等同于 [`left`](Self::left)）。
    pub fn x0(&self) -> T
    where
        T: Copy,
    {
        self.p0.x
    }

    /// 返回左上角 y 坐标（等同于 [`top`](Self::top)）。
    pub fn y0(&self) -> T
    where
        T: Copy,
    {
        self.p0.y
    }

    /// 返回右下角 x 坐标（等同于 [`right`](Self::right)）。
    pub fn x1(&self) -> T
    where
        T: Copy,
    {
        self.p1.x
    }

    /// 返回右下角 y 坐标（等同于 [`bottom`](Self::bottom)）。
    pub fn y1(&self) -> T
    where
        T: Copy,
    {
        self.p1.y
    }

    /// 返回区域左边界 x 坐标（等同于 [`x0`](Self::x0)）。
    pub fn left(&self) -> T
    where
        T: Copy,
    {
        self.p0.x
    }

    /// 返回区域上边界 y 坐标（等同于 [`y0`](Self::y0)）。
    pub fn top(&self) -> T
    where
        T: Copy,
    {
        self.p0.y
    }

    /// 返回区域右边界 x 坐标（等同于 [`x1`](Self::x1)）。
    pub fn right(&self) -> T
    where
        T: Copy,
    {
        self.p1.x
    }

    /// 返回区域下边界 y 坐标（等同于 [`y1`](Self::y1)）。
    pub fn bottom(&self) -> T
    where
        T: Copy,
    {
        self.p1.y
    }

    /// 返回区域宽度 `right - left`。
    pub fn width(&self) -> T
    where
        T: Copy + Sub<Output = T>,
    {
        self.p1.x - self.p0.x
    }

    /// 返回区域高度 `bottom - top`。
    pub fn height(&self) -> T
    where
        T: Copy + Sub<Output = T>,
    {
        self.p1.y - self.p0.y
    }

    /// 返回区域中心点坐标 `(x_center, y_center)`。
    pub fn center(&self) -> Point2D<T>
    where
        T: Copy + Add<Output = T> + Div<Output = T>,
        u8: Cast<T>,
    {
        Point2D {
            x: (self.p0.x + self.p1.x) / 2.cast(),
            y: (self.p0.y + self.p1.y) / 2.cast(),
        }
    }

    /// 返回区域面积 `width * height`。
    pub fn area(&self) -> T
    where
        T: Copy + Sub<Output = T> + Mul<Output = T>,
    {
        self.width() * self.height()
    }
}

impl<T, U> Cast<Region2D<U>> for Region2D<T>
where
    T: Cast<U>,
{
    /// 将 [`Region2D<T>`] 转换为 [`Region2D<U>`]，其中 `T` 可以转换为 `U`。
    fn cast(self) -> Region2D<U> {
        Region2D {
            p0: self.p0.cast(),
            p1: self.p1.cast(),
        }
    }
}

impl<T, U> CheckedCast<Region2D<U>> for Region2D<T>
where
    T: CheckedCast<U>,
{
    /// 尝试将 [`Region2D<T>`] 转换为 [`Region2D<U>`]，如果 `T` 无法转换为 `U`，则返回 `None`。
    fn checked_cast(self) -> Option<Region2D<U>> {
        Some(Region2D {
            p0: self.p0.checked_cast()?,
            p1: self.p1.checked_cast()?,
        })
    }
}

impl<T, U> StrictCast<Region2D<U>> for Region2D<T>
where
    T: StrictCast<U>,
{
    /// 将 [`Region2D<T>`] 严格转换为 [`Region2D<U>`]，如果 `T` 无法严格转换为 `U`，则会 panic。
    fn strict_cast(self) -> Region2D<U> {
        Region2D {
            p0: self.p0.strict_cast(),
            p1: self.p1.strict_cast(),
        }
    }
}

impl<T, U> SaturatingCast<Region2D<U>> for Region2D<T>
where
    T: SaturatingCast<U>,
{
    /// 将 [`Region2D<T>`] 饱和转换为 [`Region2D<U>`]，如果 `T` 超出 `U` 的范围，则会返回 `U` 的边界值。
    fn saturating_cast(self) -> Region2D<U> {
        Region2D {
            p0: self.p0.saturating_cast(),
            p1: self.p1.saturating_cast(),
        }
    }
}

impl<T, U> WrappingCast<Region2D<U>> for Region2D<T>
where
    T: WrappingCast<U>,
{
    /// 将 [`Region2D<T>`] 环绕转换为 [`Region2D<U>`]，如果 `T` 超出 `U` 的范围，则会环绕回 `U` 的范围内。
    fn wrapping_cast(self) -> Region2D<U> {
        Region2D {
            p0: self.p0.wrapping_cast(),
            p1: self.p1.wrapping_cast(),
        }
    }
}

impl<T, U> OverflowingCast<Region2D<U>> for Region2D<T>
where
    T: OverflowingCast<U>,
{
    /// 将 [`Region2D<T>`] 溢出转换为 [`Region2D<U>`]，如果 `T` 超出 `U` 的范围，则会返回溢出标志。
    fn overflowing_cast(self) -> (Region2D<U>, bool) {
        let (p0, p0_overflow) = self.p0.overflowing_cast();
        let (p1, p1_overflow) = self.p1.overflowing_cast();
        (Region2D { p0, p1 }, p0_overflow || p1_overflow)
    }
}

/// 以 ltwh 格式创建 `Region2D`，可用于常量声明。
macro_rules! ltwh {
    ($left:expr, $top:expr, $width:expr, $height:expr $(,)?) => {{
        let left = $left;
        let top = $top;
        let width = $width;
        let height = $height;
        $crate::utils::region::Region2D::from_ltrb(left, top, left + width, top + height)
    }};
}

/// 以 ltrb 格式创建 `Region2D`，可用于常量声明。
macro_rules! ltrb {
    ($left:expr, $top:expr, $right:expr, $bottom:expr $(,)?) => {{
        let left = $left;
        let top = $top;
        let right = $right;
        let bottom = $bottom;
        $crate::utils::region::Region2D::from_ltrb(left, top, right, bottom)
    }};
}

pub(crate) use {ltrb, ltwh};

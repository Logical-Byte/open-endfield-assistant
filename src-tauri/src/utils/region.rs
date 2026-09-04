use std::ops::{Add, Div, Mul, Sub};

use az::{Cast, CheckedCast, OverflowingCast, SaturatingCast, StrictCast, WrappingCast};

use crate::utils::point::Point2D;

/// 泛型二维矩形区域结构体。
///
/// 约定 `p0` 为区域左上角（left, top），`p1` 为区域右下角（right, bottom）。
/// 区域使用半开区间 `[left, right) × [top, bottom)`；构造时不检查边界顺序。
/// 支持 ltrb（left/top/right/bottom）与 ltwh（left/top/width/height）两种构造方式。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Region2D<T> {
    left: T,
    top: T,
    right: T,
    bottom: T,
}

impl<T> Region2D<T> {
    /// 由两个点构造 [`Region2D<T>`]。
    ///
    /// `p0` 应为左上角，`p1` 应为右下角。
    pub fn from_points(p0: Point2D<T>, p1: Point2D<T>) -> Self {
        Self {
            left: p0.x,
            top: p0.y,
            right: p1.x,
            bottom: p1.y,
        }
    }

    /// 以 ltwh（left, top, width, height）格式构造 [`Region2D<T>`]。
    ///
    /// 内部转换为 ltrb 表示：`right = left + width`，`bottom = top + height`。
    pub fn from_ltwh(x: T, y: T, width: T, height: T) -> Self
    where
        T: Copy + Add<Output = T>,
    {
        Self {
            left: x,
            top: y,
            right: x + width,
            bottom: y + height,
        }
    }

    /// 以 ltrb（left, top, right, bottom）格式构造 [`Region2D<T>`]。
    ///
    /// 此方法为 `const fn`，可用于编译期常量声明。
    pub const fn from_ltrb(left: T, top: T, right: T, bottom: T) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    /// 返回区域左上角点 `p0`。
    pub fn p0(&self) -> Point2D<T>
    where
        T: Copy,
    {
        Point2D {
            x: self.left,
            y: self.top,
        }
    }

    /// 返回区域右下角点 `p1`。
    pub fn p1(&self) -> Point2D<T>
    where
        T: Copy,
    {
        Point2D {
            x: self.right,
            y: self.bottom,
        }
    }

    /// 返回左上角 x 坐标（等同于 [`left`](Self::left)）。
    pub fn x0(&self) -> T
    where
        T: Copy,
    {
        self.left
    }

    /// 返回左上角 y 坐标（等同于 [`top`](Self::top)）。
    pub fn y0(&self) -> T
    where
        T: Copy,
    {
        self.top
    }

    /// 返回右下角 x 坐标（等同于 [`right`](Self::right)）。
    pub fn x1(&self) -> T
    where
        T: Copy,
    {
        self.right
    }

    /// 返回右下角 y 坐标（等同于 [`bottom`](Self::bottom)）。
    pub fn y1(&self) -> T
    where
        T: Copy,
    {
        self.bottom
    }

    /// 返回区域左边界 x 坐标（等同于 [`x0`](Self::x0)）。
    pub fn left(&self) -> T
    where
        T: Copy,
    {
        self.left
    }

    /// 返回区域上边界 y 坐标（等同于 [`y0`](Self::y0)）。
    pub fn top(&self) -> T
    where
        T: Copy,
    {
        self.top
    }

    /// 返回区域右边界 x 坐标（等同于 [`x1`](Self::x1)）。
    pub fn right(&self) -> T
    where
        T: Copy,
    {
        self.right
    }

    /// 返回区域下边界 y 坐标（等同于 [`y1`](Self::y1)）。
    pub fn bottom(&self) -> T
    where
        T: Copy,
    {
        self.bottom
    }

    /// 返回区域宽度 `right - left`。
    pub fn width(&self) -> T
    where
        T: Copy + Sub<Output = T>,
    {
        self.right - self.left
    }

    /// 返回区域高度 `bottom - top`。
    pub fn height(&self) -> T
    where
        T: Copy + Sub<Output = T>,
    {
        self.bottom - self.top
    }

    /// 返回区域中心点坐标 `(x_center, y_center)`。
    pub fn center(&self) -> Point2D<T>
    where
        T: Copy + Add<Output = T> + Div<Output = T>,
        u8: Cast<T>,
    {
        Point2D {
            x: (self.left + self.right) / 2.cast(),
            y: (self.top + self.bottom) / 2.cast(),
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
        Region2D::from_ltrb(
            self.left.cast(),
            self.top.cast(),
            self.right.cast(),
            self.bottom.cast(),
        )
    }
}

impl<T, U> CheckedCast<Region2D<U>> for Region2D<T>
where
    T: CheckedCast<U>,
{
    /// 尝试将 [`Region2D<T>`] 转换为 [`Region2D<U>`]，如果 `T` 无法转换为 `U`，则返回 `None`。
    fn checked_cast(self) -> Option<Region2D<U>> {
        Some(Region2D::from_ltrb(
            self.left.checked_cast()?,
            self.top.checked_cast()?,
            self.right.checked_cast()?,
            self.bottom.checked_cast()?,
        ))
    }
}

impl<T, U> StrictCast<Region2D<U>> for Region2D<T>
where
    T: StrictCast<U>,
{
    /// 将 [`Region2D<T>`] 严格转换为 [`Region2D<U>`]，如果 `T` 无法严格转换为 `U`，则会 panic。
    fn strict_cast(self) -> Region2D<U> {
        Region2D::from_ltrb(
            self.left.strict_cast(),
            self.top.strict_cast(),
            self.right.strict_cast(),
            self.bottom.strict_cast(),
        )
    }
}

impl<T, U> SaturatingCast<Region2D<U>> for Region2D<T>
where
    T: SaturatingCast<U>,
{
    /// 将 [`Region2D<T>`] 饱和转换为 [`Region2D<U>`]，如果 `T` 超出 `U` 的范围，则会返回 `U` 的边界值。
    fn saturating_cast(self) -> Region2D<U> {
        Region2D::from_ltrb(
            self.left.saturating_cast(),
            self.top.saturating_cast(),
            self.right.saturating_cast(),
            self.bottom.saturating_cast(),
        )
    }
}

impl<T, U> WrappingCast<Region2D<U>> for Region2D<T>
where
    T: WrappingCast<U>,
{
    /// 将 [`Region2D<T>`] 环绕转换为 [`Region2D<U>`]，如果 `T` 超出 `U` 的范围，则会环绕回 `U` 的范围内。
    fn wrapping_cast(self) -> Region2D<U> {
        Region2D::from_ltrb(
            self.left.wrapping_cast(),
            self.top.wrapping_cast(),
            self.right.wrapping_cast(),
            self.bottom.wrapping_cast(),
        )
    }
}

impl<T, U> OverflowingCast<Region2D<U>> for Region2D<T>
where
    T: OverflowingCast<U>,
{
    /// 将 [`Region2D<T>`] 溢出转换为 [`Region2D<U>`]，如果 `T` 超出 `U` 的范围，则会返回溢出标志。
    fn overflowing_cast(self) -> (Region2D<U>, bool) {
        let (left, left_overflow) = self.left.overflowing_cast();
        let (top, top_overflow) = self.top.overflowing_cast();
        let (right, right_overflow) = self.right.overflowing_cast();
        let (bottom, bottom_overflow) = self.bottom.overflowing_cast();
        (
            Region2D::from_ltrb(left, top, right, bottom),
            left_overflow || top_overflow || right_overflow || bottom_overflow,
        )
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

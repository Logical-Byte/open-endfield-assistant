/// 原图中的像素矩形，使用左上角坐标与宽高表示半开区间。
///
/// 该值不自行校验。传给 `ImageRegion::new` 时检查非空、坐标溢出及图片范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PixelRect {
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

impl PixelRect {
    pub const fn new(left: u32, top: u32, width: u32, height: u32) -> Self {
        Self {
            left,
            top,
            width,
            height,
        }
    }
}

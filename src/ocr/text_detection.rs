use image::{GrayImage, RgbImage, imageops};
use imageproc::contrast::{ThresholdType, threshold};

use crate::utils::point::Region2D;

/// 检测单行连续文字（黑底白字）。
///
/// 简化版：只扫描全图找出所有白色像素的整体包围框，不做连通域分析和合并。
/// 适用于保证只有单行连续文字的场景。
pub fn detect_single_line(
    image: &RgbImage,
    threshold_value: u8,
    padding: u32,
) -> Option<Region2D<u32>> {
    let gray = imageops::grayscale(image);
    let binary = threshold(&gray, threshold_value, ThresholdType::Binary);

    let (w, h) = binary.dimensions();
    let mut min_x = u32::MAX;
    let mut min_y = u32::MAX;
    let mut max_x = u32::MIN;
    let mut max_y = u32::MIN;
    let mut found = false;

    for y in 0..h {
        for x in 0..w {
            if binary.get_pixel(x, y).0[0] == 255 {
                found = true;
                min_x = min_x.min(x);
                min_y = min_y.min(y);
                max_x = max_x.max(x);
                max_y = max_y.max(y);
            }
        }
    }

    if !found {
        return None;
    }

    let region = Region2D::from_ltrb(min_x, min_y, max_x + 1, max_y + 1);
    Some(apply_padding(region, padding, w, h))
}

/// 检测黑底白字图片中的文字区域。
///
/// 流程：
/// 1. 转灰度
/// 2. 二值化（高于阈值的像素为前景/文字）
/// 3. 连通域分析，找出每个文字的包围框
/// 4. 按面积过滤噪点
/// 5. 水平方向上合并相邻的包围框（同一行文字）
/// 6. 向外 padding
pub fn detect_white_text(
    image: &RgbImage,
    threshold_value: u8,
    padding: u32,
    min_area: u32,
    merge_max_gap: u32,
) -> Vec<Region2D<u32>> {
    // 1. 转灰度
    let gray = imageops::grayscale(image);

    // 2. 二值化：白字（高亮）→ 前景(255)，黑底 → 背景(0)
    let binary = threshold(&gray, threshold_value, ThresholdType::Binary);

    // 3. 连通域分析
    let components = find_connected_components(&binary);

    // 4. 按最小面积过滤
    let components: Vec<Region2D<u32>> = components
        .into_iter()
        .filter(|r| r.area() >= min_area)
        .collect();

    // 5. 水平合并同行且间距小于 merge_max_gap 的包围框
    let merged = merge_horizontal(components, merge_max_gap);

    // 6. 加 padding，并 clamp 到图像边界内
    let img_w = image.width();
    let img_h = image.height();
    merged
        .into_iter()
        .map(|r| apply_padding(r, padding, img_w, img_h))
        .collect()
}

/// 4-邻域连通域分析，返回每个连通分量的包围框
fn find_connected_components(binary: &GrayImage) -> Vec<Region2D<u32>> {
    let (w, h) = binary.dimensions();
    let w_usize = w as usize;
    let h_usize = h as usize;
    let total = w_usize * h_usize;
    let mut visited = vec![false; total];
    let mut regions = Vec::new();

    for y in 0..h_usize {
        for x in 0..w_usize {
            let idx = y * w_usize + x;
            if visited[idx] {
                continue;
            }
            if binary.get_pixel(x as u32, y as u32).0[0] != 255 {
                visited[idx] = true;
                continue;
            }
            // BFS 找连通域
            let mut stack = vec![(x, y)];
            visited[idx] = true;
            let mut min_x = x;
            let mut min_y = y;
            let mut max_x = x;
            let mut max_y = y;

            while let Some((cx, cy)) = stack.pop() {
                // 更新包围框
                min_x = min_x.min(cx);
                min_y = min_y.min(cy);
                max_x = max_x.max(cx);
                max_y = max_y.max(cy);

                // 4-邻域
                let neighbors = [
                    (cx.wrapping_sub(1), cy),
                    (cx + 1, cy),
                    (cx, cy.wrapping_sub(1)),
                    (cx, cy + 1),
                ];
                for &(nx, ny) in &neighbors {
                    if nx < w_usize && ny < h_usize {
                        let nidx = ny * w_usize + nx;
                        if !visited[nidx] && binary.get_pixel(nx as u32, ny as u32).0[0] == 255 {
                            visited[nidx] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
            }

            regions.push(Region2D::from_ltwh(
                min_x as u32,
                min_y as u32,
                (max_x - min_x + 1) as u32,
                (max_y - min_y + 1) as u32,
            ));
        }
    }

    regions
}

/// 合并水平方向上同行、间距 <= max_gap 的包围框
fn merge_horizontal(mut regions: Vec<Region2D<u32>>, max_gap: u32) -> Vec<Region2D<u32>> {
    if regions.is_empty() {
        return regions;
    }

    // 按 x 坐标排序
    regions.sort_by_key(|r| r.x0());

    let mut merged: Vec<Region2D<u32>> = Vec::new();
    let mut current = regions[0].clone();

    for next in regions.into_iter().skip(1) {
        // 判断是否在同一行（y 方向有重叠）
        let same_line = current.y0() <= next.y1() && next.y0() <= current.y1();

        let gap = if next.x0() >= current.x1() {
            next.x0() - current.x1()
        } else {
            0 // 本身就有重叠
        };

        if same_line && gap <= max_gap {
            // 合并
            let new_x = current.x0().min(next.x0());
            let new_y = current.y0().min(next.y0());
            let right = (current.x1()).max(next.x1());
            let bottom = (current.y1()).max(next.y1());
            current = Region2D::from_ltrb(new_x, new_y, right, bottom);
        } else {
            merged.push(current);
            current = next;
        }
    }
    merged.push(current);

    merged
}

/// 给包围框加 padding，并裁剪到图像边界内
fn apply_padding(region: Region2D<u32>, padding: u32, img_w: u32, img_h: u32) -> Region2D<u32> {
    let left = region.x0().saturating_sub(padding);
    let top = region.y0().saturating_sub(padding);
    let right = (region.x1() + padding).min(img_w);
    let bottom = (region.y1() + padding).min(img_h);

    Region2D::from_ltrb(left, top, right, bottom)
}

/// 根据 Region2D 从原图中裁剪出子图
pub fn crop_region(image: &RgbImage, region: Region2D<u32>) -> RgbImage {
    imageops::crop_imm(
        image,
        region.x0(),
        region.y0(),
        region.width(),
        region.height(),
    )
    .to_image()
}

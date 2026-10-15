//! 通用颜色转换与色相比较。饱和度下限、色相容差和候选颜色由调用方决定。

/// HSV 颜色。无彩色的色相记为零，调用方应结合饱和度判断色相是否有意义。
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code, reason = "通用识别 primitive，供后续工作流调用")]
pub(crate) struct Hsv {
    /// 色相角度，范围为 `[0, 360)`。
    pub hue_degrees: f32,
    /// 饱和度，范围为 `0..=1`。
    pub saturation: f32,
    /// 明度，范围为 `0..=1`。
    pub value: f32,
}

#[allow(dead_code, reason = "通用识别 primitive，供后续工作流调用")]
impl Hsv {
    /// 将 RGB 数组 `[R, G, B]` 转为 [`Hsv`]，输入各通道范围为 `0..=255`。
    ///
    /// 输入可以含小数。返回的色相以度表示，饱和度和明度范围为 `0..=1`，
    /// 不进行 OpenCV 的 8 位 HSV 量化。
    pub(crate) fn from_rgb([r, g, b]: [f32; 3]) -> Self {
        let max = r.max(g).max(b);
        let min = r.min(g).min(b);
        let delta = max - min;
        let hue_degrees = if delta == 0.0 {
            0.0
        } else if max == r {
            60.0 * ((g - b) / delta).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / delta + 2.0)
        } else {
            60.0 * ((r - g) / delta + 4.0)
        };

        Self {
            hue_degrees: hue_degrees.rem_euclid(360.0),
            saturation: if max == 0.0 { 0.0 } else { delta / max },
            value: max / 255.0,
        }
    }

    /// 两个色相沿色环的最短距离，范围为 `0..=180` 度。
    ///
    /// 例如 359 度与 1 度相距 2 度。无彩色也参与计算，饱和度过滤由调用方处理。
    pub(crate) fn hue_distance(self, other: Self) -> f32 {
        let distance = (self.hue_degrees - other.hue_degrees)
            .abs()
            .rem_euclid(360.0);
        distance.min(360.0 - distance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_rgb_with_fractional_values_and_correct_channel_order() {
        for (rgb, hue) in [
            ([255.0, 0.0, 0.0], 0.0),
            ([0.0, 255.0, 0.0], 120.0),
            ([0.0, 0.0, 255.0], 240.0),
            ([255.0, 127.5, 0.0], 30.0),
        ] {
            let hsv = Hsv::from_rgb(rgb);
            assert!((hsv.hue_degrees - hue).abs() < 0.001);
            assert_eq!(hsv.saturation, 1.0);
            assert_eq!(hsv.value, 1.0);
        }
    }

    #[test]
    fn distinguishes_achromatic_samples_from_saturated_colors() {
        assert_eq!(
            Hsv::from_rgb([0.0; 3]),
            Hsv {
                hue_degrees: 0.0,
                saturation: 0.0,
                value: 0.0
            }
        );
        let gray = Hsv::from_rgb([127.5; 3]);
        assert_eq!(gray.saturation, 0.0);
        assert_eq!(gray.value, 0.5);
        let muted = Hsv::from_rgb([200.0, 150.0, 100.0]);
        assert_eq!(muted.hue_degrees, 30.0);
        assert_eq!(muted.saturation, 0.5);
    }

    #[test]
    fn compares_hues_across_the_wraparound() {
        let red = Hsv::from_rgb([255.0, 0.0, 0.0]);
        let above = Hsv::from_rgb([255.0, 4.25, 0.0]);
        let below = Hsv::from_rgb([255.0, 0.0, 4.25]);
        assert!((above.hue_distance(below) - 2.0).abs() < 0.001);
        assert_eq!(above.hue_distance(below), below.hue_distance(above));
        assert_eq!(red.hue_distance(Hsv::from_rgb([0.0, 255.0, 255.0])), 180.0);
    }
}

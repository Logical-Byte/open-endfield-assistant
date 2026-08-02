use image::{GrayImage, ImageBuffer, Luma};
use imageproc::{
    definitions::Image,
    integral_image,
    template_matching::{self, MatchTemplateMethod},
};

// —— CCOEFF_NORMED 实现

/// 预计算的模板统计量
struct CcoeffNormalized {
    template_mean: f64,
    template_var: f64, // ∑(T - T̄)²
    area: f64,
}

impl CcoeffNormalized {
    fn new(template: &GrayImage) -> Self {
        let (tw, th) = template.dimensions();
        let area = (tw * th) as f64;

        let mut t_sum = 0f64;
        let mut t_sq_sum = 0f64;
        for y in 0..th {
            for x in 0..tw {
                let v = template.get_pixel(x, y).0[0] as f64;
                t_sum += v;
                t_sq_sum += v * v;
            }
        }
        let template_mean = t_sum / area;
        let template_var = (t_sq_sum - t_sum * template_mean).max(0.0);

        Self {
            template_mean,
            template_var,
            area,
        }
    }

    fn score_at(
        &self,
        x: u32,
        y: u32,
        tw: u32,
        th: u32,
        integral: &IntegralImage,
        integral_sq: &IntegralSqImage,
        ccorr: &Image<Luma<f32>>,
    ) -> f32 {
        let i_mean = patch_mean(integral, x, y, tw, th);
        let numerator = ccorr.get_pixel(x, y).0[0] as f64 - self.area * self.template_mean * i_mean;

        let i_var = patch_variance(integral, integral_sq, x, y, tw, th);
        let denominator = (self.template_var * i_var).sqrt();

        if denominator < 1e-10 {
            0.0
        } else {
            (numerator / denominator).clamp(-1.0, 1.0) as f32
        }
    }

    fn match_template_parallel(&self, image: &GrayImage, template: &GrayImage) -> Image<Luma<f32>> {
        let (sw, sh) = image.dimensions();
        let (tw, th) = template.dimensions();
        let rw = sw - tw + 1;
        let rh = sh - th + 1;

        let integral = integral_image::integral_image(image);
        let integral_sq = integral_image::integral_squared_image(image);
        let ccorr = template_matching::match_template_parallel(
            image,
            template,
            MatchTemplateMethod::CrossCorrelation,
        );

        Image::from_fn(rw, rh, |x, y| {
            let score = self.score_at(x, y, tw, th, &integral, &integral_sq, &ccorr);
            Luma([score])
        })
    }
}

/// 仿 imageproc `match_template` 风格的 CCOEFF_NORMED 匹配。
///
/// R(x,y) = (∑T·I - n·T̄·Ī) / √(∑(T-T̄)² · ∑(I-Ī)²)
///
/// 利用 CrossCorrelation 拿到 ∑T·I，配合积分图 O(1) 取 ∑I/∑I²。
pub fn match_template_ccoeff_normed_parallel(
    image: &GrayImage,
    template: &GrayImage,
) -> Image<Luma<f32>> {
    let method = CcoeffNormalized::new(template);
    method.match_template_parallel(image, template)
}

// —— 积分图辅助 ——

type IntegralImage = ImageBuffer<Luma<u32>, Vec<u32>>;
type IntegralSqImage = ImageBuffer<Luma<u64>, Vec<u64>>;

fn patch_sum(integral: &IntegralImage, x: u32, y: u32, w: u32, h: u32) -> u64 {
    let x2 = x + w;
    let y2 = y + h;
    let a = integral.get_pixel(x, y).0[0] as i64;
    let b = integral.get_pixel(x2, y).0[0] as i64;
    let c = integral.get_pixel(x, y2).0[0] as i64;
    let d = integral.get_pixel(x2, y2).0[0] as i64;
    (d - b - c + a) as u64
}

fn patch_sum_sq(integral_sq: &IntegralSqImage, x: u32, y: u32, w: u32, h: u32) -> u64 {
    let x2 = x + w;
    let y2 = y + h;
    let a = integral_sq.get_pixel(x, y).0[0] as i128;
    let b = integral_sq.get_pixel(x2, y).0[0] as i128;
    let c = integral_sq.get_pixel(x, y2).0[0] as i128;
    let d = integral_sq.get_pixel(x2, y2).0[0] as i128;
    (d - b - c + a) as u64
}

fn patch_mean(integral: &IntegralImage, x: u32, y: u32, w: u32, h: u32) -> f64 {
    patch_sum(integral, x, y, w, h) as f64 / (w * h) as f64
}

fn patch_variance(
    integral: &IntegralImage,
    integral_sq: &IntegralSqImage,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> f64 {
    let n = (w * h) as f64;
    let sum_i = patch_sum(integral, x, y, w, h) as f64;
    let sum_i2 = patch_sum_sq(integral_sq, x, y, w, h) as f64;
    (sum_i2 - sum_i * sum_i / n).max(0.0)
}

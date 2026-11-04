use image::{GrayImage, Luma, Rgba, RgbaImage};
use oea_vision::{ImageRegion, PixelRect, statistics::mean_luma};

#[test]
fn measures_only_the_requested_region_and_ignores_alpha() {
    let mut image = RgbaImage::from_pixel(4, 3, Rgba([255, 0, 0, 255]));
    image.put_pixel(1, 1, Rgba([20, 40, 60, 0]));
    image.put_pixel(2, 1, Rgba([40, 60, 80, 255]));
    let region = ImageRegion::new(&image, PixelRect::new(1, 1, 2, 1)).unwrap();

    assert_eq!(mean_luma(&region), 47.0);
}

#[test]
fn preserves_fractional_brightness_of_grayscale_samples() {
    let image = GrayImage::from_fn(2, 1, |x, _| Luma([200 + x as u8]));
    assert_eq!(
        mean_luma(&ImageRegion::new(&image, PixelRect::new(0, 0, 2, 1)).unwrap()),
        200.5
    );
}

use image::{Rgb, RgbImage};
use oea_vision::{ImageRegion, PixelRect, template};

#[test]
fn maps_search_view_coordinates_back_to_the_original_image() {
    let template = RgbImage::from_fn(2, 2, |x, y| Rgb([30 + (x + y * 2) as u8 * 50; 3]));
    let mut image = RgbImage::new(6, 5);
    image::imageops::replace(&mut image, &template, 3, 2);
    let region = ImageRegion::new(&image, PixelRect::new(2, 1, 4, 4)).unwrap();
    let matched = template::match_(&region, &template).unwrap();
    assert_eq!(matched.region, PixelRect::new(3, 2, 2, 2));
    assert!((matched.score - 1.0).abs() < 0.001);
}

#[test]
fn rejects_a_template_larger_than_the_valid_search_region() {
    let image = RgbImage::new(2, 2);
    let region = ImageRegion::new(&image, PixelRect::new(0, 0, 1, 1)).unwrap();
    assert!(template::match_(&region, &image).is_err());
}

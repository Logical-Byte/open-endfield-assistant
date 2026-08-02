use anyhow::Result;
use image::{GenericImageView, imageops};

fn main() -> Result<()> {
    let image = image::open(r#"C:\Users\Administrator\Desktop\PixPin_2026-07-27_19-51-32.png"#)?;

    let image_shape = image.dimensions();

    let subimage = imageops::crop_imm(&image, 100, 100, 200, 200);

    let subimage_shape = subimage.inner().dimensions();

    dbg!(image_shape, subimage_shape);

    Ok(())
}

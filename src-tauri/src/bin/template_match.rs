use anyhow::Result;
use app_lib::utils::timeit::timeit_print;
use image::imageops;
use imageproc::template_matching::{self, MatchTemplateMethod};

fn main() -> Result<()> {
    let image1 = image::open(r#"C:\Users\Administrator\Desktop\PixPin_2026-07-27_19-51-32.png"#)?;
    let image2 = image::open(r#"C:\Users\Administrator\Desktop\PixPin_2026-07-27_21-16-59.png"#)?;
    let template = image::open("templates/下一篇.png")?;

    for image in [&image1, &image2] {
        let gray_image = imageops::grayscale(image);
        let gray_template = imageops::grayscale(&template);

        let result = timeit_print(
            || {
                app_lib::template_matching::match_template_ccoeff_normed_parallel(
                    &gray_image,
                    &gray_template,
                )
            },
            "CCOEFF_NORMED",
        );
        let extreme = template_matching::find_extremes(&result);
        dbg!(extreme);

        let result = timeit_print(
            || {
                template_matching::match_template_parallel(
                    &gray_image,
                    &gray_template,
                    MatchTemplateMethod::CrossCorrelationNormalized,
                )
            },
            "CCORR_NORMED",
        );
        let extreme = template_matching::find_extremes(&result);
        dbg!(extreme);
    }

    Ok(())
}

pub fn scale_coordinate(
    base_coordinate: (i32, i32),
    base_resolution: (i32, i32),
    actual_resolution: (i32, i32),
) -> (i32, i32) {
    if actual_resolution.0 <= 0
        || actual_resolution.1 <= 0
        || base_resolution.0 <= 0
        || base_resolution.1 <= 0
    {
        return base_coordinate;
    }

    let x =
        (base_coordinate.0 as f64 * actual_resolution.0 as f64 / base_resolution.0 as f64) as i32;
    let y =
        (base_coordinate.1 as f64 * actual_resolution.1 as f64 / base_resolution.1 as f64) as i32;
    (x, y)
}

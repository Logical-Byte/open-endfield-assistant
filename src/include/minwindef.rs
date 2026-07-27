#[macro_export]
macro_rules! MAKELONG {
    ($a:expr, $b:expr) => {
        ($a as u32 & 0xFFFF) | (($b as u32 & 0xFFFF) << 16)
    };
}

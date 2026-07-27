#[macro_export]
macro_rules! MAKEWPARAM {
    ($l:expr, $h:expr) => {
        WPARAM(MAKELONG!($l, $h) as usize)
    };
}

#[macro_export]
macro_rules! MAKELPARAM {
    ($l:expr, $h:expr) => {
        LPARAM(MAKELONG!($l, $h) as isize)
    };
}

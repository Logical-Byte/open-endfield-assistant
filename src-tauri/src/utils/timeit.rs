use std::time::{Duration, Instant};
use tracing::debug;

pub fn timeit<F, R>(func: F) -> (R, Duration)
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = func();
    let duration = start.elapsed();
    (result, duration)
}

pub fn timeit_print<F, R>(func: F, label: &str) -> R
where
    F: FnOnce() -> R,
{
    let (result, duration) = timeit(func);
    debug!("{} took {:?}", label, duration);
    result
}

#[macro_export]
macro_rules! timeit_print {
    ($label:expr, $($t:tt)*) => {
        let start = std::time::Instant::now();
        $($t)*
        let duration = start.elapsed();
        debug!("{} took {:?}", $label, duration);
    };
}

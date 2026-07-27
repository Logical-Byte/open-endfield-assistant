pub trait Cast<T> {
    fn cast(self) -> T;
}

macro_rules! impl_cast {
    ($from:ty => $($to:ty),+) => {
        $(
            impl Cast<$to> for $from {
                #[inline]
                fn cast(self) -> $to { self as $to }
            }
        )+
    };
}

impl_cast!(u8    => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
impl_cast!(u16   => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
impl_cast!(u32   => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
impl_cast!(u64   => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
impl_cast!(usize => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
impl_cast!(i8    => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
impl_cast!(i16   => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
impl_cast!(i32   => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
impl_cast!(i64   => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
impl_cast!(isize => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
impl_cast!(f32   => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);
impl_cast!(f64   => u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64);

use windows::Win32::Foundation::RECT;

pub trait Rect {
    fn width(&self) -> i32;
    fn height(&self) -> i32;
}

impl Rect for RECT {
    fn width(&self) -> i32 {
        self.right - self.left
    }

    fn height(&self) -> i32 {
        self.bottom - self.top
    }
}

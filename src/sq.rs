#[derive(Debug, Copy, Clone, Default)]
pub struct Queue {}

impl Queue {
    #[inline]
    pub fn new() -> Self {
        Default::default()
    }
}

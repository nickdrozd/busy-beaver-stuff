use crate::instrs::Color;

pub type Count = u64;

pub struct Block {
    pub color: Color,
    pub count: Count,
}

impl Block {
    pub const fn new(color: Color, count: Count) -> Self {
        Self { color, count }
    }
}

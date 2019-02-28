/// A marker trait for anything that can be used as an index
pub trait Index: Copy + Send + Sync {}

impl Index for u16 {}
impl Index for u32 {}

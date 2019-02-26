
pub trait Index: Copy + Send + Sync {}

impl Index for u16 {}
impl Index for u32 {}
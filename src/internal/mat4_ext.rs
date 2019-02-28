use vek::Mat4;

pub trait Mat4Ext<T> {
    unsafe fn as_push_constant_data(&mut self) -> &mut [u32];
}

impl<T> Mat4Ext<T> for Mat4<T> {
    unsafe fn as_push_constant_data(&mut self) -> &mut [u32] {
        &mut *(self.as_mut_col_slice() as *mut [T] as *mut [u32])
    }
}

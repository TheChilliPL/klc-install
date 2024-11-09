pub trait AsU16Slice {
    fn as_u16_slice(&self) -> &[u16];
    fn as_mut_u16_slice(&mut self) -> &mut [u16];
}

impl AsU16Slice for Vec<u8> {
    fn as_u16_slice<'a>(&'a self) -> &'a [u16] {
        unsafe { std::slice::from_raw_parts(self.as_ptr() as *const u16, self.len() / 2) }
    }

    fn as_mut_u16_slice<'a>(&'a mut self) -> &'a mut [u16] {
        unsafe { std::slice::from_raw_parts_mut(self.as_mut_ptr() as *mut u16, self.len() / 2) }
    }
}

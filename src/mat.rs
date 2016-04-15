use libc::c_void;

extern "C" {
    #[doc(hidden)] pub fn cv_core_Mat_at_int_i_float(mat: *const c_void, i: i32) -> f32;
}

impl ::core::Mat {
    pub fn at(&self, i: i32) -> f32 {
        unsafe {
            cv_core_Mat_at_int_i_float(self.as_raw_Mat(), i)
        }
    }
}

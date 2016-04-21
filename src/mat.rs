use libc::c_void;

extern "C" {
    #[doc(hidden)] pub fn cv_core_Mat_at_int_i_float(mat: *const c_void, i: i32) -> f32;
    #[doc(hidden)] pub fn cv_core_Mat_at_int_i_double(mat: *const c_void, i: i64) -> f64;
}

impl ::core::Mat {
    pub fn at_f32(&self, i: i32) -> f32 {
        unsafe {
            cv_core_Mat_at_int_i_float(self.as_raw_Mat(), i)
        }
    }
    
    pub fn at_f64(&self, i: i64) -> f64 {
        unsafe {
            cv_core_Mat_at_int_i_double(self.as_raw_Mat(), i)
        }
    }
}

use std::ffi::CStr;
use std::mem;
use libc::c_void;

use sys::cv_return_value_void_X;

extern "C" {
    #[doc(hidden)] pub fn cv_core_Mat_at_int_i_float(mat: *const c_void, i: i32) -> f32;
    #[doc(hidden)] pub fn cv_core_Mat_at_int_i_int_j_float(mat: *const c_void, i: i32, j: i32) -> f32;
    #[doc(hidden)] pub fn cv_core_Mat_at_int_i_int_j_int(mat: *const c_void, i: i32, j: i32) -> i32;
    #[doc(hidden)] pub fn cv_core_Mat_at_int_i_double(mat: *const c_void, i: i64) -> f64;
    #[doc(hidden)] pub fn cv_core_Mat_Mat_rows_cols_type_data(rows: i32, cols: i32, dtype: i32, data: *const c_void, step: usize) -> cv_return_value_void_X;
}

impl ::core::Mat {
    pub fn at_f32(&self, i: i32) -> f32 {
        unsafe {
            cv_core_Mat_at_int_i_float(self.as_raw_Mat(), i)
        }
    }
    
    pub fn at_f32_2(&self, i: i32, j: i32) -> f32 {
        unsafe {
            cv_core_Mat_at_int_i_int_j_float(self.as_raw_Mat(), i, j)
        }
    }
    
    pub fn at_i32_2(&self, i: i32, j: i32) -> i32 {
        unsafe {
            cv_core_Mat_at_int_i_int_j_int(self.as_raw_Mat(), i, j)
        }
    }
    
    pub fn at_f64(&self, i: i64) -> f64 {
        unsafe {
            cv_core_Mat_at_int_i_double(self.as_raw_Mat(), i)
        }
    }
    
    pub fn from_raw_parts(rows: i32, cols: i32, dtype: i32, data: &[u8], step: usize) -> Result<::core::Mat, String> {
        unsafe {
            let rv = cv_core_Mat_Mat_rows_cols_type_data(rows, cols, dtype, data.as_ptr() as *const c_void, step);
            if rv.error_msg as i32 != 0i32 {
                let v = CStr::from_ptr(rv.error_msg).to_bytes().to_vec();
                ::libc::free(rv.error_msg as *mut c_void);
                return Err(String::from_utf8(v).unwrap())
            }
            Ok(::core::Mat{ ptr: rv.result, owned: true })
        }
    }
}

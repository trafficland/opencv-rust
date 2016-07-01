#include "common_opencv.h"
#include "types.h"

using namespace cv;

extern "C" {
  #include "return_types.h"
  float cv_core_Mat_at_int_i_float(void* mat, int i) {
    return ((cv::Mat*)mat)->at<float>(i);
  }
  
  float cv_core_Mat_at_int_i_int_j_float(void* mat, int i, int j) {
    return ((cv::Mat*)mat)->at<float>(i, j);
  }
  
  int cv_core_Mat_at_int_i_int_j_int(void* mat, int i, int j) {
    return ((cv::Mat*)mat)->at<int>(i, j);
  }
  
  double cv_core_Mat_at_int_i_double(void* mat, int i) {
    return ((cv::Mat*)mat)->at<double>(i);
  }
  
  struct cv_return_value_void_X cv_core_Mat_Mat_rows_cols_type_data(int rows, int cols, int type, void* data, size_t step) {
    try {
      cv::Mat* cpp_return_value = new cv::Mat(rows, cols, type, data, step);
      return { NULL, (void*) cpp_return_value }; 
    } catch (cv::Exception& e) {
      char* msg = strdup(e.what());
      struct cv_return_value_void_X ret;
      memset(&ret, 0x00, sizeof(ret));
      ret.error_msg = msg;
      return ret;
    } catch (...) {
      char* msg = strdup("unspecified error in OpenCV guts");
      struct cv_return_value_void_X ret;
      memset(&ret, 0x00, sizeof(ret));
      ret.error_msg = msg;
      return ret;
    }
  }
}

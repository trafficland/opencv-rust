#include "common_opencv.h"

using namespace cv;

extern "C" {
  float cv_core_Mat_at_int_i_float(void* mat, int i) {
    return ((cv::Mat*)mat)->at<float>(i);
  }
  
  double cv_core_Mat_at_int_i_double(void* mat, int i) {
    return ((cv::Mat*)mat)->at<double>(i);
  }
}

#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("application.h");
        type QApplication;

        fn new_application(args: &[String]) -> UniquePtr<QApplication>;
        fn exec_application(app: Pin<&mut QApplication>) -> i32;
    }
}

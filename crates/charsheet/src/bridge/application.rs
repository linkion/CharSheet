#[cxx_qt::bridge]
pub mod ffi {
    unsafe extern "C++" {
        include!("application.h");
        type QApplication;

        fn new_application(args: &[String]) -> UniquePtr<QApplication>;
        fn exec_application(app: Pin<&mut QApplication>) -> i32;

        include!("theme.h");
        fn register_fonts();
        /// "stone", "parchment" or "system".
        fn apply_theme(mode: &str);
        /// The desktop's UI font, which the System theme uses.
        fn system_font_family() -> String;
        fn saved_theme() -> String;
        fn save_theme(mode: &str);
    }
}

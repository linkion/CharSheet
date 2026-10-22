//! The theme picker: switches between the Stone, Parchment and System looks.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[qml_singleton]
        /// "stone", "parchment" or "system".
        #[qproperty(QString, mode, READ, NOTIFY)]
        /// The desktop's UI font family, for the System theme.
        #[qproperty(QString, system_family, cxx_name = "systemFamily", READ, CONSTANT)]
        type ThemeSettings = super::ThemeSettingsRust;

        /// Applies a theme now and remembers it for the next start.
        #[qinvokable]
        fn choose(self: Pin<&mut Self>, mode: &QString);
    }
}

use super::application::ffi::{apply_theme, save_theme, saved_theme, system_font_family};
use cxx_qt::CxxQtType;
use cxx_qt_lib::QString;
use std::pin::Pin;

pub struct ThemeSettingsRust {
    mode: QString,
    system_family: QString,
}

impl Default for ThemeSettingsRust {
    fn default() -> Self {
        ThemeSettingsRust {
            mode: QString::from(&normalize(&saved_theme())),
            system_family: QString::from(&system_font_family()),
        }
    }
}

fn normalize(mode: &str) -> String {
    match mode {
        "stone" | "parchment" | "system" => mode.to_owned(),
        _ => "stone".to_owned(),
    }
}

impl qobject::ThemeSettings {
    fn choose(mut self: Pin<&mut Self>, mode: &QString) {
        let mode = normalize(&mode.to_string());
        apply_theme(&mode);
        save_theme(&mode);
        self.as_mut().rust_mut().get_mut().mode = QString::from(&mode);
        self.mode_changed();
    }
}

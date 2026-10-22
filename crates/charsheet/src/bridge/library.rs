//! The character library: the list of saved characters and file-level operations.

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qurl.h");
        type QUrl = cxx_qt_lib::QUrl;
    }

    extern "RustQt" {
        #[qobject]
        #[qml_element]
        /// JSON array of character summaries.
        #[qproperty(QString, entries, READ, NOTIFY)]
        #[qproperty(QString, last_error, cxx_name = "lastError", READ, NOTIFY)]
        type CharacterLibrary = super::CharacterLibraryRust;

        #[qinvokable]
        fn refresh(self: Pin<&mut Self>);

        /// Creates a character and returns its id, or "" on failure.
        #[qinvokable]
        fn create(self: Pin<&mut Self>, name: &QString) -> QString;

        #[qinvokable]
        fn duplicate(self: Pin<&mut Self>, id: &QString) -> QString;

        #[qinvokable]
        fn remove(self: Pin<&mut Self>, id: &QString) -> bool;

        #[qinvokable]
        #[cxx_name = "importFile"]
        fn import_file(self: Pin<&mut Self>, url: &QUrl) -> QString;

        #[qinvokable]
        #[cxx_name = "exportFile"]
        fn export_file(self: Pin<&mut Self>, id: &QString, url: &QUrl) -> bool;
    }
}

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QString, QUrl};
use dnd_core::Library;
use dnd_core::storage::{StorageError, Summary};
use std::path::PathBuf;
use std::pin::Pin;
use uuid::Uuid;

pub struct CharacterLibraryRust {
    entries: QString,
    last_error: QString,
    library: Result<Library, String>,
}

impl Default for CharacterLibraryRust {
    fn default() -> Self {
        let library = Library::open_default().map_err(|e| e.to_string());
        let mut this = CharacterLibraryRust {
            entries: QString::from("[]"),
            last_error: QString::default(),
            library,
        };
        this.entries = QString::from(&this.entries_json());
        if let Err(e) = &this.library {
            this.last_error = QString::from(&format!("Cannot open the character library: {e}"));
        }
        this
    }
}

impl CharacterLibraryRust {
    fn entries_json(&self) -> String {
        let summaries: Vec<Summary> = match &self.library {
            Ok(lib) => lib.list().unwrap_or_default(),
            Err(_) => Vec::new(),
        };
        serde_json::to_string(&summaries).unwrap_or_else(|_| "[]".into())
    }
}

fn parse_id(id: &QString) -> Result<Uuid, String> {
    Uuid::parse_str(&id.to_string()).map_err(|e| format!("Invalid character id: {e}"))
}

fn local_path(url: &QUrl) -> Result<PathBuf, String> {
    url.to_local_file()
        .map(|s| PathBuf::from(s.to_string()))
        .filter(|p| !p.as_os_str().is_empty())
        .ok_or_else(|| "Only local files are supported".to_owned())
}

impl qobject::CharacterLibrary {
    /// Runs a library operation, recording any error and refreshing the list on success.
    fn run<T>(
        mut self: Pin<&mut Self>,
        op: impl FnOnce(&Library) -> Result<T, String>,
    ) -> Option<T> {
        let result = match &self.rust().library {
            Ok(lib) => op(lib),
            Err(e) => Err(e.clone()),
        };
        match result {
            Ok(value) => {
                self.as_mut().set_error("");
                self.as_mut().refresh();
                Some(value)
            }
            Err(e) => {
                self.as_mut().set_error(&e);
                None
            }
        }
    }

    fn set_error(mut self: Pin<&mut Self>, message: &str) {
        let message = QString::from(message);
        if self.rust().last_error != message {
            self.as_mut().rust_mut().get_mut().last_error = message;
            self.last_error_changed();
        }
    }

    pub fn refresh(mut self: Pin<&mut Self>) {
        let json = QString::from(&self.rust().entries_json());
        if self.rust().entries != json {
            self.as_mut().rust_mut().get_mut().entries = json;
            self.entries_changed();
        }
    }

    pub fn create(self: Pin<&mut Self>, name: &QString) -> QString {
        let name = name.to_string();
        let name = if name.trim().is_empty() {
            "New Adventurer".to_owned()
        } else {
            name
        };
        self.run(|lib| lib.create(&name).map_err(|e| e.to_string()))
            .map(|c| QString::from(&c.id.to_string()))
            .unwrap_or_default()
    }

    pub fn duplicate(self: Pin<&mut Self>, id: &QString) -> QString {
        self.run(|lib| lib.duplicate(parse_id(id)?).map_err(|e| e.to_string()))
            .map(|c| QString::from(&c.id.to_string()))
            .unwrap_or_default()
    }

    pub fn remove(self: Pin<&mut Self>, id: &QString) -> bool {
        self.run(|lib| lib.delete(parse_id(id)?).map_err(|e| e.to_string()))
            .is_some()
    }

    pub fn import_file(self: Pin<&mut Self>, url: &QUrl) -> QString {
        self.run(|lib| {
            lib.import(&local_path(url)?)
                .map_err(|e: StorageError| format!("Import failed: {e}"))
        })
        .map(|c| QString::from(&c.id.to_string()))
        .unwrap_or_default()
    }

    pub fn export_file(self: Pin<&mut Self>, id: &QString, url: &QUrl) -> bool {
        self.run(|lib| {
            lib.export(parse_id(id)?, &local_path(url)?)
                .map_err(|e| format!("Export failed: {e}"))
        })
        .is_some()
    }
}

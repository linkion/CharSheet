//! Tells QML when another process, such as the MCP server, has changed the library.

#[cxx_qt::bridge]
pub mod qobject {
    extern "RustQt" {
        #[qobject]
        #[qml_element]
        type LibraryWatcher = super::LibraryWatcherRust;

        #[qsignal]
        #[cxx_name = "libraryChanged"]
        fn library_changed(self: Pin<&mut Self>);
    }

    impl cxx_qt::Threading for LibraryWatcher {}
    impl cxx_qt::Initialize for LibraryWatcher {}
}

use cxx_qt::Threading;
use dnd_core::notify::Listener;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Default)]
pub struct LibraryWatcherRust;

impl cxx_qt::Initialize for qobject::LibraryWatcher {
    fn initialize(self: core::pin::Pin<&mut Self>) {
        let listener = match Listener::bind() {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("charsheet: not listening for library changes: {e}");
                return;
            }
        };
        let qt_thread = self.qt_thread();
        // Set while a signal is queued, so a burst of notifications makes only one.
        let pending = Arc::new(AtomicBool::new(false));
        std::thread::spawn(move || {
            while listener.wait().is_ok() {
                if pending.swap(true, Ordering::AcqRel) {
                    continue;
                }
                let pending = pending.clone();
                let queued = qt_thread.queue(move |watcher| {
                    pending.store(false, Ordering::Release);
                    watcher.library_changed();
                });
                if queued.is_err() {
                    break;
                }
            }
        });
    }
}

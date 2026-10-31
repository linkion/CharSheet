//! Wakes running apps when another process has changed the library.
//!
//! Each app binds a Unix datagram socket in a shared directory, and a writer sends one
//! datagram to every socket there. A datagram carries no data: it only tells the app to look
//! for changes now, so a lost or needless one does no harm.

use std::io::{self, ErrorKind};
use std::os::unix::net::UnixDatagram;
use std::path::{Path, PathBuf};

const EXTENSION: &str = "sock";

/// Where apps bind their sockets, if this session has a runtime directory.
fn default_dir() -> Option<PathBuf> {
    dirs::runtime_dir().map(|dir| dir.join("charsheet"))
}

/// Tells running apps that the library changed.
#[derive(Clone, Debug)]
pub struct Notifier {
    dir: Option<PathBuf>,
}

impl Notifier {
    /// A notifier for the apps in this session.
    pub fn new() -> Notifier {
        Notifier { dir: default_dir() }
    }

    pub fn in_dir(dir: impl Into<PathBuf>) -> Notifier {
        Notifier {
            dir: Some(dir.into()),
        }
    }

    /// A notifier that does nothing.
    pub fn disabled() -> Notifier {
        Notifier { dir: None }
    }

    /// Wakes every app, removing the sockets of those that have exited. This never fails:
    /// the change is already saved, and apps also check for changes on a timer.
    pub fn notify(&self) {
        let Some(entries) = self.dir.as_ref().and_then(|dir| dir.read_dir().ok()) else {
            return;
        };
        let Ok(socket) = UnixDatagram::unbound() else {
            return;
        };
        if socket.set_nonblocking(true).is_err() {
            return;
        }
        for path in entries.flatten().map(|entry| entry.path()) {
            if path.extension().is_none_or(|e| e != EXTENSION) {
                continue;
            }
            if let Err(e) = socket.send_to(&[0], &path)
                && e.kind() == ErrorKind::ConnectionRefused
            {
                let _ = std::fs::remove_file(&path);
            }
        }
    }
}

impl Default for Notifier {
    fn default() -> Self {
        Notifier::new()
    }
}

/// An app's end of the connection.
#[derive(Debug)]
pub struct Listener {
    socket: UnixDatagram,
}

impl Listener {
    /// Listens for notifications in this session.
    pub fn bind() -> io::Result<Listener> {
        let dir = default_dir()
            .ok_or_else(|| io::Error::new(ErrorKind::NotFound, "no runtime directory"))?;
        Listener::bind_in(&dir, std::process::id())
    }

    /// Listens in `dir`, under a name unique to `id`.
    pub fn bind_in(dir: &Path, id: u32) -> io::Result<Listener> {
        std::fs::create_dir_all(dir)?;
        let path = dir.join(format!("app-{id}.{EXTENSION}"));
        // Left by an earlier process with the same pid.
        match std::fs::remove_file(&path) {
            Err(e) if e.kind() != ErrorKind::NotFound => return Err(e),
            _ => {}
        }
        Ok(Listener {
            socket: UnixDatagram::bind(path)?,
        })
    }

    /// Blocks until a notification arrives.
    pub fn wait(&self) -> io::Result<()> {
        self.socket.recv(&mut [0]).map(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn received(listener: &Listener) -> bool {
        listener
            .socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .unwrap();
        listener.wait().is_ok()
    }

    #[test]
    fn wakes_every_listener_once_per_notification() {
        let dir = tempfile::tempdir().unwrap();
        let first = Listener::bind_in(dir.path(), 1).unwrap();
        let second = Listener::bind_in(dir.path(), 2).unwrap();
        Notifier::in_dir(dir.path()).notify();
        assert!(received(&first));
        assert!(received(&second));
        assert!(!received(&first));
    }

    #[test]
    fn removes_sockets_of_exited_apps_and_still_wakes_the_rest() {
        let dir = tempfile::tempdir().unwrap();
        drop(Listener::bind_in(dir.path(), 1).unwrap());
        let live = Listener::bind_in(dir.path(), 2).unwrap();
        Notifier::in_dir(dir.path()).notify();
        assert!(!dir.path().join("app-1.sock").exists());
        assert!(dir.path().join("app-2.sock").exists());
        assert!(received(&live));
    }

    #[test]
    fn ignores_other_files_and_keeps_them() {
        let dir = tempfile::tempdir().unwrap();
        let other = dir.path().join("notes.txt");
        std::fs::write(&other, "keep").unwrap();
        Notifier::in_dir(dir.path()).notify();
        assert!(other.exists());
    }

    #[test]
    fn rebinds_over_a_socket_left_by_an_earlier_process() {
        let dir = tempfile::tempdir().unwrap();
        drop(Listener::bind_in(dir.path(), 1).unwrap());
        let listener = Listener::bind_in(dir.path(), 1).unwrap();
        Notifier::in_dir(dir.path()).notify();
        assert!(received(&listener));
    }

    #[test]
    fn does_nothing_without_a_directory() {
        Notifier::disabled().notify();
        let dir = tempfile::tempdir().unwrap();
        Notifier::in_dir(dir.path().join("missing")).notify();
    }

    #[test]
    fn a_full_queue_does_not_block_the_writer() {
        let dir = tempfile::tempdir().unwrap();
        let _listener = Listener::bind_in(dir.path(), 1).unwrap();
        let notifier = Notifier::in_dir(dir.path());
        for _ in 0..1000 {
            notifier.notify();
        }
        assert!(dir.path().join("app-1.sock").exists());
    }
}

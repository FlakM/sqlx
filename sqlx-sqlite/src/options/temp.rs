use std::fmt::{Debug, Formatter};
use std::io;
use std::path::Path;
use std::sync::Arc;

use once_cell::sync::OnceCell;

#[cfg(doc)]
use {
    crate::{SqliteConnectOptions, SqliteConnection}
};

/// Handle tracking a named, temporary path for a SQLite database.
///
/// The directory and its contents, including the database file as well as any temporary files
/// created by SQLite, will be deleted when the last handle is dropped.
///
/// [`SqliteConnectOptions`] will retain a handle, as well as any [`SqliteConnection`]s it creates.
#[derive(Clone)]
pub struct SqliteTempPath {
    inner: Arc<OnceCell<tempfile::TempDir>>,
}

struct TempDbPath {}

impl SqliteTempPath {
    /// Create a handle that will lazily create the temporary directory on first connection.
    pub fn lazy() -> Self {
        Self {
            inner: Arc::new(OnceCell::new())
        }
    }

    /// Create a temporary directory immediately, returning the handle.
    ///
    /// This will spawn a blocking task in the current runtime.
    ///
    /// ### Panics
    /// If no runtime is available.
    pub async fn create() -> io::Result<Self> {
        let this = Self::lazy();
        this.force_create().await?;
        Ok(this)
    }

    /// Create a handle from a custom [`tempfile::TempDir`].
    ///
    ///
    pub fn from_tempdir(tempdir: tempfile::TempDir) -> Self {
        Self {
            inner: Arc::new(OnceCell::with_value(tempdir)),
        }
    }

    /// Create a temporary directory for this handle immediately, returning the created path.
    ///
    /// If the directory has already been created, this simply returns the path.
    ///
    /// This will spawn a blocking task in the current runtime to create the directory.
    ///
    /// ### Panics
    /// If no runtime is available.
    pub async fn force_create(&self) -> io::Result<&Path> {
        let this = self.clone();

        sqlx_core::rt::spawn_blocking(move || {
            this.force_create_blocking().map(|_| ())
        }).await?;

        Ok(
            self.inner
                .get()
                .expect("BUG: `self.inner` should be initialized at this point!")
                .path()
        )
    }

    /// Create a temporary directory for this handle immediately, returning the created path.
    ///
    /// If the directory has already been created, this simply returns the path.
    pub fn force_create_blocking(&self) -> io::Result<&Path> {
        self.inner.get_or_try_init(|| {
            tempfile::Builder::new()
                .prefix("sqlx-sqlite")
                .suffix(".db")
                .tempdir()
        })
    }
}

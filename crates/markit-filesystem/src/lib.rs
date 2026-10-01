//! Markit filesystem capability: the local-file read/write mechanism for
//! the first desktop product. The filesystem owns the mechanism, path
//! handling, and OS error results; document truth, revision identity, and
//! the dirty relation stay in the document crate. Saving is atomic per
//! file (write to a sibling temp file, then rename). No watcher, no VFS,
//! no cloud, no remote URIs — deferred by the foundation campaign.

use std::io;
use std::path::Path;
use std::rc::Rc;

use markit_composition::{Capability, ComponentSpec};

/// The stable `filesystem` role's capability contract: byte-level
/// read/write of local files. Consumers reach it through this face, never
/// through a platform brand (`std_filesystem`/`windows_filesystem` stay
/// private implementation detail).
pub trait LocalFiles {
    /// Read the complete bytes of one local file.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Persist the complete bytes of one local file atomically: the write
    /// lands in a sibling temp file which then replaces the target by
    /// rename, so a process crash mid-write cannot leave a torn target
    /// (power-loss durability is a later concern). An existing target's
    /// permissions are preserved.
    fn write(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;
}

/// The std-based local implementation. The concrete type is public for
/// composition and tests; consumers should bind to [`LocalFiles`].
pub struct StdFiles;

impl LocalFiles for StdFiles {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn write(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let file_name = path
            .file_name()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?;
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        let base = file_name.to_string_lossy().into_owned();
        let pid = std::process::id();
        // Create the temp exclusively (O_EXCL): acquisition is atomic, so
        // concurrent savers never share a temp, and a planted symlink at
        // the temp name is never followed. A target's permissions carry
        // over so replacing it never widens access.
        let mut n = 0u32;
        let tmp = loop {
            let candidate = dir.join(format!(".{base}.{pid}.tmp{n}"));
            match std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&candidate)
            {
                Ok(_) => break candidate,
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists => n += 1,
                Err(e) => return Err(e),
            }
        };
        let cleanup = |e: io::Error| {
            let _ = std::fs::remove_file(&tmp);
            e
        };
        if let Ok(meta) = std::fs::metadata(path) {
            #[cfg(unix)]
            if let Err(e) = std::fs::set_permissions(&tmp, meta.permissions()) {
                return Err(cleanup(e));
            }
            #[cfg(not(unix))]
            let _ = meta;
        }
        if let Err(e) = std::fs::write(&tmp, bytes) {
            return Err(cleanup(e));
        }
        std::fs::rename(&tmp, path).map_err(cleanup)
    }
}

/// The stable `filesystem` capability: identity is the contract, not the
/// backend.
pub struct FilesystemCapability;

impl Capability for FilesystemCapability {
    const NAME: &'static str = "Filesystem";
    type Service = dyn LocalFiles;
}

/// The stable `filesystem` role (foundation-rules constructor convention).
pub fn filesystem_plugin() -> ComponentSpec {
    ComponentSpec::new("filesystem")
        .provides::<FilesystemCapability>()
        .on_activate(|ctx| {
            ctx.provide::<FilesystemCapability>(Rc::new(StdFiles))
                .expect("provides declared");
            Ok(())
        })
}

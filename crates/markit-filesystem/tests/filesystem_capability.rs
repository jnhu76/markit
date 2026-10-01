//! Filesystem-capability behavior tests: local read/write of Markdown
//! bytes under real paths, atomic save, OS error surfacing, and the
//! stable `filesystem` composition role. Path/OS rules per campaign D1/D2:
//! the filesystem owns the mechanism and OS errors; document truth and
//! dirty relations stay in the document crate (paths are never URLized
//! here).

use std::path::PathBuf;
use std::rc::Rc;

use markit_composition::{CompositionKernel, DesiredEntry, Revision as K0Revision};
use markit_filesystem::{FilesystemCapability, LocalFiles, StdFiles, filesystem_plugin};

fn tmp_dir(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join(format!("markit-fs-test-{}-{}", std::process::id(), name));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).expect("create temp dir");
    base
}

/// A committed revision's bytes survive a save/load round trip exactly —
/// including CJK and emoji content and paths with spaces/Unicode — and
/// the save leaves no temporary files behind.
#[test]
fn save_round_trips_revision_bytes_and_leaves_no_temp_files() {
    let dir = tmp_dir("roundtrip");
    let path = dir.join("笔记 最終 📝.md");

    let service = StdFiles;
    let bytes = "# 世界 🌍\n\n## 絵文字 🎨 body\n".as_bytes();
    service.write(&path, bytes).expect("atomic save");

    assert_eq!(service.read(&path).expect("read back"), bytes);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);

    let left: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(left, ["笔记 最終 📝.md"], "no temp files left behind");
}

/// Reading a missing file surfaces the OS error instead of fabricating
/// content.
#[test]
fn missing_file_surfaces_the_os_error() {
    let dir = tmp_dir("missing");
    let error = StdFiles.read(&dir.join("nope.md")).expect_err("missing");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

/// Saving over an existing file replaces its bytes exactly (no append, no
/// partial mix).
#[test]
fn save_replaces_existing_bytes() {
    let dir = tmp_dir("replace");
    let path = dir.join("doc.md");
    StdFiles.write(&path, b"# Old\n").expect("first save");
    StdFiles
        .write(&path, "# New 内容\n".as_bytes())
        .expect("second save");
    assert_eq!(StdFiles.read(&path).unwrap(), "# New 内容\n".as_bytes());
}

/// The stable `filesystem` role binds through K0; consumers drive the
/// already-bound capability face without re-entering composition.
#[test]
fn filesystem_plugin_binds_through_k0() {
    let mut kernel = CompositionKernel::new();
    kernel
        .register_component(filesystem_plugin())
        .expect("legal");
    kernel
        .set_desired(vec![DesiredEntry::enabled(
            "filesystem",
            "filesystem",
            K0Revision::fresh(),
        )])
        .expect("legal");
    kernel.settle();

    let dir = tmp_dir("k0");
    let path = dir.join("hello 世界.md");
    let observed: Rc<std::cell::RefCell<Option<Vec<u8>>>> = Default::default();

    // a consumer resolves the capability once during activation, then uses
    // the bound service directly
    let probe_observed = observed.clone();
    let probe_path = path.clone();
    kernel
        .register_component(
            markit_composition::ComponentSpec::new("fs_probe")
                .requires::<FilesystemCapability>()
                .on_activate(move |ctx| {
                    let files = ctx
                        .resolve::<FilesystemCapability>()
                        .map_err(|e| markit_composition::ActivationError::new(format!("{e:?}")))?;
                    let files = files.service();
                    files
                        .write(&probe_path, b"# Bound\n")
                        .map_err(|e| markit_composition::ActivationError::new(format!("{e}")))?;
                    *probe_observed.borrow_mut() = Some(files.read(&probe_path).unwrap());
                    Ok(())
                }),
        )
        .expect("legal");
    kernel
        .set_desired(vec![
            DesiredEntry::enabled("filesystem", "filesystem", K0Revision::fresh()),
            DesiredEntry::enabled("probe", "fs_probe", K0Revision::fresh()),
        ])
        .expect("legal");
    kernel.settle();

    assert_eq!(
        observed.borrow().as_deref(),
        Some(b"# Bound\n".as_ref()),
        "the bound face performed real IO"
    );
}

/// Paths are filesystem paths, never URL strings: a path that only makes
/// sense un-URLized round-trips byte-identically.
#[test]
fn paths_are_not_urlized() {
    let dir = tmp_dir("urlish");
    let path = dir.join("weird #hash %20 name?.md");
    StdFiles.write(&path, b"x").expect("write");
    assert!(path.is_file());
    assert_eq!(StdFiles.read(&path).unwrap(), b"x");
}

/// A missing parent directory surfaces the OS error (no temp junk, no
/// silent creation of directories).
#[test]
fn missing_parent_dir_surfaces_the_os_error() {
    let dir = tmp_dir("no-parent");
    let error = StdFiles
        .write(&dir.join("gone").join("doc.md"), b"x")
        .expect_err("missing parent");
    assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
}

/// Replacing a file preserves its permissions: saving a private
/// (owner-only) file never widens access.
#[cfg(unix)]
#[test]
fn save_preserves_target_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tmp_dir("permissions");
    let path = dir.join("private.md");
    StdFiles
        .write(
            &path, b"# v1
",
        )
        .expect("initial save");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).expect("restrict");

    StdFiles
        .write(
            &path, b"# v2
",
        )
        .expect("save over private file");

    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600, "save must not widen access");
    assert_eq!(StdFiles.read(&path).unwrap(), b"# v2\n");
}

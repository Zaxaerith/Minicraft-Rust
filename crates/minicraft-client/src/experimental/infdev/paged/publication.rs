//! Failure-only Windows publication evidence. Never retry, replace, or change
//! sharing permissions; the exclusive operation remains the migration helper.
use std::{fs, path::Path};

#[cfg(windows)]
pub(super) fn handle_count() -> Option<u32> {
    #[link(name = "Kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn GetProcessHandleCount(process: *mut std::ffi::c_void, count: *mut u32) -> i32;
    }
    let mut count = 0;
    // SAFETY: current-process pseudo handle is valid; count is a live u32.
    (unsafe { GetProcessHandleCount(GetCurrentProcess(), &mut count) } != 0).then_some(count)
}
#[cfg(not(windows))]
pub(super) fn handle_count() -> Option<u32> {
    None
}

fn status(p: &Path) -> String {
    match fs::symlink_metadata(p) {
        Ok(m) => {
            #[cfg(windows)]
            let attributes = {
                use std::os::windows::fs::MetadataExt;
                m.file_attributes()
            };
            #[cfg(not(windows))]
            let attributes = 0;
            format!(
                "exists dir={} len={} readonly={} attributes=0x{attributes:x}",
                m.is_dir(),
                m.len(),
                m.permissions().readonly()
            )
        }
        Err(e) => format!("metadata_error={e}; win32={:?}", e.raw_os_error()),
    }
}
pub(super) fn publish(
    from: &Path,
    to: &Path,
    before_close: Option<u32>,
    after_close: Option<u32>,
) -> Result<(), String> {
    crate::saveload::migration::publish_exclusive(from, to).map_err(|e| {
        // Capture the original error before any further OS calls can change it.
        let code = e.raw_os_error();
        let children = fs::read_dir(from).map(|entries| {
            entries.take(16).map(|entry| match entry {
                Ok(entry) => format!("{} [{}]", entry.path().display(), status(&entry.path())),
                Err(e) => format!("read_dir entry error: {e}"),
            }).collect::<Vec<_>>()
        });
        format!("cannot exclusively publish experimental world: operation=MoveFileW win32={code:?}; {e}; from={} [{}]; to={} [{}]; pid={}; process_handles_before_world_drop={before_close:?} after_world_drop={after_close:?} now={:?}; writer lock uses share_mode(0); counts are process-wide, not ownership attribution; children(sample<=16)={children:?}; external handle owners UNKNOWN; no retry or replacement performed", from.display(), status(from), to.display(), status(to), std::process::id(), handle_count())
    })
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    use crate::experimental::infdev::paged::tests::Owned;
    use std::os::windows::fs::OpenOptionsExt;

    #[test]
    fn held_source_directory_without_delete_sharing_refuses_publish_then_release_succeeds() {
        let owned = Owned::new();
        let source = owned.0.join("stage");
        let target = owned.0.join("installed");
        fs::create_dir(&source).unwrap();
        let child = source.join("checkpoint.json");
        fs::write(&child, b"unchanged").unwrap();
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(3)
            .custom_flags(0x0200_0000)
            .open(&source)
            .unwrap();
        let error = publish(&source, &target, handle_count(), handle_count()).unwrap_err();
        println!("CONTROLLED Windows source directory handle reproduction: {error}");
        assert!(error.contains("operation=MoveFileW") && error.contains("win32=Some("));
        assert!(source.exists() && !target.exists());
        assert!(error.contains("checkpoint.json") && error.contains(&source.display().to_string()));
        drop(held);
        assert_eq!(fs::read(&child).unwrap(), b"unchanged");
        publish(&source, &target, handle_count(), handle_count()).unwrap();
        assert_eq!(
            fs::read(target.join("checkpoint.json")).unwrap(),
            b"unchanged"
        );
        // An existing EMPTY destination is protected too.
        fs::create_dir(&source).unwrap();
        assert!(publish(&target, &source, None, None).is_err());
        assert_eq!(
            fs::read(target.join("checkpoint.json")).unwrap(),
            b"unchanged"
        );
    }

    #[test]
    fn experimental_writer_lock_must_be_closed_before_directory_publication() {
        let owned = Owned::new();
        let source = owned.0.join("stage");
        let target = owned.0.join("installed");
        fs::create_dir(&source).unwrap();
        let lock = crate::experimental::infdev::store::lock(&source).unwrap();
        let error = publish(&source, &target, handle_count(), handle_count()).unwrap_err();
        println!("CONTROLLED own session.lock handle: {error}");
        assert!(!target.exists());
        drop(lock);
        publish(&source, &target, handle_count(), handle_count()).unwrap();
        drop(crate::experimental::infdev::store::lock(&target).unwrap());
    }

    #[test]
    fn concurrent_creations_close_own_locks_and_never_reuse_stage_paths() {
        let owned = Owned::new();
        std::thread::scope(|scope| {
            let jobs: Vec<_> = (0..4)
                .map(|worker| {
                    let root = &owned.0;
                    scope.spawn(move || {
                        for iteration in 0..2 {
                            let name = format!("Create-{worker}-{iteration}");
                            let w = crate::experimental::infdev::paged::World::create_survival(
                                root, &name, 42, 1,
                            )
                            .unwrap();
                            let directory = w.directory().to_path_buf();
                            drop(w);
                            drop(
                                crate::experimental::infdev::paged::World::open(&directory, 1)
                                    .unwrap(),
                            );
                            assert!(
                                crate::experimental::infdev::paged::World::create_survival(
                                    root, &name, 42, 1
                                )
                                .is_err()
                            );
                        }
                    })
                })
                .collect();
            for job in jobs {
                job.join().unwrap();
            }
        });
    }
}

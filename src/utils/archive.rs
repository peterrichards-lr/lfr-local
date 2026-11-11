use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sevenz_rust::{Password, SevenZReader};
use std::fs::File;
use std::path::{Component, Path, PathBuf};
use tar::{Archive, Builder};

/// Creates a .tar.gz archive from a source directory
#[allow(dead_code)]
pub fn create_tar_gz(src_dir: &Path, dest_file: &Path) -> Result<(), String> {
    let tar_gz =
        File::create(dest_file).map_err(|e| format!("Failed to create archive file: {}", e))?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = Builder::new(enc);

    tar.append_dir_all(".", src_dir)
        .map_err(|e| format!("Failed to bundle directory: {}", e))?;

    tar.finish()
        .map_err(|e| format!("Failed to finish tar: {}", e))
}

/// Unpacks a Liferay bundle (zip, tar.gz or 7z) and strips the first component
pub fn extract_bundle(
    archive_path: &Path,
    dest_dir: &Path,
    strip_first: bool,
) -> Result<(), String> {
    let filename = archive_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or_default();

    if filename.ends_with(".zip") {
        extract_zip(archive_path, dest_dir, strip_first)
    } else if filename.ends_with(".tar.gz") {
        extract_tar_gz_stripped(archive_path, dest_dir, strip_first)
    } else if filename.ends_with(".7z") {
        extract_7z(archive_path, dest_dir, strip_first)
    } else {
        Err(format!("Unsupported archive format: {}", filename))
    }
}

/// Extracts a .7z archive with optional component stripping
pub fn extract_7z(archive_path: &Path, dest_dir: &Path, strip_first: bool) -> Result<(), String> {
    let file = std::fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let len = file.metadata().map_err(|e| e.to_string())?.len();
    let mut archive = SevenZReader::new(file, len, Password::empty()).map_err(|e| e.to_string())?;

    archive
        .for_each_entries(|entry, reader| {
            let path = PathBuf::from(entry.name());
            let outpath = match safe_relative_path(&path, strip_first) {
                Some(p) => p,
                None => {
                    warn_unsafe_entry(&path);
                    return Ok(true);
                }
            };

            if outpath.as_os_str().is_empty() {
                return Ok(true);
            }

            let full_outpath = dest_dir.join(outpath);
            if entry.is_directory() {
                std::fs::create_dir_all(&full_outpath)
                    .map_err(|e| sevenz_rust::Error::other(e.to_string()))?;
            } else {
                if !ensure_parent_inside(dest_dir, &full_outpath)
                    .map_err(sevenz_rust::Error::other)?
                {
                    warn_unsafe_entry(&path);
                    return Ok(true);
                }
                let mut outfile = std::fs::File::create(&full_outpath)
                    .map_err(|e| sevenz_rust::Error::other(e.to_string()))?;
                std::io::copy(reader, &mut outfile)
                    .map_err(|e| sevenz_rust::Error::other(e.to_string()))?;
            }
            Ok(true)
        })
        .map_err(|e| format!("7z Error: {}", e))
}

/// Converts an archive entry name into a path relative to the extraction
/// directory, optionally stripping its first component.
///
/// Returns `None` if the name is absolute or contains a `..` component, i.e. if
/// it could resolve outside the extraction directory. The returned path may be
/// empty (e.g. for the stripped top-level directory itself).
fn safe_relative_path(path: &Path, strip_first: bool) -> Option<PathBuf> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    let skip = usize::from(strip_first);
    Some(parts.into_iter().skip(skip).collect())
}

/// Creates the parent directories of `full_outpath` and checks that, once any
/// existing symlinks are resolved, they still lie inside `dest_dir`.
fn ensure_parent_inside(dest_dir: &Path, full_outpath: &Path) -> Result<bool, String> {
    let parent = match full_outpath.parent() {
        Some(p) => p,
        None => return Ok(false),
    };
    std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
    let canon_parent = parent
        .canonicalize()
        .map_err(|e| format!("Failed to resolve {}: {}", parent.display(), e))?;
    let canon_dest = dest_dir
        .canonicalize()
        .map_err(|e| format!("Failed to resolve {}: {}", dest_dir.display(), e))?;
    Ok(canon_parent.starts_with(canon_dest))
}

/// Returns true if a symlink at `link_rel` (relative to the extraction
/// directory) pointing at `target` stays inside the extraction directory.
fn symlink_target_is_inside(link_rel: &Path, target: &Path) -> bool {
    let mut depth = link_rel.components().count().saturating_sub(1);
    for component in target.components() {
        match component {
            Component::Normal(_) => depth += 1,
            Component::CurDir => {}
            Component::ParentDir => match depth.checked_sub(1) {
                Some(d) => depth = d,
                None => return false,
            },
            Component::RootDir | Component::Prefix(_) => return false,
        }
    }
    true
}

fn warn_unsafe_entry(name: &Path) {
    eprintln!(
        "Warning: skipping archive entry with unsafe path: {}",
        name.display()
    );
}

/// Extracts a .tar.gz archive with optional component stripping
pub fn extract_tar_gz_stripped(
    archive_path: &Path,
    dest_dir: &Path,
    strip_first: bool,
) -> Result<(), String> {
    let tar_gz = File::open(archive_path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    for entry in archive
        .entries()
        .map_err(|e| format!("Failed to read entries: {}", e))?
    {
        let mut entry = entry.map_err(|e| format!("Failed to read entry: {}", e))?;
        let path = entry
            .path()
            .map_err(|e| format!("Failed to get entry path: {}", e))?
            .into_owned();

        let outpath = match safe_relative_path(&path, strip_first) {
            Some(p) => p,
            None => {
                warn_unsafe_entry(&path);
                continue;
            }
        };

        if outpath.as_os_str().is_empty() {
            continue;
        }

        let full_outpath = dest_dir.join(&outpath);
        if !ensure_parent_inside(dest_dir, &full_outpath)? {
            warn_unsafe_entry(&path);
            continue;
        }

        let kind = entry.header().entry_type();
        if kind.is_hard_link() || kind.is_symlink() {
            let target = entry
                .link_name()
                .map_err(|e| format!("Failed to get link target: {}", e))?
                .map(|t| t.into_owned())
                .unwrap_or_default();

            if kind.is_hard_link() {
                // Hard link targets are archive paths: strip and resolve them
                // against dest_dir ourselves rather than relative to the CWD.
                let link_src = match safe_relative_path(&target, strip_first) {
                    Some(t) if !t.as_os_str().is_empty() => dest_dir.join(t),
                    _ => {
                        warn_unsafe_entry(&path);
                        continue;
                    }
                };
                if !ensure_parent_inside(dest_dir, &link_src)? {
                    warn_unsafe_entry(&path);
                    continue;
                }
                std::fs::hard_link(&link_src, &full_outpath)
                    .map_err(|e| format!("Failed to create hard link: {}", e))?;
                continue;
            }

            if !symlink_target_is_inside(&outpath, &target) {
                warn_unsafe_entry(&path);
                continue;
            }
        }

        entry
            .unpack(&full_outpath)
            .map_err(|e| format!("Failed to unpack entry: {}", e))?;
    }
    Ok(())
}

/// Extracts a .tar.gz archive to a destination directory
#[allow(dead_code)]
pub fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    extract_tar_gz_stripped(archive_path, dest_dir, false)
}

/// Extracts a .zip archive to a destination directory, optionally stripping the first component
pub fn extract_zip(archive_path: &Path, dest_dir: &Path, strip_first: bool) -> Result<(), String> {
    let file = File::open(archive_path).map_err(|e| format!("Failed to open zip: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Zip Error: {}", e))?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .map_err(|e| format!("Zip Entry Error: {}", e))?;
        let outpath = match file.enclosed_name() {
            Some(path) => {
                if strip_first {
                    let mut components = path.components();
                    components.next(); // Skip the first component
                    components.as_path().to_owned()
                } else {
                    path.to_owned()
                }
            }
            None => continue,
        };

        if outpath.as_os_str().is_empty() {
            continue;
        }

        let full_outpath = dest_dir.join(outpath);

        if file.is_dir() {
            std::fs::create_dir_all(&full_outpath)
                .map_err(|e| format!("Failed to create dir: {}", e))?;
        } else {
            if let Some(p) = full_outpath.parent() {
                if !p.exists() {
                    std::fs::create_dir_all(p)
                        .map_err(|e| format!("Failed to create dir: {}", e))?;
                }
            }
            let mut outfile =
                File::create(&full_outpath).map_err(|e| format!("Failed to create file: {}", e))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Failed to copy file: {}", e))?;
        }

        // Set permissions if on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Some(mode) = file.unix_mode() {
                std::fs::set_permissions(&full_outpath, std::fs::Permissions::from_mode(mode)).ok();
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use zip::write::SimpleFileOptions;

    fn write_zip(path: &Path, entries: &[(&str, Option<&str>)]) {
        let mut zip = zip::ZipWriter::new(File::create(path).unwrap());
        for (name, content) in entries {
            match content {
                None => zip
                    .add_directory(*name, SimpleFileOptions::default())
                    .unwrap(),
                Some(body) => {
                    zip.start_file(*name, SimpleFileOptions::default()).unwrap();
                    zip.write_all(body.as_bytes()).unwrap();
                }
            }
        }
        zip.finish().unwrap();
    }

    #[test]
    fn extract_zip_strips_top_level_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("bundle.zip");
        write_zip(
            &archive,
            &[
                ("liferay/", None),
                ("liferay/a.txt", Some("a")),
                ("liferay/tomcat/bin/", None),
                ("liferay/tomcat/bin/run.sh", Some("run")),
                ("liferay/../evil.txt", Some("x")),
            ],
        );
        let out = tmp.path().join("out");
        fs::create_dir(&out).unwrap();

        extract_bundle(&archive, &out, true).unwrap();

        assert_eq!(fs::read_to_string(out.join("a.txt")).unwrap(), "a");
        assert_eq!(
            fs::read_to_string(out.join("tomcat/bin/run.sh")).unwrap(),
            "run"
        );
        assert!(!out.join("liferay").exists());
        assert!(!tmp.path().join("evil.txt").exists());
    }

    #[test]
    fn extract_zip_without_stripping() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("bundle.zip");
        write_zip(
            &archive,
            &[
                ("a.txt", Some("a")),
                ("dir/nested/b.txt", Some("b")),
                ("../evil.txt", Some("x")),
            ],
        );
        let out = tmp.path().join("out");
        fs::create_dir(&out).unwrap();

        extract_bundle(&archive, &out, false).unwrap();

        assert_eq!(fs::read_to_string(out.join("a.txt")).unwrap(), "a");
        assert_eq!(
            fs::read_to_string(out.join("dir/nested/b.txt")).unwrap(),
            "b"
        );
        assert!(!tmp.path().join("evil.txt").exists());
    }

    #[test]
    fn extract_tar_gz_strips_top_level_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src/liferay/tomcat");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("c.txt"), "c").unwrap();
        let archive = tmp.path().join("bundle.tar.gz");
        create_tar_gz(&tmp.path().join("src"), &archive).unwrap();
        let out = tmp.path().join("out");
        fs::create_dir(&out).unwrap();

        extract_bundle(&archive, &out, true).unwrap();

        assert_eq!(fs::read_to_string(out.join("tomcat/c.txt")).unwrap(), "c");
        assert!(!out.join("liferay").exists());
    }

    /// A tar entry to write with a raw (unvalidated) name, bypassing the
    /// `tar` builder's own `..`/absolute-path checks.
    enum RawEntry<'a> {
        File(&'a str, &'a str),
        Symlink(&'a str, &'a str),
        HardLink(&'a str, &'a str),
    }

    fn write_raw_tar_gz(path: &Path, entries: &[RawEntry]) {
        let enc = GzEncoder::new(File::create(path).unwrap(), Compression::default());
        let mut builder = Builder::new(enc);
        for entry in entries {
            let mut header = tar::Header::new_gnu();
            let (name, link, kind, body) = match entry {
                RawEntry::File(n, b) => (*n, None, tar::EntryType::Regular, *b),
                RawEntry::Symlink(n, l) => (*n, Some(*l), tar::EntryType::Symlink, ""),
                RawEntry::HardLink(n, l) => (*n, Some(*l), tar::EntryType::Link, ""),
            };
            {
                let gnu = header.as_gnu_mut().unwrap();
                gnu.name[..name.len()].copy_from_slice(name.as_bytes());
                if let Some(l) = link {
                    gnu.linkname[..l.len()].copy_from_slice(l.as_bytes());
                }
            }
            header.set_entry_type(kind);
            header.set_mode(0o644);
            header.set_size(body.len() as u64);
            header.set_cksum();
            builder.append(&header, body.as_bytes()).unwrap();
        }
        builder.into_inner().unwrap().finish().unwrap();
    }

    fn write_7z(path: &Path, entries: &[(&str, &str)]) {
        let mut sz = sevenz_rust::SevenZWriter::create(path).unwrap();
        for (name, body) in entries {
            let mut entry = sevenz_rust::SevenZArchiveEntry::new();
            entry.name = name.to_string();
            entry.has_stream = true;
            entry.size = body.len() as u64;
            sz.push_archive_entry(entry, Some(body.as_bytes())).unwrap();
        }
        sz.finish().unwrap();
    }

    /// Returns (tmp root, dest dir). The destination is nested two
    /// levels deep so that `..` escapes still land inside the temp root.
    fn nested_dest() -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::tempdir().unwrap();
        let out = tmp.path().join("a/b/out");
        fs::create_dir_all(&out).unwrap();
        (tmp, out)
    }

    #[test]
    fn extract_tar_gz_skips_traversal_entries_when_stripping() {
        let (tmp, out) = nested_dest();
        let archive = tmp.path().join("bundle.tar.gz");
        write_raw_tar_gz(
            &archive,
            &[
                RawEntry::File("top/ok.txt", "ok"),
                RawEntry::File("top/../evil1.txt", "x"),
                RawEntry::File("top/../../evil2.txt", "x"),
                RawEntry::File("top/sub/../../../evil3.txt", "x"),
            ],
        );

        extract_bundle(&archive, &out, true).unwrap();

        assert_eq!(fs::read_to_string(out.join("ok.txt")).unwrap(), "ok");
        let b = tmp.path().join("a/b");
        assert!(!b.join("evil1.txt").exists());
        assert!(!tmp.path().join("a/evil2.txt").exists());
        assert!(!b.join("evil3.txt").exists());
    }

    #[test]
    fn extract_tar_gz_skips_traversal_entries_without_stripping() {
        let (tmp, out) = nested_dest();
        let abs = tmp.path().join("abs_evil.txt");
        let abs_name = abs.to_str().unwrap().to_string();
        let archive = tmp.path().join("bundle.tar.gz");
        write_raw_tar_gz(
            &archive,
            &[
                RawEntry::File("ok.txt", "ok"),
                RawEntry::File("../evil1.txt", "x"),
                RawEntry::File("dir/../../../evil2.txt", "x"),
                RawEntry::File(&abs_name, "x"),
            ],
        );

        extract_bundle(&archive, &out, false).unwrap();

        assert_eq!(fs::read_to_string(out.join("ok.txt")).unwrap(), "ok");
        assert!(!tmp.path().join("a/b/evil1.txt").exists());
        assert!(!tmp.path().join("a/evil2.txt").exists());
        assert!(!abs.exists());
    }

    #[test]
    fn extract_tar_gz_skips_absolute_entries_when_stripping() {
        let (tmp, out) = nested_dest();
        let abs = tmp.path().join("abs_evil.txt");
        let abs_name = abs.to_str().unwrap().to_string();
        let archive = tmp.path().join("bundle.tar.gz");
        write_raw_tar_gz(
            &archive,
            &[
                RawEntry::File("top/ok.txt", "ok"),
                RawEntry::File(&abs_name, "x"),
            ],
        );

        extract_bundle(&archive, &out, true).unwrap();

        assert_eq!(fs::read_to_string(out.join("ok.txt")).unwrap(), "ok");
        assert!(!abs.exists());
        // The absolute entry must be skipped, not re-rooted under dest.
        assert_eq!(fs::read_dir(&out).unwrap().count(), 1);
    }

    #[cfg(unix)]
    #[test]
    fn extract_tar_gz_does_not_write_through_escaping_symlink() {
        let (tmp, out) = nested_dest();
        let outside = tmp.path().join("outside");
        fs::create_dir(&outside).unwrap();
        let archive = tmp.path().join("bundle.tar.gz");
        let outside_name = outside.to_str().unwrap().to_string();
        write_raw_tar_gz(
            &archive,
            &[
                RawEntry::Symlink("top/abs", &outside_name),
                RawEntry::File("top/abs/evil1.txt", "x"),
                RawEntry::Symlink("top/rel", "../../../outside"),
                RawEntry::File("top/rel/evil2.txt", "x"),
            ],
        );

        let _ = extract_bundle(&archive, &out, true);

        assert!(!outside.join("evil1.txt").exists());
        assert!(!outside.join("evil2.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn extract_tar_gz_rejects_escaping_hardlink_and_keeps_internal_one() {
        let (tmp, out) = nested_dest();
        let secret = tmp.path().join("secret.txt");
        fs::write(&secret, "secret").unwrap();
        let secret_name = secret.to_str().unwrap().to_string();
        let archive = tmp.path().join("bundle.tar.gz");
        write_raw_tar_gz(
            &archive,
            &[
                RawEntry::File("top/a.txt", "a"),
                RawEntry::HardLink("top/b.txt", "top/a.txt"),
                RawEntry::HardLink("top/abs", &secret_name),
                RawEntry::HardLink("top/rel", "top/../../../../secret.txt"),
            ],
        );

        let _ = extract_bundle(&archive, &out, true);

        assert!(!out.join("abs").exists());
        assert!(!out.join("rel").exists());
        assert_eq!(fs::read_to_string(&secret).unwrap(), "secret");
        assert_eq!(fs::read_to_string(out.join("b.txt")).unwrap(), "a");
    }

    #[test]
    fn extract_7z_skips_traversal_entries_when_stripping() {
        let (tmp, out) = nested_dest();
        let archive = tmp.path().join("bundle.7z");
        write_7z(
            &archive,
            &[
                ("top/ok.txt", "ok"),
                ("top/../evil1.txt", "x"),
                ("top/../../evil2.txt", "x"),
            ],
        );

        extract_bundle(&archive, &out, true).unwrap();

        assert_eq!(fs::read_to_string(out.join("ok.txt")).unwrap(), "ok");
        assert!(!tmp.path().join("a/b/evil1.txt").exists());
        assert!(!tmp.path().join("a/evil2.txt").exists());
    }

    #[test]
    fn extract_7z_skips_traversal_entries_without_stripping() {
        let (tmp, out) = nested_dest();
        let abs = tmp.path().join("abs_evil.txt");
        let archive = tmp.path().join("bundle.7z");
        write_7z(
            &archive,
            &[
                ("ok.txt", "ok"),
                ("../evil1.txt", "x"),
                (abs.to_str().unwrap(), "x"),
            ],
        );

        extract_bundle(&archive, &out, false).unwrap();

        assert_eq!(fs::read_to_string(out.join("ok.txt")).unwrap(), "ok");
        assert!(!tmp.path().join("a/b/evil1.txt").exists());
        assert!(!abs.exists());
    }
}

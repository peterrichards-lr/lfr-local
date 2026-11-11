use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use sevenz_rust::{Password, SevenZReader};
use std::fs::File;
use std::path::{Path, PathBuf};
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
    if !strip_first {
        return sevenz_rust::decompress_file(archive_path, dest_dir)
            .map_err(|e| format!("7z Error: {}", e));
    }

    // Manual extraction to handle stripping
    let file = std::fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let mut archive = SevenZReader::new(
        file,
        archive_path.metadata().unwrap().len(),
        Password::empty(),
    )
    .map_err(|e| e.to_string())?;

    archive
        .for_each_entries(|entry, reader| {
            let path = PathBuf::from(entry.name());
            let mut components = path.components();
            components.next(); // Skip the first component
            let outpath = components.as_path();

            if outpath.as_os_str().is_empty() {
                return Ok(true);
            }

            let full_outpath = dest_dir.join(outpath);
            if entry.is_directory() {
                std::fs::create_dir_all(&full_outpath)
                    .map_err(|e| sevenz_rust::Error::other(e.to_string()))?;
            } else {
                if let Some(parent) = full_outpath.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|e| sevenz_rust::Error::other(e.to_string()))?;
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
            .map_err(|e| format!("Failed to get entry path: {}", e))?;

        let outpath: PathBuf = if strip_first {
            let mut components = path.components();
            components.next(); // Skip the first component
            components.as_path().to_owned()
        } else {
            path.to_path_buf()
        };

        if outpath.as_os_str().is_empty() {
            continue;
        }

        entry
            .unpack(dest_dir.join(outpath))
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
}

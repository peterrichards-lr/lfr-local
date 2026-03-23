#![allow(dead_code)]
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use std::fs::File;
use std::path::Path;
use tar::{Archive, Builder};

/// Creates a .tar.gz archive from a source directory
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

/// Extracts a .tar.gz archive to a destination directory
pub fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let tar_gz = File::open(archive_path).map_err(|e| format!("Failed to open archive: {}", e))?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    archive
        .unpack(dest_dir)
        .map_err(|e| format!("Failed to unpack archive: {}", e))
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

        if file.name().ends_with('/') {
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

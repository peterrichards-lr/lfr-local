use std::fs::File;
use std::path::Path;
use flate2::Compression;
use flate2::write::GzEncoder;
use flate2::read::GzDecoder;
use tar::{Archive, Builder};

/// Creates a .tar.gz archive from a source directory
pub fn create_tar_gz(src_dir: &Path, dest_file: &Path) -> Result<(), String> {
    let tar_gz = File::create(dest_file)
        .map_err(|e| format!("Failed to create archive file: {}", e))?;
    let enc = GzEncoder::new(tar_gz, Compression::default());
    let mut tar = Builder::new(enc);

    tar.append_dir_all(".", src_dir)
        .map_err(|e| format!("Failed to bundle directory: {}", e))?;
    
    tar.finish().map_err(|e| format!("Failed to finish tar: {}", e))
}

/// Extracts a .tar.gz archive to a destination directory
pub fn extract_tar_gz(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let tar_gz = File::open(archive_path)
        .map_err(|e| format!("Failed to open archive: {}", e))?;
    let tar = GzDecoder::new(tar_gz);
    let mut archive = Archive::new(tar);

    archive.unpack(dest_dir)
        .map_err(|e| format!("Failed to unpack archive: {}", e))
}
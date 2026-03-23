use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::blocking::Client;
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

pub fn download_file(url: &str, dest_path: &Path) -> Result<()> {
    let client = Client::new();
    let mut response = client.get(url).send().context("Failed to send request")?;

    if !response.status().is_success() {
        anyhow::bail!("Request failed with status: {}", response.status());
    }

    let total_size = response.content_length();

    let pb = match total_size {
        Some(size) => {
            let pb = ProgressBar::new(size);
            pb.set_style(ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")?
                .progress_chars("#>-"));
            pb
        }
        None => {
            let pb = ProgressBar::new_spinner();
            pb.set_style(ProgressStyle::default_spinner().template(
                "{spinner:.green} [{elapsed_precise}] Downloading... {bytes} downloaded",
            )?);
            pb
        }
    };

    let mut file = File::create(dest_path).context("Failed to create file")?;
    let mut buffer = [0; 8192];
    let mut downloaded: u64 = 0;

    loop {
        let n = response
            .read(&mut buffer)
            .context("Failed to read response")?;
        if n == 0 {
            break;
        }
        file.write_all(&buffer[..n])
            .context("Failed to write to file")?;
        downloaded += n as u64;
        pb.set_position(downloaded);
    }

    pb.finish_with_message("Download complete");
    Ok(())
}

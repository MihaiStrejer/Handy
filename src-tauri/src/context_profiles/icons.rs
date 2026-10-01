//! Imported icons are decoded and normalized before entering the profile store.
//! A saved profile owns its image bytes; the source file is never referenced again.

use base64::{engine::general_purpose::STANDARD, Engine};
use image::{DynamicImage, ImageFormat, ImageReader, Limits};
use std::io::{Cursor, Read};

const MAX_FILE_BYTES: usize = 2 * 1024 * 1024;
const MAX_SOURCE_SIDE: u32 = 2048;
const ICON_SIDE: u32 = 128;
const MAX_DATA_BYTES: usize = 100_000;
const PNG_PREFIX: &str = "data:image/png;base64,";

fn decode(bytes: &[u8], format: ImageFormat, side: u32) -> Result<DynamicImage, String> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(side);
    limits.max_image_height = Some(side);
    limits.max_alloc = Some(32 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|_| "Invalid icon image or image dimensions exceed the limit".into())
}

fn normalize(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() > MAX_FILE_BYTES {
        return Err("Choose an icon smaller than 2 MB".into());
    }
    let format = image::guess_format(bytes).map_err(|_| "Choose a PNG or ICO image")?;
    if !matches!(format, ImageFormat::Png | ImageFormat::Ico) {
        return Err("Choose a PNG or ICO image".into());
    }
    let image = decode(bytes, format, MAX_SOURCE_SIDE)?;
    let image = image.thumbnail(ICON_SIDE, ICON_SIDE).to_rgba8();
    let mut png = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut png, ImageFormat::Png)
        .map_err(|_| "Could not prepare icon image")?;
    let data = format!("{PNG_PREFIX}{}", STANDARD.encode(png.into_inner()));
    validate_custom(&data)?;
    Ok(data)
}

pub(super) fn validate_custom(data: &str) -> Result<(), String> {
    if data.len() > MAX_DATA_BYTES {
        return Err("Custom icon exceeds its size limit".into());
    }
    let encoded = data
        .strip_prefix(PNG_PREFIX)
        .ok_or("Custom icon must be an imported PNG image")?;
    let bytes = STANDARD
        .decode(encoded)
        .map_err(|_| "Invalid custom icon encoding")?;
    decode(&bytes, ImageFormat::Png, ICON_SIDE)?;
    Ok(())
}

fn import_path(path: &str) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|_| "Could not open the selected icon")?;
    let metadata = file
        .metadata()
        .map_err(|_| "Could not read the selected icon")?;
    if !metadata.is_file() || metadata.len() > MAX_FILE_BYTES as u64 {
        return Err("Choose a PNG or ICO file smaller than 2 MB".into());
    }
    let mut bytes = Vec::new();
    file.take((MAX_FILE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read the selected icon")?;
    normalize(&bytes)
}

#[tauri::command]
#[specta::specta]
pub(crate) async fn import_profile_icon(path: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || import_path(&path))
        .await
        .map_err(|_| "Could not import the selected icon")?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(format: ImageFormat, side: u32) -> Vec<u8> {
        let mut bytes = Cursor::new(Vec::new());
        DynamicImage::new_rgba8(side, side)
            .write_to(&mut bytes, format)
            .unwrap();
        bytes.into_inner()
    }

    #[test]
    fn png_and_ico_become_bounded_pngs() {
        for format in [ImageFormat::Png, ImageFormat::Ico] {
            let data = normalize(&fixture(format, 256)).unwrap();
            validate_custom(&data).unwrap();
            let bytes = STANDARD
                .decode(data.strip_prefix(PNG_PREFIX).unwrap())
                .unwrap();
            assert_eq!(
                decode(&bytes, ImageFormat::Png, ICON_SIDE).unwrap().width(),
                128
            );
        }
    }

    #[test]
    fn imported_icon_survives_source_deletion_and_catalog_round_trip() {
        let file = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(file.path(), fixture(ImageFormat::Png, 32)).unwrap();
        let data = import_path(file.path().to_str().unwrap()).unwrap();
        drop(file);
        let icon = super::super::storage::ProfileIcon::Custom(data.clone());
        let saved = serde_json::to_string(&icon).unwrap();
        let restored: super::super::storage::ProfileIcon = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.display_value(), data);
        validate_custom(&restored.display_value()).unwrap();
    }

    #[test]
    fn rejects_non_images_remote_references_oversized_and_corrupt_input() {
        assert!(normalize(b"<svg onload='alert(1)' />").is_err());
        assert!(normalize(&vec![0; MAX_FILE_BYTES + 1]).is_err());
        assert!(normalize(&fixture(ImageFormat::Png, MAX_SOURCE_SIDE + 1)).is_err());
        assert!(validate_custom("https://example.com/icon.png").is_err());
        assert!(validate_custom("data:image/png;base64,AAAA").is_err());
        assert!(validate_custom(&format!(
            "{PNG_PREFIX}{}",
            STANDARD.encode(fixture(ImageFormat::Png, 256))
        ))
        .is_err());
    }
}

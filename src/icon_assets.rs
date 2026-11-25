use eframe::egui;
use std::{
    io::{Cursor, Read, Write},
    path::{Path, PathBuf},
};

const MAX_BYTES: u64 = 5 * 1024 * 1024;

pub fn set_root(ctx: &egui::Context, root: Option<PathBuf>) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("icon_library_root"), root));
}
pub fn root(ctx: &egui::Context) -> Option<PathBuf> {
    ctx.data(|data| {
        data.get_temp::<Option<PathBuf>>(egui::Id::new("icon_library_root"))
            .flatten()
    })
}
pub fn validate_id(id: &str) -> Result<(), String> {
    if !id.ends_with(".png")
        || id.len() > 160
        || id.len() <= 4
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
        || id.contains("..")
    {
        return Err("Invalid imported icon identifier.".into());
    }
    Ok(())
}

pub fn decode(path: &Path) -> Result<image::DynamicImage, String> {
    decode_image(path, false)
}
pub fn decode_photo(path: &Path) -> Result<image::DynamicImage, String> {
    decode_image(path, true)
}
fn decode_image(path: &Path, photos: bool) -> Result<image::DynamicImage, String> {
    let file = std::fs::File::open(path).map_err(|e| format!("Cannot open icon: {e}"))?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Choose a PNG image file.".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Choose an image smaller than 5 MB.".into());
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    if reader.format() != Some(image::ImageFormat::Png)
        && !(photos && reader.format() == Some(image::ImageFormat::Jpeg))
    {
        return Err(if photos {
            "Choose a PNG or JPEG photo."
        } else {
            "Choose a PNG image. Transparent PNG icons are supported."
        }
        .into());
    }
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(4096);
    limits.max_image_height = Some(4096);
    limits.max_alloc = Some(64 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map_err(|e| format!("Cannot decode icon: {e}"))
}

pub fn import(root: &Path, source: &Path) -> Result<String, String> {
    let image = decode(source)?.thumbnail(256, 256);
    let stem: String = source
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Icon")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .take(48)
        .collect();
    let mut random = [0u8; 12];
    rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, &mut random);
    let suffix = random
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let id = format!(
        "{}-{suffix}.png",
        if stem.is_empty() { "Icon" } else { &stem }
    );
    validate_id(&id)?;
    let mut encoded = Cursor::new(Vec::new());
    image
        .write_to(&mut encoded, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    std::fs::create_dir_all(root).map_err(|e| format!("Cannot create icon library: {e}"))?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(root.join(&id))
        .map_err(|e| e.to_string())?;
    file.write_all(&encoded.into_inner())
        .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    Ok(id)
}

pub fn list(root: &Path) -> Vec<String> {
    let mut icons: Vec<_> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|id| validate_id(id).is_ok())
        .collect();
    icons.sort();
    icons
}
pub fn label(id: &str) -> &str {
    id.strip_suffix(".png")
        .unwrap_or(id)
        .rsplit_once('-')
        .map(|(name, _)| name)
        .unwrap_or(id)
}

pub fn texture(ctx: &egui::Context, id: &str) -> Option<egui::TextureHandle> {
    validate_id(id).ok()?;
    let path = root(ctx)?.join(id);
    let cache_id = egui::Id::new(("imported_icon_texture", &path));
    if let Some(cached) = ctx.data(|data| data.get_temp::<Option<egui::TextureHandle>>(cache_id)) {
        return cached;
    }
    let texture = decode(&path).ok().map(|image| {
        let rgba = image.thumbnail(256, 256).to_rgba8();
        ctx.load_texture(
            path.to_string_lossy(),
            egui::ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize],
                rgba.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        )
    });
    ctx.data_mut(|data| data.insert_temp(cache_id, texture.clone()));
    texture
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_images_and_keeps_imports_in_account_library() {
        for invalid in [
            "../secret.png",
            "C:\\secret.png",
            "https://icon.png",
            "a.jpg",
            "..png",
        ] {
            assert!(validate_id(invalid).is_err());
        }
        let base = std::env::temp_dir().join(format!(
            "lab-icons-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        let source = base.join("Ubuntu.png");
        image::RgbaImage::from_pixel(32, 32, image::Rgba([30, 150, 60, 170]))
            .save(&source)
            .unwrap();
        let owner = base.join("owner");
        let other = base.join("other");
        let id = import(&owner, &source).unwrap();
        assert_eq!(list(&owner), vec![id.clone()]);
        assert!(list(&other).is_empty());
        assert_eq!(
            decode(&owner.join(&id))
                .unwrap()
                .to_rgba8()
                .get_pixel(0, 0)
                .0,
            [30, 150, 60, 170]
        );
        let bad = base.join("bad.png");
        std::fs::write(&bad, b"not an image").unwrap();
        assert!(import(&owner, &bad).is_err());
        let huge = base.join("huge.png");
        image::RgbaImage::new(4097, 1).save(&huge).unwrap();
        assert!(import(&owner, &huge).is_err());
        std::fs::remove_file(owner.join(id)).unwrap();
        std::fs::remove_dir(owner).unwrap();
        for path in [source, bad, huge] {
            std::fs::remove_file(path).unwrap();
        }
        std::fs::remove_dir(base).unwrap();
    }
}

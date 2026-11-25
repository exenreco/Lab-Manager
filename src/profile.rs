use serde::{Deserialize, Serialize};
use std::{io::Cursor, path::Path};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Profile {
    #[serde(default)]
    pub avatar: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub photo: Option<Vec<u8>>,
}

impl Profile {
    pub fn from_photo(path: &Path) -> Result<Self, String> {
        let image = crate::icon_assets::decode_photo(path)?.resize_to_fill(
            256,
            256,
            image::imageops::FilterType::Triangle,
        );
        let mut png = Cursor::new(Vec::new());
        image
            .write_to(&mut png, image::ImageFormat::Png)
            .map_err(|e| e.to_string())?;
        Ok(Self {
            avatar: 0,
            photo: Some(png.into_inner()),
        })
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.avatar >= 5 {
            return Err("Choose one of the five profile avatars.".into());
        }
        if let Some(bytes) = &self.photo {
            if bytes.len() > 1024 * 1024 {
                return Err("Profile photo is too large.".into());
            }
            let mut reader =
                image::ImageReader::with_format(Cursor::new(bytes), image::ImageFormat::Png);
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(256);
            limits.max_image_height = Some(256);
            limits.max_alloc = Some(1024 * 1024);
            reader.limits(limits);
            reader
                .decode()
                .map_err(|_| "Profile photo is not a valid thumbnail.".to_owned())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_avatar_and_photo_validation() {
        assert_eq!(Profile::default().avatar, 0);
        assert!(
            serde_json::from_str::<Profile>("{}")
                .unwrap()
                .validate()
                .is_ok()
        );
        assert!(
            Profile {
                avatar: 5,
                photo: None
            }
            .validate()
            .is_err()
        );
        assert!(
            Profile {
                avatar: 0,
                photo: Some(b"not a photo".to_vec())
            }
            .validate()
            .is_err()
        );
        let image = image::RgbImage::from_pixel(400, 200, image::Rgb([25, 90, 145]));
        let file = std::env::temp_dir().join(format!("lab-photo-test-{}.jpg", std::process::id()));
        image.save(&file).unwrap();
        let profile = Profile::from_photo(&file).unwrap();
        profile.validate().unwrap();
        let decoded = image::load_from_memory(profile.photo.as_ref().unwrap()).unwrap();
        assert_eq!((decoded.width(), decoded.height()), (256, 256));
        std::fs::remove_file(file).unwrap();
    }
}

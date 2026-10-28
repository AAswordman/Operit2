//! Bounded, decoder-free image previews for constrained Link clients.
use operit_link::CoreValue;
use std::collections::BTreeMap;

pub const PREVIEW_WIDTH: usize = 128;
pub const PREVIEW_HEIGHT: usize = 96;
pub const PREVIEW_CHUNK_BYTES: usize = 1024;
pub const PREVIEW_MAX_BYTES: usize = PREVIEW_WIDTH * PREVIEW_HEIGHT * 2;

#[derive(Debug)]
pub struct ImagePreviewChunk {
    pub width: usize,
    pub height: usize,
    pub offset: usize,
    pub bytes: Vec<u8>,
}

impl ImagePreviewChunk {
    pub fn into_value(self) -> CoreValue {
        CoreValue::Map(BTreeMap::from([
            ("format".into(), CoreValue::String("rgb565le".into())),
            ("width".into(), CoreValue::Unsigned(self.width as u64)),
            ("height".into(), CoreValue::Unsigned(self.height as u64)),
            ("offset".into(), CoreValue::Unsigned(self.offset as u64)),
            ("bytes".into(), CoreValue::Bytes(self.bytes.into())),
        ]))
    }

    /// Validate before allocating on Edge; never accept an arbitrary remote size.
    pub fn from_value(value: CoreValue) -> Result<Self, String> {
        let CoreValue::Map(mut fields) = value else { return Err("Invalid image chunk".into()); };
        if fields.remove("format") != Some(CoreValue::String("rgb565le".into())) {
            return Err("Unsupported preview format".into());
        }
        let number = |fields: &mut BTreeMap<String, CoreValue>, key: &str| -> Result<usize, String> {
            match fields.remove(key) {
                Some(CoreValue::Unsigned(n)) => usize::try_from(n).map_err(|_| "Image size overflow".into()),
                Some(CoreValue::Signed(n)) if n >= 0 => usize::try_from(n).map_err(|_| "Image size overflow".into()),
                _ => Err(format!("Invalid image {key}")),
            }
        };
        let width = number(&mut fields, "width")?;
        let height = number(&mut fields, "height")?;
        let offset = number(&mut fields, "offset")?;
        let Some(CoreValue::Bytes(bytes)) = fields.remove("bytes") else { return Err("Image bytes missing".into()); };
        if width == 0 || height == 0 || width > PREVIEW_WIDTH || height > PREVIEW_HEIGHT ||
            bytes.is_empty() || bytes.len() > PREVIEW_CHUNK_BYTES || offset % 2 != 0 || bytes.len() % 2 != 0 ||
            offset >= width * height * 2 || bytes.len() > width * height * 2 - offset {
            return Err("Image chunk exceeds preview limits".into());
        }
        Ok(Self {width, height, offset, bytes: bytes.to_vec()})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bounds_are_checked_before_allocation() {
        for (w, h, offset, len, valid) in [(128,96,0,1024,true),(0,1,0,2,false),
            (129,1,0,2,false),(1,97,0,2,false),(1,1,0,4,false),(1,1,1,1,false),
            (128,96,0,1026,false),(1,1,0,0,false)] {
            let value = ImagePreviewChunk {width:w,height:h,offset,bytes:vec![0;len]}.into_value();
            assert_eq!(ImagePreviewChunk::from_value(value).is_ok(), valid);
        }
    }
}

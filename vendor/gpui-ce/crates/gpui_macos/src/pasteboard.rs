use objc2::rc::Retained;
use objc2_app_kit::{
    NSPasteboard, NSPasteboardNameFind, NSPasteboardType, NSPasteboardTypeFileURL,
    NSPasteboardTypeString,
};
use objc2_foundation::{NSData, NSString};
use smallvec::SmallVec;
use strum::IntoEnumIterator as _;

use gpui::{
    ClipboardEntry, ClipboardItem, ClipboardString, ExternalPaths, Image, ImageFormat, hash,
};

pub struct Pasteboard {
    inner: Retained<NSPasteboard>,
    text_hash_type: Retained<NSPasteboardType>,
    metadata_type: Retained<NSPasteboardType>,
}

impl Pasteboard {
    pub fn general() -> Self {
        Self::new(NSPasteboard::generalPasteboard())
    }

    pub fn find() -> Self {
        Self::new(NSPasteboard::pasteboardWithName(unsafe {
            NSPasteboardNameFind
        }))
    }

    #[cfg(test)]
    pub fn unique() -> Option<Self> {
        let inner: Option<Retained<NSPasteboard>> =
            unsafe { objc2::msg_send![objc2::class!(NSPasteboard), pasteboardWithUniqueName] };

        inner.map(Self::new)
    }

    fn new(inner: Retained<NSPasteboard>) -> Self {
        Self {
            inner,
            text_hash_type: NSString::from_str("zed-text-hash"),
            metadata_type: NSString::from_str("zed-metadata"),
        }
    }

    pub fn read(&self) -> Option<ClipboardItem> {
        // Modern pasteboards represent each selected file as an item with a
        // file-URL payload. This avoids the deprecated filename property list
        // and validates the URL before it becomes a native path.
        if let Some(items) = self.inner.pasteboardItems() {
            let paths = items
                .iter()
                .filter_map(|item| item.stringForType(unsafe { NSPasteboardTypeFileURL }))
                .filter_map(|url| url::Url::parse(&url.to_string()).ok())
                .filter_map(|url| url.to_file_path().ok())
                .collect::<SmallVec<_>>();
            if !paths.is_empty() {
                let mut entries = vec![ClipboardEntry::ExternalPaths(ExternalPaths(paths))];

                // Also include the string representation so text editors can
                // paste the path as text.
                if let Some(string_item) = self.read_string_from_pasteboard() {
                    entries.push(string_item);
                }

                return Some(ClipboardItem { entries });
            }
        }

        // Next, check for a plain string.
        if let Some(string_entry) = self.read_string_from_pasteboard() {
            return Some(ClipboardItem {
                entries: vec![string_entry],
            });
        }

        // Finally, try the various supported image types.
        for format in ImageFormat::iter() {
            if let Some(item) = self.read_image(format) {
                return Some(item);
            }
        }

        None
    }

    fn read_image(&self, format: ImageFormat) -> Option<ClipboardItem> {
        let ut_type: UTType = format.into();

        if self.inner.types().is_some_and(|types| {
            types
                .iter()
                .any(|kind| kind.isEqualToString(ut_type.inner()))
        }) {
            self.data_for_type(ut_type.inner()).map(|bytes| {
                let id = hash(&bytes);

                ClipboardItem {
                    entries: vec![ClipboardEntry::Image(Image { format, bytes, id })],
                }
            })
        } else {
            None
        }
    }

    fn read_string_from_pasteboard(&self) -> Option<ClipboardEntry> {
        let string_type = NSString::from_str("public.utf8-plain-text");
        if !self
            .inner
            .types()
            .is_some_and(|types| types.iter().any(|kind| kind.isEqualToString(&string_type)))
        {
            return None;
        }

        let text_bytes = self.data_for_type(&string_type)?;
        let text = String::from_utf8_lossy(&text_bytes).to_string();
        let metadata = self
            .data_for_type(&self.text_hash_type)
            .and_then(|hash_bytes| {
                let hash_bytes = hash_bytes.as_slice().try_into().ok()?;
                let hash = u64::from_be_bytes(hash_bytes);
                let metadata = self.data_for_type(&self.metadata_type)?;

                if hash == ClipboardString::text_hash(&text) {
                    String::from_utf8(metadata).ok()
                } else {
                    None
                }
            });

        Some(ClipboardEntry::String(ClipboardString { text, metadata }))
    }

    fn data_for_type(&self, kind: &NSPasteboardType) -> Option<Vec<u8>> {
        self.inner.dataForType(kind).map(|data| data.to_vec())
    }

    pub fn write(&self, item: ClipboardItem) {
        match item.entries.as_slice() {
            [] => {
                // Writing an empty list of entries just clears the clipboard.
                self.inner.clearContents();
            }
            [ClipboardEntry::String(string)] => {
                self.write_plaintext(string);
            }
            [ClipboardEntry::Image(image)] => {
                self.write_image(image);
            }
            [ClipboardEntry::ExternalPaths(_)] => {}
            _ => {
                // Agus NB: We're currently only writing string entries to the clipboard when we have more than one.
                //
                // This was the existing behavior before I refactored the outer clipboard code:
                // https://github.com/zed-industries/zed/blob/65f7412a0265552b06ce122655369d6cc7381dd6/crates/gpui/src/platform/mac/platform.rs#L1060-L1110
                //
                // Note how `any_images` is always `false`. We should fix that, but that's orthogonal to the refactor.

                let mut combined = ClipboardString {
                    text: String::new(),
                    metadata: None,
                };

                for entry in item.entries {
                    match entry {
                        ClipboardEntry::String(text) => {
                            combined.text.push_str(&text.text());
                            if combined.metadata.is_none() {
                                combined.metadata = text.metadata;
                            }
                        }
                        _ => {}
                    }
                }

                self.write_plaintext(&combined);
            }
        }
    }

    fn write_plaintext(&self, string: &ClipboardString) {
        self.inner.clearContents();

        let text_bytes = NSData::with_bytes(string.text.as_bytes());
        self.inner
            .setData_forType(Some(&text_bytes), unsafe { NSPasteboardTypeString });

        if let Some(metadata) = string.metadata.as_ref() {
            let hash_bytes =
                NSData::with_bytes(&ClipboardString::text_hash(&string.text).to_be_bytes());
            self.inner
                .setData_forType(Some(&hash_bytes), &self.text_hash_type);

            let metadata_bytes = NSData::with_bytes(metadata.as_bytes());
            self.inner
                .setData_forType(Some(&metadata_bytes), &self.metadata_type);
        }
    }

    fn write_image(&self, image: &Image) {
        self.inner.clearContents();

        let bytes = NSData::with_bytes(&image.bytes);
        self.inner
            .setData_forType(Some(&bytes), Into::<UTType>::into(image.format).inner());
    }
}

impl From<ImageFormat> for UTType {
    fn from(value: ImageFormat) -> Self {
        match value {
            ImageFormat::Png => Self::png(),
            ImageFormat::Jpeg => Self::jpeg(),
            ImageFormat::Tiff => Self::tiff(),
            ImageFormat::Webp => Self::webp(),
            ImageFormat::Gif => Self::gif(),
            ImageFormat::Bmp => Self::bmp(),
            ImageFormat::Svg => Self::svg(),
            ImageFormat::Ico => Self::ico(),
            ImageFormat::Pnm => Self::pnm(),
        }
    }
}

// See https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/
pub struct UTType(Retained<NSPasteboardType>);

impl UTType {
    pub fn png() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/png
        Self(NSString::from_str("public.png"))
    }

    pub fn jpeg() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/jpeg
        Self(NSString::from_str("public.jpeg"))
    }

    pub fn gif() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/gif
        Self(NSString::from_str("com.compuserve.gif"))
    }

    pub fn webp() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/webp
        Self(NSString::from_str("org.webmproject.webp"))
    }

    pub fn bmp() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/bmp
        Self(NSString::from_str("com.microsoft.bmp"))
    }

    pub fn svg() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/svg
        Self(NSString::from_str("public.svg-image"))
    }

    pub fn ico() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/ico
        Self(NSString::from_str("com.microsoft.ico"))
    }

    pub fn tiff() -> Self {
        // https://developer.apple.com/documentation/uniformtypeidentifiers/uttype-swift.struct/tiff
        Self(NSString::from_str("public.tiff"))
    }

    pub fn pnm() -> Self {
        //https://en.wikipedia.org/w/index.php?title=Netpbm&oldid=1336679433 under Uniform Type Identifier
        Self(NSString::from_str("public.pbm"))
    }

    fn inner(&self) -> &NSPasteboardType {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::{Mutex, MutexGuard};

    use objc2::runtime::ProtocolObject;
    use objc2_app_kit::{
        NSPasteboardItem, NSPasteboardTypeFileURL, NSPasteboardTypePNG, NSPasteboardTypeString,
        NSPasteboardWriting,
    };
    use objc2_foundation::{NSArray, NSData, NSString};

    use gpui::{ClipboardEntry, ClipboardItem, ClipboardString, ImageFormat};

    use super::*;

    macro_rules! unique_pasteboard {
        () => {
            match unique_pasteboard() {
                Some(value) => value,
                None => return,
            }
        };
    }

    static PASTEBOARD_TEST_LOCK: Mutex<()> = Mutex::new(());

    fn unique_pasteboard() -> Option<(MutexGuard<'static, ()>, Pasteboard)> {
        let guard = PASTEBOARD_TEST_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        Pasteboard::unique().map(|pasteboard| (guard, pasteboard))
    }

    fn simulate_external_file_copy(pasteboard: &Pasteboard, paths: &[&str]) {
        let items = paths
            .iter()
            .map(|path| {
                let item = NSPasteboardItem::new();
                let file_url = url::Url::from_file_path(path).expect("absolute test path");
                let file_url = NSString::from_str(file_url.as_str());
                assert!(item.setString_forType(&file_url, unsafe { NSPasteboardTypeFileURL }));
                ProtocolObject::<dyn NSPasteboardWriting>::from_retained(item)
            })
            .collect::<Vec<_>>();
        let items = NSArray::from_retained_slice(&items);
        pasteboard.inner.clearContents();
        assert!(pasteboard.inner.writeObjects(&items));

        let joined = NSString::from_str(&paths.join("\n"));
        assert!(
            pasteboard
                .inner
                .setString_forType(&joined, unsafe { NSPasteboardTypeString })
        );
    }

    #[test]
    fn test_string() {
        let (_guard, pasteboard) = unique_pasteboard!();
        assert_eq!(pasteboard.read(), None);

        let item = ClipboardItem::new_string("1".to_string());
        pasteboard.write(item.clone());
        assert_eq!(pasteboard.read(), Some(item));

        let item = ClipboardItem {
            entries: vec![ClipboardEntry::String(
                ClipboardString::new("2".to_string()).with_json_metadata(vec![3, 4]),
            )],
        };
        pasteboard.write(item.clone());
        assert_eq!(pasteboard.read(), Some(item));

        let text_from_other_app = "text from other app";
        let bytes = NSData::with_bytes(text_from_other_app.as_bytes());
        pasteboard
            .inner
            .setData_forType(Some(&bytes), unsafe { NSPasteboardTypeString });
        assert_eq!(
            pasteboard.read(),
            Some(ClipboardItem::new_string(text_from_other_app.to_string()))
        );
    }

    #[test]
    fn test_custom_types_are_owned_by_the_pasteboard() {
        let (_guard, pasteboard) = unique_pasteboard!();

        assert_eq!(pasteboard.text_hash_type.to_string(), "zed-text-hash");
        assert_eq!(pasteboard.metadata_type.to_string(), "zed-metadata");
    }

    #[test]
    fn test_read_external_path() {
        let (_guard, pasteboard) = unique_pasteboard!();

        simulate_external_file_copy(&pasteboard, &["/test.txt"]);

        let item = pasteboard.read().expect("should read clipboard item");

        // Test both ExternalPaths and String entries exist
        assert_eq!(item.entries.len(), 2);

        // Test first entry is ExternalPaths
        match &item.entries[0] {
            ClipboardEntry::ExternalPaths(ep) => {
                assert_eq!(ep.paths(), &[PathBuf::from("/test.txt")]);
            }
            other => panic!("expected ExternalPaths, got {:?}", other),
        }

        // Test second entry is String
        match &item.entries[1] {
            ClipboardEntry::String(s) => {
                assert_eq!(s.text(), "/test.txt");
            }
            other => panic!("expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_read_external_paths_with_spaces() {
        let (_guard, pasteboard) = unique_pasteboard!();
        let paths = ["/some file with spaces.txt"];

        simulate_external_file_copy(&pasteboard, &paths);

        let item = pasteboard.read().expect("should read clipboard item");

        match &item.entries[0] {
            ClipboardEntry::ExternalPaths(ep) => {
                assert_eq!(ep.paths(), &[PathBuf::from("/some file with spaces.txt")]);
            }
            other => panic!("expected ExternalPaths, got {:?}", other),
        }
    }

    #[test]
    fn test_read_multiple_external_paths() {
        let (_guard, pasteboard) = unique_pasteboard!();
        let paths = ["/file.txt", "/image.png"];

        simulate_external_file_copy(&pasteboard, &paths);

        let item = pasteboard.read().expect("should read clipboard item");
        assert_eq!(item.entries.len(), 2);

        // Test both ExternalPaths and String entries exist
        match &item.entries[0] {
            ClipboardEntry::ExternalPaths(ep) => {
                assert_eq!(
                    ep.paths(),
                    &[PathBuf::from("/file.txt"), PathBuf::from("/image.png"),]
                );
            }
            other => panic!("expected ExternalPaths, got {:?}", other),
        }

        match &item.entries[1] {
            ClipboardEntry::String(s) => {
                assert_eq!(s.text(), "/file.txt\n/image.png");
                assert_eq!(s.metadata, None);
            }
            other => panic!("expected String, got {:?}", other),
        }
    }

    #[test]
    fn test_read_image() {
        let (_guard, pasteboard) = unique_pasteboard!();

        // Smallest valid PNG: 1x1 transparent pixel
        let png_bytes: &[u8] = &[
            0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1F, 0x15, 0xC4, 0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9C, 0x62, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0xE5, 0x27, 0xDE, 0xFC, 0x00, 0x00,
            0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, 0x42, 0x60, 0x82,
        ];

        let data = NSData::with_bytes(png_bytes);
        pasteboard
            .inner
            .setData_forType(Some(&data), unsafe { NSPasteboardTypePNG });

        let item = pasteboard.read().expect("should read PNG image");

        // Test Image entry exists
        assert_eq!(item.entries.len(), 1);
        match &item.entries[0] {
            ClipboardEntry::Image(img) => {
                assert_eq!(img.format, ImageFormat::Png);
                assert_eq!(img.bytes, png_bytes);
            }
            other => panic!("expected Image, got {:?}", other),
        }
    }
}

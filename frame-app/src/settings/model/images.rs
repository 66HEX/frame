#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::settings) struct ImageEncodingOptionDefinition {
    pub(in crate::settings) id: &'static str,
    pub(in crate::settings) label: &'static str,
    pub(in crate::settings) caption: &'static str,
}

pub(in crate::settings) const IMAGE_JPEG_HUFFMAN_OPTIONS: [ImageEncodingOptionDefinition; 2] = [
    ImageEncodingOptionDefinition {
        id: "optimal",
        label: "Optimized",
        caption: "Smaller files, slower encode",
    },
    ImageEncodingOptionDefinition {
        id: "default",
        label: "Default",
        caption: "Fast standard tables",
    },
];

pub(in crate::settings) const IMAGE_WEBP_PRESET_OPTIONS: [ImageEncodingOptionDefinition; 6] = [
    ImageEncodingOptionDefinition {
        id: "default",
        label: "Default",
        caption: "Balanced encoder defaults",
    },
    ImageEncodingOptionDefinition {
        id: "picture",
        label: "Picture",
        caption: "Portraits and indoor shots",
    },
    ImageEncodingOptionDefinition {
        id: "photo",
        label: "Photo",
        caption: "Natural outdoor images",
    },
    ImageEncodingOptionDefinition {
        id: "drawing",
        label: "Drawing",
        caption: "Lines and high contrast",
    },
    ImageEncodingOptionDefinition {
        id: "icon",
        label: "Icon",
        caption: "Small colorful graphics",
    },
    ImageEncodingOptionDefinition {
        id: "text",
        label: "Text",
        caption: "Text-like imagery",
    },
];

pub(in crate::settings) const IMAGE_PNG_PREDICTION_OPTIONS: [ImageEncodingOptionDefinition; 6] = [
    ImageEncodingOptionDefinition {
        id: "paeth",
        label: "Paeth",
        caption: "PNG default",
    },
    ImageEncodingOptionDefinition {
        id: "mixed",
        label: "Mixed",
        caption: "Adaptive prediction",
    },
    ImageEncodingOptionDefinition {
        id: "sub",
        label: "Sub",
        caption: "Horizontal predictor",
    },
    ImageEncodingOptionDefinition {
        id: "up",
        label: "Up",
        caption: "Vertical predictor",
    },
    ImageEncodingOptionDefinition {
        id: "avg",
        label: "Average",
        caption: "Average predictor",
    },
    ImageEncodingOptionDefinition {
        id: "none",
        label: "None",
        caption: "No prediction",
    },
];

pub(in crate::settings) const IMAGE_TIFF_COMPRESSION_OPTIONS: [ImageEncodingOptionDefinition; 4] = [
    ImageEncodingOptionDefinition {
        id: "packbits",
        label: "PackBits",
        caption: "Fast lossless default",
    },
    ImageEncodingOptionDefinition {
        id: "lzw",
        label: "LZW",
        caption: "Broad lossless support",
    },
    ImageEncodingOptionDefinition {
        id: "deflate",
        label: "Deflate",
        caption: "Smaller lossless output",
    },
    ImageEncodingOptionDefinition {
        id: "raw",
        label: "Raw",
        caption: "Uncompressed pixels",
    },
];

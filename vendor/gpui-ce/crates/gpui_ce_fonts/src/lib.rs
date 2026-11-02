pub struct FontFixture {
    pub family: &'static str,
    pub data: &'static [u8],
}

impl std::ops::Deref for FontFixture {
    type Target = &'static [u8];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

pub const IBM_PLEX: FontFixture = FontFixture {
    family: "IBM Plex Sans",
    data: include_bytes!("../../../assets/fonts/ibm-plex-sans/IBMPlexSans-Regular.ttf"),
};

pub const IBM_PLEX_ITALIC: FontFixture = FontFixture {
    family: "IBM Plex Sans",
    data: include_bytes!("../../../assets/fonts/ibm-plex-sans/IBMPlexSans-Italic.ttf"),
};

pub const IBM_PLEX_SEMIBOLD: FontFixture = FontFixture {
    family: IBM_PLEX.family,
    data: include_bytes!("../../../assets/fonts/ibm-plex-sans/IBMPlexSans-SemiBold.ttf"),
};

pub const IBM_PLEX_SEMIBOLD_ITALIC: FontFixture = FontFixture {
    family: IBM_PLEX.family,
    data: include_bytes!("../../../assets/fonts/ibm-plex-sans/IBMPlexSans-SemiBoldItalic.ttf"),
};

pub const LILEX: FontFixture = FontFixture {
    family: "Lilex",
    data: include_bytes!("../../../assets/fonts/lilex/Lilex-Regular.ttf"),
};

pub const LILEX_BOLD: FontFixture = FontFixture {
    family: "Lilex",
    data: include_bytes!("../../../assets/fonts/lilex/Lilex-Bold.ttf"),
};

pub const SOURCE_SERIF: FontFixture = FontFixture {
    family: "Source Serif 4",
    data: include_bytes!("../../../assets/fonts/source-serif-4/SourceSerif4[opsz-wght].ttf"),
};

pub const NOTO_SANS: FontFixture = FontFixture {
    family: "Noto Sans",
    data: include_bytes!("../../../assets/fonts/noto-sans/NotoSans[wdth-wght].subset.ttf"),
};

pub const NOTO_ARABIC: FontFixture = FontFixture {
    family: "Noto Sans Arabic",
    data: include_bytes!("../../../assets/fonts/noto-sans-arabic/NotoSansArabic-Regular.ttf"),
};

pub const NOTO_HEBREW: FontFixture = FontFixture {
    family: "Noto Sans Hebrew",
    data: include_bytes!("../../../assets/fonts/noto-sans-hebrew/NotoSansHebrew-Regular.ttf"),
};

pub const NOTO_COLOR_EMOJI: FontFixture = FontFixture {
    family: "Noto Color Emoji",
    data: include_bytes!("../../../assets/fonts/noto-color-emoji/NotoColorEmoji.subset.ttf"),
};

pub const WIDTH_REGULAR: FontFixture = FontFixture {
    family: "GPUI Width Fixture",
    data: include_bytes!("../../../assets/fonts/noto-sans/WidthFixture-Regular.ttf"),
};

pub const WIDTH_CONDENSED: FontFixture = FontFixture {
    family: WIDTH_REGULAR.family,
    data: include_bytes!("../../../assets/fonts/noto-sans/WidthFixture-Condensed.ttf"),
};

use ratatui::style::Color;
use std::sync::LazyLock;

pub struct Theme {
    pub bg: Color,
    pub bg0: Color,
    pub bg1: Color,
    pub bg2: Color,
    pub bg3: Color,
    pub bg4: Color,
    pub fg: Color,
    pub grey0: Color,
    pub grey1: Color,
    pub grey2: Color,
    pub red: Color,
    pub orange: Color,
    pub yellow: Color,
    pub green: Color,
    pub aqua: Color,
    pub blue: Color,
    pub purple: Color,
}

impl Theme {
    /// Everforest Dark Hard — true color (24-bit RGB).
    /// Requires COLORTERM=truecolor or 24bit.
    const fn everforest_rgb() -> Self {
        Self {
            bg:     Color::Rgb(0x27, 0x2e, 0x33),
            bg0:    Color::Rgb(0x23, 0x2a, 0x2e),
            bg1:    Color::Rgb(0x2e, 0x38, 0x3c),
            bg2:    Color::Rgb(0x37, 0x41, 0x45),
            bg3:    Color::Rgb(0x41, 0x4b, 0x50),
            bg4:    Color::Rgb(0x49, 0x51, 0x56),
            fg:     Color::Rgb(0xd3, 0xc6, 0xaa),
            grey0:  Color::Rgb(0x7a, 0x84, 0x78),
            grey1:  Color::Rgb(0x85, 0x92, 0x89),
            grey2:  Color::Rgb(0x9d, 0xa9, 0xa0),
            red:    Color::Rgb(0xe6, 0x7e, 0x80),
            orange: Color::Rgb(0xe6, 0x98, 0x75),
            yellow: Color::Rgb(0xdb, 0xbc, 0x7f),
            green:  Color::Rgb(0xa7, 0xc0, 0x80),
            aqua:   Color::Rgb(0x83, 0xc0, 0x92),
            blue:   Color::Rgb(0x7f, 0xbb, 0xb3),
            purple: Color::Rgb(0xd6, 0x99, 0xb6),
        }
    }

    /// Everforest Dark Hard — 256-color approximation.
    /// Closest xterm-256 indices for each Everforest color.
    const fn everforest_256() -> Self {
        Self {
            bg:     Color::Indexed(236),  // #303030
            bg0:    Color::Indexed(235),  // #262626
            bg1:    Color::Indexed(237),  // #3a3a3a
            bg2:    Color::Indexed(238),  // #444444
            bg3:    Color::Indexed(239),  // #4e4e4e
            bg4:    Color::Indexed(240),  // #585858
            fg:     Color::Indexed(187),  // #d7d7af
            grey0:  Color::Indexed(102),  // #878787
            grey1:  Color::Indexed(108),  // #87af87
            grey2:  Color::Indexed(144),  // #afaf87
            red:    Color::Indexed(174),  // #d78787
            orange: Color::Indexed(180),  // #d7af87
            yellow: Color::Indexed(186),  // #d7d787
            green:  Color::Indexed(142),  // #afaf00 -> 150 #afd787
            aqua:   Color::Indexed(108),  // #87af87
            blue:   Color::Indexed(109),  // #87afaf
            purple: Color::Indexed(175),  // #d787af
        }
    }
}

fn supports_truecolor() -> bool {
    std::env::var("COLORTERM")
        .map(|v| v == "truecolor" || v == "24bit")
        .unwrap_or(false)
}

pub static THEME: LazyLock<Theme> = LazyLock::new(|| {
    if supports_truecolor() {
        Theme::everforest_rgb()
    } else {
        Theme::everforest_256()
    }
});

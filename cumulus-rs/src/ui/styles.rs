use ratatui::style::{Color, Modifier, Style};

/// All named color/style slots used across the cumulus UI.
///
/// Adding a new theme: implement a constructor (`Theme::my_theme() -> Theme`)
/// and add an entry to `ALL_THEMES` and `from_name`. No draw function needs to change.
#[derive(Debug, Clone)]
pub struct Theme {
    // ── Background ────────────────────────────────────────────────────────────
    /// Main panel / app background.
    pub background: Color,

    // ── Chrome ────────────────────────────────────────────────────────────────
    /// Primary border color (blocks, panels).
    pub border: Color,
    /// Dimmer border color (inner / secondary panels).
    pub border_dim: Color,

    // ── Text ─────────────────────────────────────────────────────────────────
    /// Normal body text.
    pub text: Color,
    /// Secondary / muted text (timestamps, labels).
    pub text_dim: Color,
    /// Accent text (service names, paths, highlights).
    pub text_accent: Color,

    // ── Table / list selection ────────────────────────────────────────────────
    /// Selected row background.
    pub selection_bg: Color,
    /// Selected row foreground.
    pub selection_fg: Color,

    // ── Status bar ────────────────────────────────────────────────────────────
    /// Profile pill background.
    pub profile_bg: Color,
    /// Profile pill foreground.
    pub profile_fg: Color,
    /// Region pill background.
    pub region_bg: Color,
    /// Region pill foreground.
    pub region_fg: Color,
    /// Active breadcrumb foreground.
    pub breadcrumb_active: Color,
    /// Inactive breadcrumb foreground.
    pub breadcrumb_inactive: Color,

    // ── Key hint badges ───────────────────────────────────────────────────────
    /// Foreground on key badge (dark so the bg is readable).
    pub key_fg: Color,
    /// Background of key badge.
    pub key_bg: Color,
    /// Description text next to a key badge.
    pub key_desc: Color,

    // ── Diff / edit / status ──────────────────────────────────────────────────
    pub diff_added_fg: Color,
    pub diff_added_bg: Color,
    pub diff_removed_fg: Color,
    pub diff_removed_bg: Color,

    // ── Misc ─────────────────────────────────────────────────────────────────
    /// `──` separator lines.
    pub separator: Color,
    /// Section headers in help / detail views.
    pub section_header: Color,
    /// Error message text.
    pub error: Color,
    /// Success / informational status text.
    pub status: Color,
}

/// Registry of all bundled themes: (id, display name).
/// Order here is the order shown in the theme picker.
pub const ALL_THEMES: &[(&str, &str)] = &[
    ("tokyonight", "Tokyo Night"),
    ("gruvbox", "Gruvbox Dark"),
    ("catppuccin", "Catppuccin Mocha"),
    ("nord", "Nord"),
    ("dracula", "Dracula"),
    ("rosepine", "Rosé Pine"),
    ("catppuccin_latte", "Catppuccin Latte"),
    ("rosepine_dawn", "Rosé Pine Dawn"),
    ("gruvbox_light", "Gruvbox Light"),
    ("solarized_dark", "Solarized Dark"),
    ("solarized_light", "Solarized Light"),
    ("everforest_dark", "Everforest Dark"),
    ("onedark", "One Dark"),
];

#[allow(dead_code)]
impl Theme {
    /// Resolve a theme by id (case-insensitive).  Unknown ids fall back to
    /// `tokyonight` so old/bad configs never break.
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "tokyonight" => Self::tokyonight(),
            "gruvbox" => Self::gruvbox(),
            "catppuccin" => Self::catppuccin(),
            "nord" => Self::nord(),
            "dracula" => Self::dracula(),
            "rosepine" => Self::rosepine(),
            "catppuccin_latte" => Self::catppuccin_latte(),
            "rosepine_dawn" => Self::rosepine_dawn(),
            "gruvbox_light" => Self::gruvbox_light(),
            "solarized_dark" => Self::solarized_dark(),
            "solarized_light" => Self::solarized_light(),
            "everforest_dark" => Self::everforest_dark(),
            "onedark" => Self::onedark(),
            _ => Self::tokyonight(),
        }
    }

    /// Return the index of `name` in `ALL_THEMES`, defaulting to 0.
    pub fn index_of(name: &str) -> usize {
        let lower = name.to_lowercase();
        ALL_THEMES
            .iter()
            .position(|(id, _)| *id == lower.as_str())
            .unwrap_or(0)
    }

    // ── Theme constructors ────────────────────────────────────────────────────

    /// Tokyo Night Dark — https://github.com/folke/tokyonight.nvim
    pub fn tokyonight() -> Self {
        Self {
            background: Color::Rgb(26, 27, 38),
            border: Color::Rgb(86, 95, 137),
            border_dim: Color::Rgb(54, 58, 79),
            text: Color::Rgb(192, 202, 245),
            text_dim: Color::Rgb(86, 95, 137),
            text_accent: Color::Rgb(122, 162, 247),
            selection_bg: Color::Rgb(40, 46, 74),
            selection_fg: Color::Rgb(192, 202, 245),
            profile_bg: Color::Rgb(122, 162, 247),
            profile_fg: Color::Rgb(26, 27, 38),
            region_bg: Color::Rgb(158, 206, 106),
            region_fg: Color::Rgb(26, 27, 38),
            breadcrumb_active: Color::Rgb(224, 175, 104),
            breadcrumb_inactive: Color::Rgb(86, 95, 137),
            key_fg: Color::Rgb(26, 27, 38),
            key_bg: Color::Rgb(224, 175, 104),
            key_desc: Color::Rgb(192, 202, 245),
            diff_added_fg: Color::Rgb(158, 206, 106),
            diff_added_bg: Color::Rgb(29, 43, 29),
            diff_removed_fg: Color::Rgb(247, 118, 142),
            diff_removed_bg: Color::Rgb(43, 23, 28),
            separator: Color::Rgb(54, 58, 79),
            section_header: Color::Rgb(187, 154, 247),
            error: Color::Rgb(247, 118, 142),
            status: Color::Rgb(158, 206, 106),
        }
    }

    /// Gruvbox Dark Hard — https://github.com/morhetz/gruvbox
    pub fn gruvbox() -> Self {
        Self {
            background: Color::Rgb(29, 32, 33),
            border: Color::Rgb(80, 73, 69),
            border_dim: Color::Rgb(60, 56, 54),
            text: Color::Rgb(235, 219, 178),
            text_dim: Color::Rgb(146, 131, 116),
            text_accent: Color::Rgb(131, 165, 152),
            selection_bg: Color::Rgb(60, 56, 54),
            selection_fg: Color::Rgb(235, 219, 178),
            profile_bg: Color::Rgb(131, 165, 152),
            profile_fg: Color::Rgb(29, 32, 33),
            region_bg: Color::Rgb(184, 187, 38),
            region_fg: Color::Rgb(29, 32, 33),
            breadcrumb_active: Color::Rgb(250, 189, 47),
            breadcrumb_inactive: Color::Rgb(146, 131, 116),
            key_fg: Color::Rgb(29, 32, 33),
            key_bg: Color::Rgb(250, 189, 47),
            key_desc: Color::Rgb(235, 219, 178),
            diff_added_fg: Color::Rgb(184, 187, 38),
            diff_added_bg: Color::Rgb(36, 43, 27),
            diff_removed_fg: Color::Rgb(251, 73, 52),
            diff_removed_bg: Color::Rgb(43, 24, 20),
            separator: Color::Rgb(60, 56, 54),
            section_header: Color::Rgb(211, 134, 155),
            error: Color::Rgb(251, 73, 52),
            status: Color::Rgb(184, 187, 38),
        }
    }

    /// Catppuccin Mocha — https://github.com/catppuccin/catppuccin
    pub fn catppuccin() -> Self {
        Self {
            background: Color::Rgb(30, 30, 46),
            border: Color::Rgb(88, 91, 112),
            border_dim: Color::Rgb(49, 50, 68),
            text: Color::Rgb(205, 214, 244),
            text_dim: Color::Rgb(108, 112, 134),
            text_accent: Color::Rgb(137, 180, 250),
            selection_bg: Color::Rgb(49, 50, 68),
            selection_fg: Color::Rgb(205, 214, 244),
            profile_bg: Color::Rgb(137, 180, 250),
            profile_fg: Color::Rgb(17, 17, 27),
            region_bg: Color::Rgb(166, 227, 161),
            region_fg: Color::Rgb(17, 17, 27),
            breadcrumb_active: Color::Rgb(249, 226, 175),
            breadcrumb_inactive: Color::Rgb(108, 112, 134),
            key_fg: Color::Rgb(17, 17, 27),
            key_bg: Color::Rgb(249, 226, 175),
            key_desc: Color::Rgb(205, 214, 244),
            diff_added_fg: Color::Rgb(166, 227, 161),
            diff_added_bg: Color::Rgb(28, 42, 34),
            diff_removed_fg: Color::Rgb(243, 139, 168),
            diff_removed_bg: Color::Rgb(42, 26, 34),
            separator: Color::Rgb(49, 50, 68),
            section_header: Color::Rgb(203, 166, 247),
            error: Color::Rgb(243, 139, 168),
            status: Color::Rgb(166, 227, 161),
        }
    }

    /// Nord — https://www.nordtheme.com
    pub fn nord() -> Self {
        Self {
            background: Color::Rgb(46, 52, 64),
            border: Color::Rgb(76, 86, 106),
            border_dim: Color::Rgb(59, 66, 82),
            text: Color::Rgb(236, 239, 244),
            text_dim: Color::Rgb(76, 86, 106),
            text_accent: Color::Rgb(136, 192, 208),
            selection_bg: Color::Rgb(67, 76, 94),
            selection_fg: Color::Rgb(236, 239, 244),
            profile_bg: Color::Rgb(136, 192, 208),
            profile_fg: Color::Rgb(46, 52, 64),
            region_bg: Color::Rgb(163, 190, 140),
            region_fg: Color::Rgb(46, 52, 64),
            breadcrumb_active: Color::Rgb(235, 203, 139),
            breadcrumb_inactive: Color::Rgb(76, 86, 106),
            key_fg: Color::Rgb(46, 52, 64),
            key_bg: Color::Rgb(235, 203, 139),
            key_desc: Color::Rgb(236, 239, 244),
            diff_added_fg: Color::Rgb(163, 190, 140),
            diff_added_bg: Color::Rgb(30, 44, 32),
            diff_removed_fg: Color::Rgb(191, 97, 106),
            diff_removed_bg: Color::Rgb(42, 24, 26),
            separator: Color::Rgb(59, 66, 82),
            section_header: Color::Rgb(180, 142, 173),
            error: Color::Rgb(191, 97, 106),
            status: Color::Rgb(163, 190, 140),
        }
    }

    /// Dracula — https://draculatheme.com
    pub fn dracula() -> Self {
        Self {
            background: Color::Rgb(40, 42, 54),
            border: Color::Rgb(98, 114, 164),
            border_dim: Color::Rgb(68, 71, 90),
            text: Color::Rgb(248, 248, 242),
            text_dim: Color::Rgb(98, 114, 164),
            text_accent: Color::Rgb(139, 233, 253),
            selection_bg: Color::Rgb(68, 71, 90),
            selection_fg: Color::Rgb(248, 248, 242),
            profile_bg: Color::Rgb(139, 233, 253),
            profile_fg: Color::Rgb(40, 42, 54),
            region_bg: Color::Rgb(80, 250, 123),
            region_fg: Color::Rgb(40, 42, 54),
            breadcrumb_active: Color::Rgb(255, 184, 108),
            breadcrumb_inactive: Color::Rgb(98, 114, 164),
            key_fg: Color::Rgb(40, 42, 54),
            key_bg: Color::Rgb(255, 184, 108),
            key_desc: Color::Rgb(248, 248, 242),
            diff_added_fg: Color::Rgb(80, 250, 123),
            diff_added_bg: Color::Rgb(22, 46, 30),
            diff_removed_fg: Color::Rgb(255, 85, 85),
            diff_removed_bg: Color::Rgb(46, 20, 20),
            separator: Color::Rgb(68, 71, 90),
            section_header: Color::Rgb(189, 147, 249),
            error: Color::Rgb(255, 85, 85),
            status: Color::Rgb(80, 250, 123),
        }
    }

    /// Rosé Pine — https://rosepinetheme.com (Main/dark variant)
    pub fn rosepine() -> Self {
        Self {
            background: Color::Rgb(25, 23, 36),
            border: Color::Rgb(64, 61, 82),
            border_dim: Color::Rgb(38, 35, 58),
            text: Color::Rgb(224, 222, 244),
            text_dim: Color::Rgb(110, 106, 134),
            text_accent: Color::Rgb(156, 207, 216),
            selection_bg: Color::Rgb(38, 35, 58),
            selection_fg: Color::Rgb(224, 222, 244),
            profile_bg: Color::Rgb(156, 207, 216),
            profile_fg: Color::Rgb(25, 23, 36),
            region_bg: Color::Rgb(246, 193, 119),
            region_fg: Color::Rgb(25, 23, 36),
            breadcrumb_active: Color::Rgb(246, 193, 119),
            breadcrumb_inactive: Color::Rgb(110, 106, 134),
            key_fg: Color::Rgb(25, 23, 36),
            key_bg: Color::Rgb(246, 193, 119),
            key_desc: Color::Rgb(224, 222, 244),
            diff_added_fg: Color::Rgb(156, 207, 216),
            diff_added_bg: Color::Rgb(22, 37, 42),
            diff_removed_fg: Color::Rgb(235, 111, 146),
            diff_removed_bg: Color::Rgb(42, 22, 32),
            separator: Color::Rgb(38, 35, 58),
            section_header: Color::Rgb(196, 167, 231),
            error: Color::Rgb(235, 111, 146),
            status: Color::Rgb(156, 207, 216),
        }
    }

    /// Catppuccin Latte — https://github.com/catppuccin/catppuccin (light variant)
    pub fn catppuccin_latte() -> Self {
        Self {
            background: Color::Rgb(239, 241, 245),
            border: Color::Rgb(172, 176, 190),
            border_dim: Color::Rgb(204, 208, 218),
            text: Color::Rgb(76, 79, 105),
            text_dim: Color::Rgb(156, 160, 176),
            text_accent: Color::Rgb(30, 102, 245),
            selection_bg: Color::Rgb(204, 208, 218),
            selection_fg: Color::Rgb(76, 79, 105),
            profile_bg: Color::Rgb(30, 102, 245),
            profile_fg: Color::Rgb(239, 241, 245),
            region_bg: Color::Rgb(64, 160, 43),
            region_fg: Color::Rgb(239, 241, 245),
            breadcrumb_active: Color::Rgb(223, 142, 29),
            breadcrumb_inactive: Color::Rgb(156, 160, 176),
            key_fg: Color::Rgb(239, 241, 245),
            key_bg: Color::Rgb(30, 102, 245),
            key_desc: Color::Rgb(76, 79, 105),
            diff_added_fg: Color::Rgb(64, 160, 43),
            diff_added_bg: Color::Rgb(213, 237, 209),
            diff_removed_fg: Color::Rgb(210, 15, 57),
            diff_removed_bg: Color::Rgb(249, 210, 218),
            separator: Color::Rgb(204, 208, 218),
            section_header: Color::Rgb(136, 57, 239),
            error: Color::Rgb(210, 15, 57),
            status: Color::Rgb(64, 160, 43),
        }
    }

    /// Rosé Pine Dawn — https://rosepinetheme.com (light variant)
    pub fn rosepine_dawn() -> Self {
        Self {
            background: Color::Rgb(250, 244, 237),
            border: Color::Rgb(215, 210, 195),
            border_dim: Color::Rgb(242, 233, 222),
            text: Color::Rgb(87, 82, 121),
            text_dim: Color::Rgb(152, 147, 165),
            text_accent: Color::Rgb(40, 105, 131),
            selection_bg: Color::Rgb(242, 233, 222),
            selection_fg: Color::Rgb(87, 82, 121),
            profile_bg: Color::Rgb(40, 105, 131),
            profile_fg: Color::Rgb(250, 244, 237),
            region_bg: Color::Rgb(234, 157, 52),
            region_fg: Color::Rgb(250, 244, 237),
            breadcrumb_active: Color::Rgb(234, 157, 52),
            breadcrumb_inactive: Color::Rgb(152, 147, 165),
            key_fg: Color::Rgb(250, 244, 237),
            key_bg: Color::Rgb(40, 105, 131),
            key_desc: Color::Rgb(87, 82, 121),
            diff_added_fg: Color::Rgb(40, 105, 131),
            diff_added_bg: Color::Rgb(214, 235, 232),
            diff_removed_fg: Color::Rgb(180, 99, 122),
            diff_removed_bg: Color::Rgb(248, 220, 228),
            separator: Color::Rgb(215, 210, 195),
            section_header: Color::Rgb(144, 122, 169),
            error: Color::Rgb(180, 99, 122),
            status: Color::Rgb(40, 105, 131),
        }
    }

    /// Gruvbox Light Hard — https://github.com/morhetz/gruvbox
    pub fn gruvbox_light() -> Self {
        Self {
            background: Color::Rgb(249, 245, 215),
            border: Color::Rgb(189, 174, 147),
            border_dim: Color::Rgb(213, 196, 161),
            text: Color::Rgb(60, 56, 54),
            text_dim: Color::Rgb(124, 111, 100),
            text_accent: Color::Rgb(69, 133, 136),
            selection_bg: Color::Rgb(213, 196, 161),
            selection_fg: Color::Rgb(60, 56, 54),
            profile_bg: Color::Rgb(69, 133, 136),
            profile_fg: Color::Rgb(249, 245, 215),
            region_bg: Color::Rgb(121, 116, 14),
            region_fg: Color::Rgb(249, 245, 215),
            breadcrumb_active: Color::Rgb(181, 118, 20),
            breadcrumb_inactive: Color::Rgb(124, 111, 100),
            key_fg: Color::Rgb(249, 245, 215),
            key_bg: Color::Rgb(181, 118, 20),
            key_desc: Color::Rgb(60, 56, 54),
            diff_added_fg: Color::Rgb(121, 116, 14),
            diff_added_bg: Color::Rgb(215, 232, 196),
            diff_removed_fg: Color::Rgb(157, 0, 6),
            diff_removed_bg: Color::Rgb(248, 210, 198),
            separator: Color::Rgb(213, 196, 161),
            section_header: Color::Rgb(143, 63, 113),
            error: Color::Rgb(157, 0, 6),
            status: Color::Rgb(121, 116, 14),
        }
    }

    /// Solarized Dark — https://ethanschoonover.com/solarized/
    pub fn solarized_dark() -> Self {
        Self {
            background: Color::Rgb(0, 43, 54),       // base03
            border: Color::Rgb(88, 110, 117),        // base01
            border_dim: Color::Rgb(7, 54, 66),       // base02
            text: Color::Rgb(131, 148, 150),         // base0
            text_dim: Color::Rgb(88, 110, 117),      // base01
            text_accent: Color::Rgb(38, 139, 210),   // blue
            selection_bg: Color::Rgb(7, 54, 66),     // base02
            selection_fg: Color::Rgb(147, 161, 161), // base1
            profile_bg: Color::Rgb(38, 139, 210),    // blue
            profile_fg: Color::Rgb(0, 43, 54),
            region_bg: Color::Rgb(133, 153, 0), // green
            region_fg: Color::Rgb(0, 43, 54),
            breadcrumb_active: Color::Rgb(181, 137, 0), // yellow
            breadcrumb_inactive: Color::Rgb(88, 110, 117),
            key_fg: Color::Rgb(0, 43, 54),
            key_bg: Color::Rgb(181, 137, 0), // yellow
            key_desc: Color::Rgb(131, 148, 150),
            diff_added_fg: Color::Rgb(133, 153, 0),
            diff_added_bg: Color::Rgb(10, 40, 10),
            diff_removed_fg: Color::Rgb(220, 50, 47),
            diff_removed_bg: Color::Rgb(40, 10, 10),
            separator: Color::Rgb(7, 54, 66),
            section_header: Color::Rgb(108, 113, 196), // violet
            error: Color::Rgb(220, 50, 47),
            status: Color::Rgb(133, 153, 0),
        }
    }

    /// Solarized Light — https://ethanschoonover.com/solarized/
    pub fn solarized_light() -> Self {
        Self {
            background: Color::Rgb(253, 246, 227),
            border: Color::Rgb(147, 161, 161),
            border_dim: Color::Rgb(238, 232, 213),
            text: Color::Rgb(101, 123, 131),
            text_dim: Color::Rgb(147, 161, 161),
            text_accent: Color::Rgb(38, 139, 210),
            selection_bg: Color::Rgb(238, 232, 213),
            selection_fg: Color::Rgb(101, 123, 131),
            profile_bg: Color::Rgb(38, 139, 210),
            profile_fg: Color::Rgb(253, 246, 227),
            region_bg: Color::Rgb(133, 153, 0),
            region_fg: Color::Rgb(253, 246, 227),
            breadcrumb_active: Color::Rgb(181, 137, 0),
            breadcrumb_inactive: Color::Rgb(147, 161, 161),
            key_fg: Color::Rgb(253, 246, 227),
            key_bg: Color::Rgb(38, 139, 210),
            key_desc: Color::Rgb(101, 123, 131),
            diff_added_fg: Color::Rgb(133, 153, 0),
            diff_added_bg: Color::Rgb(220, 237, 193),
            diff_removed_fg: Color::Rgb(220, 50, 47),
            diff_removed_bg: Color::Rgb(250, 213, 212),
            separator: Color::Rgb(238, 232, 213),
            section_header: Color::Rgb(108, 113, 196),
            error: Color::Rgb(220, 50, 47),
            status: Color::Rgb(133, 153, 0),
        }
    }

    /// Everforest Dark — https://github.com/sainnhe/everforest
    pub fn everforest_dark() -> Self {
        Self {
            background: Color::Rgb(35, 38, 46),     // bg0
            border: Color::Rgb(83, 100, 87),        // bg5
            border_dim: Color::Rgb(53, 57, 66),     // bg2
            text: Color::Rgb(211, 198, 170),        // fg
            text_dim: Color::Rgb(131, 143, 123),    // grey2
            text_accent: Color::Rgb(127, 187, 179), // aqua
            selection_bg: Color::Rgb(65, 71, 84),   // bg3
            selection_fg: Color::Rgb(211, 198, 170),
            profile_bg: Color::Rgb(127, 187, 179), // aqua
            profile_fg: Color::Rgb(35, 38, 46),
            region_bg: Color::Rgb(167, 192, 128), // green
            region_fg: Color::Rgb(35, 38, 46),
            breadcrumb_active: Color::Rgb(223, 199, 120), // yellow
            breadcrumb_inactive: Color::Rgb(131, 143, 123),
            key_fg: Color::Rgb(35, 38, 46),
            key_bg: Color::Rgb(223, 199, 120), // yellow
            key_desc: Color::Rgb(211, 198, 170),
            diff_added_fg: Color::Rgb(167, 192, 128),
            diff_added_bg: Color::Rgb(33, 47, 35),
            diff_removed_fg: Color::Rgb(230, 126, 128),
            diff_removed_bg: Color::Rgb(47, 30, 32),
            separator: Color::Rgb(53, 57, 66),
            section_header: Color::Rgb(214, 153, 182), // purple
            error: Color::Rgb(230, 126, 128),
            status: Color::Rgb(167, 192, 128),
        }
    }

    /// One Dark — https://github.com/atom/one-dark-syntax
    pub fn onedark() -> Self {
        Self {
            background: Color::Rgb(40, 44, 52),    // hue-core-0
            border: Color::Rgb(62, 68, 81),        // mono-3
            border_dim: Color::Rgb(49, 53, 63),    // hue-core-1
            text: Color::Rgb(171, 178, 191),       // mono-1
            text_dim: Color::Rgb(92, 99, 112),     // mono-3
            text_accent: Color::Rgb(97, 175, 239), // blue
            selection_bg: Color::Rgb(62, 68, 81),
            selection_fg: Color::Rgb(171, 178, 191),
            profile_bg: Color::Rgb(97, 175, 239), // blue
            profile_fg: Color::Rgb(40, 44, 52),
            region_bg: Color::Rgb(152, 195, 121), // green
            region_fg: Color::Rgb(40, 44, 52),
            breadcrumb_active: Color::Rgb(229, 192, 123), // yellow
            breadcrumb_inactive: Color::Rgb(92, 99, 112),
            key_fg: Color::Rgb(40, 44, 52),
            key_bg: Color::Rgb(229, 192, 123), // yellow
            key_desc: Color::Rgb(171, 178, 191),
            diff_added_fg: Color::Rgb(152, 195, 121),
            diff_added_bg: Color::Rgb(29, 43, 26),
            diff_removed_fg: Color::Rgb(224, 108, 117),
            diff_removed_bg: Color::Rgb(43, 24, 26),
            separator: Color::Rgb(49, 53, 63),
            section_header: Color::Rgb(198, 120, 221), // purple
            error: Color::Rgb(224, 108, 117),
            status: Color::Rgb(152, 195, 121),
        }
    }

    // ── Convenience style builders ────────────────────────────────────────────

    pub fn background_style(&self) -> Style {
        Style::default().bg(self.background)
    }

    pub fn border_style(&self) -> Style {
        Style::default().fg(self.border)
    }

    pub fn border_dim_style(&self) -> Style {
        Style::default().fg(self.border_dim)
    }

    pub fn text_style(&self) -> Style {
        Style::default().fg(self.text).bg(self.background)
    }

    pub fn text_dim_style(&self) -> Style {
        Style::default().fg(self.text_dim).bg(self.background)
    }

    pub fn text_accent_style(&self) -> Style {
        Style::default().fg(self.text_accent).bg(self.background)
    }

    pub fn selection_style(&self) -> Style {
        Style::default().bg(self.selection_bg).fg(self.selection_fg)
    }

    pub fn key_badge_style(&self) -> Style {
        Style::default()
            .fg(self.key_fg)
            .bg(self.key_bg)
            .add_modifier(Modifier::BOLD)
    }

    pub fn key_desc_style(&self) -> Style {
        Style::default().fg(self.key_desc).bg(self.background)
    }

    pub fn section_header_style(&self) -> Style {
        Style::default()
            .fg(self.section_header)
            .bg(self.background)
            .add_modifier(Modifier::BOLD)
    }

    pub fn separator_style(&self) -> Style {
        Style::default().fg(self.separator).bg(self.background)
    }

    pub fn error_style(&self) -> Style {
        Style::default().fg(self.error).bg(self.background)
    }

    pub fn status_style(&self) -> Style {
        Style::default().fg(self.status).bg(self.background)
    }
}

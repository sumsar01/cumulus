// Package ui provides shared lipgloss styles and theme constants used across
// all views in cumulus.
package ui

import (
	"github.com/charmbracelet/bubbles/table"
	"github.com/charmbracelet/lipgloss"
)

// ── Theme definition ──────────────────────────────────────────────────────────

// Theme holds the full colour palette for a cumulus colour scheme.
// All semantic colour slots are named so that components never need to know
// which specific hex value they are using.
type Theme struct {
	// Name is the machine-readable identifier stored in config.toml.
	Name string
	// DisplayName is the human-readable label shown in the theme picker.
	DisplayName string
	// Description is a short description shown alongside the name.
	Description string

	Primary   lipgloss.Color // main accent (links, titles, active elements)
	Secondary lipgloss.Color // secondary accent (subtitles, badges)
	Accent    lipgloss.Color // bright highlight (cursor, key hints)
	Muted     lipgloss.Color // dimmed foreground (labels, separators)
	Success   lipgloss.Color // success / healthy state
	Warning   lipgloss.Color // warning state
	Danger    lipgloss.Color // error / destructive action
	Bg        lipgloss.Color // main background
	Surface   lipgloss.Color // slightly elevated surface (modals, status bar)
	Border    lipgloss.Color // border / rule colour
	Highlight lipgloss.Color // selection background
	Text      lipgloss.Color // primary text
	Subtext   lipgloss.Color // secondary text
}

// ── Built-in theme presets ────────────────────────────────────────────────────

// ThemeTokyoNight is the classic dark blue/purple Tokyo Night palette.
var ThemeTokyoNight = Theme{
	Name:        "tokyo-night",
	DisplayName: "Tokyo Night",
	Description: "dark blue / purple",
	Primary:     lipgloss.Color("#7aa2f7"),
	Secondary:   lipgloss.Color("#bb9af7"),
	Accent:      lipgloss.Color("#7dcfff"),
	Muted:       lipgloss.Color("#565f89"),
	Success:     lipgloss.Color("#9ece6a"),
	Warning:     lipgloss.Color("#e0af68"),
	Danger:      lipgloss.Color("#f7768e"),
	Bg:          lipgloss.Color("#1a1b26"),
	Surface:     lipgloss.Color("#24283b"),
	Border:      lipgloss.Color("#414868"),
	Highlight:   lipgloss.Color("#283457"),
	Text:        lipgloss.Color("#c0caf5"),
	Subtext:     lipgloss.Color("#a9b1d6"),
}

// ThemeCatppuccinMocha is a warm pastel dark theme with pinks, peaches, and lavender.
var ThemeCatppuccinMocha = Theme{
	Name:        "catppuccin-mocha",
	DisplayName: "Catppuccin Mocha",
	Description: "warm pastel dark",
	Primary:     lipgloss.Color("#cba6f7"), // mauve
	Secondary:   lipgloss.Color("#f38ba8"), // red
	Accent:      lipgloss.Color("#89dceb"), // sky
	Muted:       lipgloss.Color("#6c7086"), // overlay0
	Success:     lipgloss.Color("#a6e3a1"), // green
	Warning:     lipgloss.Color("#f9e2af"), // yellow
	Danger:      lipgloss.Color("#f38ba8"), // red
	Bg:          lipgloss.Color("#1e1e2e"), // base
	Surface:     lipgloss.Color("#313244"), // surface0
	Border:      lipgloss.Color("#45475a"), // surface1
	Highlight:   lipgloss.Color("#45475a"), // surface1
	Text:        lipgloss.Color("#cdd6f4"), // text
	Subtext:     lipgloss.Color("#bac2de"), // subtext1
}

// ThemeDracula is a high-contrast dark theme with bright purples, greens, and pinks.
var ThemeDracula = Theme{
	Name:        "dracula",
	DisplayName: "Dracula",
	Description: "high-contrast dark",
	Primary:     lipgloss.Color("#bd93f9"), // purple
	Secondary:   lipgloss.Color("#ff79c6"), // pink
	Accent:      lipgloss.Color("#8be9fd"), // cyan
	Muted:       lipgloss.Color("#6272a4"), // comment
	Success:     lipgloss.Color("#50fa7b"), // green
	Warning:     lipgloss.Color("#f1fa8c"), // yellow
	Danger:      lipgloss.Color("#ff5555"), // red
	Bg:          lipgloss.Color("#282a36"), // background
	Surface:     lipgloss.Color("#44475a"), // current line
	Border:      lipgloss.Color("#6272a4"), // comment
	Highlight:   lipgloss.Color("#44475a"), // current line
	Text:        lipgloss.Color("#f8f8f2"), // foreground
	Subtext:     lipgloss.Color("#cfcfcf"),
}

// ThemeGruvbox is a warm earthy dark theme with orange, yellow, and brown.
var ThemeGruvbox = Theme{
	Name:        "gruvbox",
	DisplayName: "Gruvbox",
	Description: "warm earthy dark",
	Primary:     lipgloss.Color("#83a598"), // aqua
	Secondary:   lipgloss.Color("#d3869b"), // purple
	Accent:      lipgloss.Color("#fabd2f"), // yellow
	Muted:       lipgloss.Color("#928374"), // gray
	Success:     lipgloss.Color("#b8bb26"), // green
	Warning:     lipgloss.Color("#fabd2f"), // yellow
	Danger:      lipgloss.Color("#fb4934"), // red
	Bg:          lipgloss.Color("#282828"), // bg
	Surface:     lipgloss.Color("#3c3836"), // bg1
	Border:      lipgloss.Color("#504945"), // bg2
	Highlight:   lipgloss.Color("#504945"), // bg2
	Text:        lipgloss.Color("#ebdbb2"), // fg
	Subtext:     lipgloss.Color("#d5c4a1"), // fg2
}

// ThemeNord is a cool arctic dark theme with blues and off-whites.
var ThemeNord = Theme{
	Name:        "nord",
	DisplayName: "Nord",
	Description: "cool arctic blues",
	Primary:     lipgloss.Color("#88c0d0"), // nord8
	Secondary:   lipgloss.Color("#81a1c1"), // nord9
	Accent:      lipgloss.Color("#8fbcbb"), // nord7
	Muted:       lipgloss.Color("#4c566a"), // nord3
	Success:     lipgloss.Color("#a3be8c"), // nord14
	Warning:     lipgloss.Color("#ebcb8b"), // nord13
	Danger:      lipgloss.Color("#bf616a"), // nord11
	Bg:          lipgloss.Color("#2e3440"), // nord0
	Surface:     lipgloss.Color("#3b4252"), // nord1
	Border:      lipgloss.Color("#434c5e"), // nord2
	Highlight:   lipgloss.Color("#434c5e"), // nord2
	Text:        lipgloss.Color("#eceff4"), // nord6
	Subtext:     lipgloss.Color("#d8dee9"), // nord4
}

// ThemeSolarizedLight is a classic light theme with warm creams and cool blues.
var ThemeSolarizedLight = Theme{
	Name:        "solarized-light",
	DisplayName: "Solarized Light",
	Description: "warm cream & blues",
	Primary:     lipgloss.Color("#268bd2"), // blue
	Secondary:   lipgloss.Color("#6c71c4"), // violet
	Accent:      lipgloss.Color("#2aa198"), // cyan
	Muted:       lipgloss.Color("#93a1a1"), // base1
	Success:     lipgloss.Color("#859900"), // green
	Warning:     lipgloss.Color("#b58900"), // yellow
	Danger:      lipgloss.Color("#dc322f"), // red
	Bg:          lipgloss.Color("#fdf6e3"), // base3
	Surface:     lipgloss.Color("#eee8d5"), // base2
	Border:      lipgloss.Color("#93a1a1"), // base1
	Highlight:   lipgloss.Color("#eee8d5"), // base2
	Text:        lipgloss.Color("#657b83"), // base00
	Subtext:     lipgloss.Color("#839496"), // base0
}

// AllThemes is the ordered list of built-in themes shown in the theme picker.
var AllThemes = []Theme{
	ThemeTokyoNight,
	ThemeCatppuccinMocha,
	ThemeDracula,
	ThemeGruvbox,
	ThemeNord,
	ThemeSolarizedLight,
}

// ActiveTheme is the currently applied theme. Initialised to Tokyo Night;
// call ApplyTheme to switch at runtime.
var ActiveTheme = &ThemeTokyoNight

// SetThemeByName looks up a theme by its Name field and calls ApplyTheme.
// Unknown names are silently ignored (the current theme stays active).
func SetThemeByName(name string) {
	for i := range AllThemes {
		if AllThemes[i].Name == name {
			ApplyTheme(&AllThemes[i])
			return
		}
	}
}

// ApplyTheme switches the active theme and rebuilds all package-level style
// vars so that every component picks up the new colours on its next View call.
func ApplyTheme(t *Theme) {
	ActiveTheme = t

	// ── Colour vars ───────────────────────────────────────────────────────────
	ColorPrimary = t.Primary
	ColorSecondary = t.Secondary
	ColorAccent = t.Accent
	ColorMuted = t.Muted
	ColorSuccess = t.Success
	ColorWarning = t.Warning
	ColorDanger = t.Danger
	ColorBg = t.Bg
	ColorSurface = t.Surface
	ColorBorder = t.Border
	ColorHighlight = t.Highlight
	ColorText = t.Text
	ColorSubtext = t.Subtext

	// ── Base styles ───────────────────────────────────────────────────────────
	StyleTitle = lipgloss.NewStyle().Foreground(ColorPrimary).Bold(true)
	StyleSubtitle = lipgloss.NewStyle().Foreground(ColorSecondary)
	StyleMuted = lipgloss.NewStyle().Foreground(ColorSubtext)
	StyleDimmed = lipgloss.NewStyle().Foreground(ColorMuted)
	StyleSelected = lipgloss.NewStyle().Background(ColorHighlight).Foreground(ColorText).Bold(true)
	StyleBorder = lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).BorderForeground(ColorBorder)
	StyleSuccess = lipgloss.NewStyle().Foreground(ColorSuccess)
	StyleWarning = lipgloss.NewStyle().Foreground(ColorWarning)
	StyleDanger = lipgloss.NewStyle().Foreground(ColorDanger)
	StyleStatusBar = lipgloss.NewStyle().Background(ColorSurface).Foreground(ColorSubtext)
	StyleStatusBarSep = lipgloss.NewStyle().Foreground(ColorAccent)
	StylePill = lipgloss.NewStyle().Background(ColorHighlight).Foreground(ColorPrimary).Bold(true).PaddingLeft(1).PaddingRight(1)
	StylePanel = lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).BorderForeground(ColorBorder).Padding(0, 1)
	StyleKey = lipgloss.NewStyle().Foreground(ColorAccent).Bold(true)
	StyleFooterBar = lipgloss.NewStyle().Background(ColorSurface).Foreground(ColorMuted)

	// ── Helper styles ─────────────────────────────────────────────────────────
	StyleSpinner = lipgloss.NewStyle().Foreground(ColorPrimary).Background(ColorBg)
	StyleHorizSep = lipgloss.NewStyle().Foreground(ColorBorder).Background(ColorBg)

	// ── Navigator styles ──────────────────────────────────────────────────────
	StyleWordmark = lipgloss.NewStyle().Foreground(ColorPrimary).Background(ColorBg).Bold(true)
	StyleNavRowSelected = lipgloss.NewStyle().Foreground(ColorText).Background(ColorBg).Bold(true)
	StyleNavRowNormal = lipgloss.NewStyle().Foreground(ColorSubtext).Background(ColorBg)
	StyleCursor = lipgloss.NewStyle().Foreground(ColorAccent).Bold(true)

	// ── StatusBar styles ──────────────────────────────────────────────────────
	StyleStatusDimSep = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorSurface)
	StyleStatusActiveCrumb = lipgloss.NewStyle().Foreground(ColorPrimary).Bold(true).Background(ColorSurface)
	StyleStatusHints = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorSurface)

	// ── Modal / overlay styles ────────────────────────────────────────────────
	StyleModalInner = lipgloss.NewStyle().Background(ColorSurface)
	StyleModalBox = lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).BorderForeground(ColorBorder).Background(ColorSurface).Padding(1, 2)
	StylePromptInner = lipgloss.NewStyle().Background(ColorSurface)
	StylePromptBox = lipgloss.NewStyle().Border(lipgloss.RoundedBorder()).BorderForeground(ColorPrimary).Background(ColorSurface).Padding(1, 3)
	StylePromptSep = lipgloss.NewStyle().Foreground(ColorBorder).Background(ColorSurface)
	StyleInputPrompt = lipgloss.NewStyle().Foreground(ColorAccent).Background(ColorSurface)
	StyleInputText = lipgloss.NewStyle().Foreground(ColorText).Background(ColorSurface)
	StyleInputPlaceholder = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorSurface)

	// ── Profile picker styles ─────────────────────────────────────────────────
	StyleProfileCursorPrefix = lipgloss.NewStyle().Foreground(ColorPrimary).Background(ColorSurface).Bold(true)
	StyleProfileSelectedName = lipgloss.NewStyle().Foreground(ColorText).Background(ColorSurface).Bold(true)

	// ── Table list styles ─────────────────────────────────────────────────────
	StyleCount = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorBg)
	StyleFilterActive = lipgloss.NewStyle().Foreground(ColorText).Background(ColorBg)
	StyleTableSep = lipgloss.NewStyle().Foreground(ColorBorder).Background(ColorBg)
	StyleListRowSelected = lipgloss.NewStyle().Background(ColorHighlight)
	StyleListRowSelectedCursor = lipgloss.NewStyle().Foreground(ColorAccent).Bold(true).Background(ColorHighlight)
	StyleListRowSelectedName = lipgloss.NewStyle().Foreground(ColorText).Bold(true).Background(ColorHighlight)
	StyleListRowNormal = lipgloss.NewStyle().Background(ColorBg)
	StyleListRowNormalName = lipgloss.NewStyle().Foreground(ColorSubtext).Background(ColorBg)
	StyleEmptyState = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorBg)

	// ── Items model styles ────────────────────────────────────────────────────
	StyleModeBadgeQuery = lipgloss.NewStyle().Foreground(ColorBg).Background(ColorPrimary).PaddingLeft(1).PaddingRight(1)
	StyleModeBadgeScan = lipgloss.NewStyle().Foreground(ColorBg).Background(ColorMuted).PaddingLeft(1).PaddingRight(1)
	StyleKeyLabel = lipgloss.NewStyle().Foreground(ColorAccent).Background(ColorBg)
	StyleValueLabel = lipgloss.NewStyle().Foreground(ColorText).Background(ColorBg)
	StyleFilterLabel = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorBg)
	StyleFilterValue = lipgloss.NewStyle().Foreground(ColorSubtext).Background(ColorBg)
	StylePageIndicator = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorBg)
	StyleItemsHeader = lipgloss.NewStyle().PaddingLeft(2).Background(ColorBg)

	// ── Detail view styles ────────────────────────────────────────────────────
	StyleViewportBorder = lipgloss.NewStyle().
		BorderStyle(lipgloss.NormalBorder()).
		BorderForeground(ColorBorder).
		Background(ColorBg)

	// ── DynamoDB table widget styles ──────────────────────────────────────────
	DynamoTableStyles = buildDynamoTableStyles()
}

// buildDynamoTableStyles constructs a bubbles/table.Styles value from the
// current colour vars. Called by ApplyTheme whenever the theme changes.
func buildDynamoTableStyles() table.Styles {
	s := table.DefaultStyles()
	s.Header = s.Header.
		BorderStyle(lipgloss.NormalBorder()).
		BorderForeground(ColorBorder).
		BorderBottom(true).
		Foreground(ColorMuted).
		Background(ColorSurface).
		Bold(false)
	s.Selected = s.Selected.
		Foreground(ColorText).
		Background(ColorHighlight).
		Bold(true)
	s.Cell = s.Cell.
		Foreground(ColorSubtext).
		Background(ColorBg)
	return s
}

// ── Colour vars ───────────────────────────────────────────────────────────────
// These are initialised from ThemeTokyoNight below and updated by ApplyTheme.

// Colour palette — initialised to Tokyo Night.
var (
	ColorPrimary   = ThemeTokyoNight.Primary
	ColorSecondary = ThemeTokyoNight.Secondary
	ColorAccent    = ThemeTokyoNight.Accent
	ColorMuted     = ThemeTokyoNight.Muted
	ColorSuccess   = ThemeTokyoNight.Success
	ColorWarning   = ThemeTokyoNight.Warning
	ColorDanger    = ThemeTokyoNight.Danger
	ColorBg        = ThemeTokyoNight.Bg
	ColorSurface   = ThemeTokyoNight.Surface
	ColorBorder    = ThemeTokyoNight.Border
	ColorHighlight = ThemeTokyoNight.Highlight
	ColorText      = ThemeTokyoNight.Text
	ColorSubtext   = ThemeTokyoNight.Subtext
)

// ── Style vars ────────────────────────────────────────────────────────────────
// All styles are initialised to match ThemeTokyoNight. Call ApplyTheme to
// rebuild them with a different palette.

// Base styles.
var (
	StyleTitle = lipgloss.NewStyle().
			Foreground(ColorPrimary).
			Bold(true)

	StyleSubtitle = lipgloss.NewStyle().
			Foreground(ColorSecondary)

	StyleMuted = lipgloss.NewStyle().
			Foreground(ColorSubtext)

	StyleDimmed = lipgloss.NewStyle().
			Foreground(ColorMuted)

	StyleSelected = lipgloss.NewStyle().
			Background(ColorHighlight).
			Foreground(ColorText).
			Bold(true)

	StyleBorder = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(ColorBorder)

	StyleSuccess = lipgloss.NewStyle().Foreground(ColorSuccess)
	StyleWarning = lipgloss.NewStyle().Foreground(ColorWarning)
	StyleDanger  = lipgloss.NewStyle().Foreground(ColorDanger)

	// Status bar — rendered as a slim bottom strip.
	StyleStatusBar = lipgloss.NewStyle().
			Background(ColorSurface).
			Foreground(ColorSubtext)

	StyleStatusBarSep = lipgloss.NewStyle().
				Foreground(ColorAccent)

	// Pill — small rounded badge used for profile name.
	StylePill = lipgloss.NewStyle().
			Background(ColorHighlight).
			Foreground(ColorPrimary).
			Bold(true).
			PaddingLeft(1).
			PaddingRight(1)

	// Panel — generic bordered content box.
	StylePanel = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(ColorBorder).
			Padding(0, 1)

	// Key hint used in help rows and footer bars.
	StyleKey = lipgloss.NewStyle().
			Foreground(ColorAccent).
			Bold(true)

	// Footer bar — surface-background hints strip at bottom of views.
	StyleFooterBar = lipgloss.NewStyle().
			Background(ColorSurface).
			Foreground(ColorMuted)
)

// Helper styles — used in helpers.go.
var (
	// StyleSpinner is applied to the spinner widget.
	StyleSpinner = lipgloss.NewStyle().Foreground(ColorPrimary).Background(ColorBg)

	// StyleHorizSep is the horizontal rule between sections.
	StyleHorizSep = lipgloss.NewStyle().Foreground(ColorBorder).Background(ColorBg)
)

// Navigator styles.
var (
	// StyleWordmark is the "cumulus" brand text in the navigator.
	StyleWordmark = lipgloss.NewStyle().Foreground(ColorPrimary).Background(ColorBg).Bold(true)

	// StyleNavRowSelected is the name text of the currently selected service.
	StyleNavRowSelected = lipgloss.NewStyle().Foreground(ColorText).Background(ColorBg).Bold(true)

	// StyleNavRowNormal is the name text of an unselected service.
	StyleNavRowNormal = lipgloss.NewStyle().Foreground(ColorSubtext).Background(ColorBg)

	// StyleCursor is the arrow indicator for the selected row (› character).
	StyleCursor = lipgloss.NewStyle().Foreground(ColorAccent).Bold(true)
)

// StatusBar styles.
var (
	// StyleStatusDimSep is the dim › separator between breadcrumb segments.
	StyleStatusDimSep = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorSurface)

	// StyleStatusActiveCrumb is the last (active) breadcrumb segment.
	StyleStatusActiveCrumb = lipgloss.NewStyle().Foreground(ColorPrimary).Bold(true).Background(ColorSurface)

	// StyleStatusHints is the right-side keyboard hint text in the status bar.
	StyleStatusHints = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorSurface)
)

// Modal / overlay styles.
var (
	// StyleModalInner wraps the inner content of help/profile-picker modals.
	StyleModalInner = lipgloss.NewStyle().Background(ColorSurface)

	// StyleModalBox is the outer rounded-border container for informational modals.
	StyleModalBox = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(ColorBorder).
			Background(ColorSurface).
			Padding(1, 2)

	// StylePromptInner wraps the inner content of the prompt modal.
	StylePromptInner = lipgloss.NewStyle().Background(ColorSurface)

	// StylePromptBox is the outer rounded-border container for interactive prompts.
	StylePromptBox = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(ColorPrimary).
			Background(ColorSurface).
			Padding(1, 3)

	// StylePromptSep is the separator line inside the prompt modal.
	StylePromptSep = lipgloss.NewStyle().Foreground(ColorBorder).Background(ColorSurface)

	// StyleInputPrompt is the prompt arrow style for the text input.
	StyleInputPrompt = lipgloss.NewStyle().Foreground(ColorAccent).Background(ColorSurface)

	// StyleInputText is the typed-text style for the text input.
	StyleInputText = lipgloss.NewStyle().Foreground(ColorText).Background(ColorSurface)

	// StyleInputPlaceholder is the placeholder style for the text input.
	StyleInputPlaceholder = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorSurface)
)

// Profile picker styles.
var (
	// StyleProfileCursorPrefix is "  › " rendered for the selected profile row.
	StyleProfileCursorPrefix = lipgloss.NewStyle().Foreground(ColorPrimary).Background(ColorSurface).Bold(true)

	// StyleProfileSelectedName is the selected profile name text.
	StyleProfileSelectedName = lipgloss.NewStyle().Foreground(ColorText).Background(ColorSurface).Bold(true)
)

// Table list styles (tables.go — full-width hand-rolled list).
var (
	// StyleCount is the table-count indicator in the header.
	StyleCount = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorBg)

	// StyleFilterActive is the filter text displayed after the cursor character.
	StyleFilterActive = lipgloss.NewStyle().Foreground(ColorText).Background(ColorBg)

	// StyleTableSep is the full-width separator line under the header.
	StyleTableSep = lipgloss.NewStyle().Foreground(ColorBorder).Background(ColorBg)

	// StyleListRowSelected is the full-width highlighted row.
	StyleListRowSelected = lipgloss.NewStyle().Background(ColorHighlight)

	// StyleListRowSelectedCursor is "›" inside a selected row.
	StyleListRowSelectedCursor = lipgloss.NewStyle().Foreground(ColorAccent).Bold(true).Background(ColorHighlight)

	// StyleListRowSelectedName is the table name inside a selected row.
	StyleListRowSelectedName = lipgloss.NewStyle().Foreground(ColorText).Bold(true).Background(ColorHighlight)

	// StyleListRowNormal is the full-width normal (unselected) row.
	StyleListRowNormal = lipgloss.NewStyle().Background(ColorBg)

	// StyleListRowNormalName is the table name inside a normal row.
	StyleListRowNormalName = lipgloss.NewStyle().Foreground(ColorSubtext).Background(ColorBg)

	// StyleEmptyState is used for "no tables found" / "no items" messages.
	StyleEmptyState = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorBg)
)

// Items model styles (items_model.go).
var (
	// StyleModeBadgeQuery is the "query" badge in the items header.
	StyleModeBadgeQuery = lipgloss.NewStyle().
				Foreground(ColorBg).Background(ColorPrimary).
				PaddingLeft(1).PaddingRight(1)

	// StyleModeBadgeScan is the "scan" badge in the items header.
	StyleModeBadgeScan = lipgloss.NewStyle().
				Foreground(ColorBg).Background(ColorMuted).
				PaddingLeft(1).PaddingRight(1)

	// StyleKeyLabel is used for "pk=" / "sk=" labels in the query display.
	StyleKeyLabel = lipgloss.NewStyle().Foreground(ColorAccent).Background(ColorBg)

	// StyleValueLabel is used for key values in the query display.
	StyleValueLabel = lipgloss.NewStyle().Foreground(ColorText).Background(ColorBg)

	// StyleFilterLabel is used for "  filter: " in the items header.
	StyleFilterLabel = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorBg)

	// StyleFilterValue is the filter expression text in the items header.
	StyleFilterValue = lipgloss.NewStyle().Foreground(ColorSubtext).Background(ColorBg)

	// StylePageIndicator is the "page N" text in the items header.
	StylePageIndicator = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorBg)

	// StyleItemsHeader wraps the entire items header line.
	StyleItemsHeader = lipgloss.NewStyle().PaddingLeft(2).Background(ColorBg)
)

// Detail view styles.
var (
	// StyleViewportBorder is the style applied to the detail viewport.
	StyleViewportBorder = lipgloss.NewStyle().
		BorderStyle(lipgloss.NormalBorder()).
		BorderForeground(ColorBorder).
		Background(ColorBg)
)

// DynamoTableStyles is the bubbles/table style set for the items table widget.
// It is rebuilt by ApplyTheme whenever the theme changes.
var DynamoTableStyles = buildDynamoTableStyles()

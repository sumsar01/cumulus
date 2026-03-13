// Package ui provides shared lipgloss styles and theme constants used across
// all views in cumulus.
package ui

import "github.com/charmbracelet/lipgloss"

// Colour palette — Tokyo Night dark theme.
var (
	ColorPrimary   = lipgloss.Color("#7aa2f7") // blue
	ColorSecondary = lipgloss.Color("#bb9af7") // purple
	ColorAccent    = lipgloss.Color("#7dcfff") // cyan
	ColorMuted     = lipgloss.Color("#565f89") // dim purple-gray
	ColorSuccess   = lipgloss.Color("#9ece6a") // green
	ColorWarning   = lipgloss.Color("#e0af68") // amber
	ColorDanger    = lipgloss.Color("#f7768e") // red/pink
	ColorBg        = lipgloss.Color("#1a1b26") // deep navy background
	ColorSurface   = lipgloss.Color("#24283b") // slightly lighter surface
	ColorBorder    = lipgloss.Color("#414868") // visible border
	ColorHighlight = lipgloss.Color("#283457") // selection background
	ColorText      = lipgloss.Color("#c0caf5") // primary text
	ColorDimText   = lipgloss.Color("#3b4261") // very dim text
	ColorSubtext   = lipgloss.Color("#a9b1d6") // secondary text
)

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

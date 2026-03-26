package ui

import (
	"strings"

	"github.com/charmbracelet/bubbles/spinner"
)

// MaxContentWidth is the maximum width (in columns) that any content pane will
// occupy. On terminals wider than this the content is centered with background
// padding on both sides.
const MaxContentWidth = 120

// EffectiveWidth returns the content width to use for layout given the actual
// terminal width. It caps at MaxContentWidth so that wide terminals don't
// stretch content uncomfortably across the full screen.
func EffectiveWidth(termWidth int) int {
	if termWidth < MaxContentWidth {
		return termWidth
	}
	return MaxContentWidth
}

// NewSpinner returns a spinner pre-configured with the cumulus theme.
func NewSpinner() spinner.Model {
	sp := spinner.New()
	sp.Spinner = spinner.Dot
	sp.Style = StyleSpinner
	return sp
}

// HorizontalSep returns a full-width horizontal rule for use between sections.
// width is the total terminal width.
func HorizontalSep(width int) string {
	if width < 1 {
		width = 40
	}
	return StyleHorizSep.Width(width).Render(strings.Repeat("─", width))
}

// RenderHints builds the footer key-hint bar from a slice of [key, description]
// pairs. width is used to set the full bar width.
func RenderHints(width int, pairs [][2]string) string {
	var hintParts []string
	for _, p := range pairs {
		k := StyleKey.Background(ColorSurface).Render(p[0])
		v := StyleFooterBar.Render(" " + p[1])
		hintParts = append(hintParts, k+v)
	}
	return StyleFooterBar.Width(width).Render(
		"  " + strings.Join(hintParts, StyleFooterBar.Foreground(ColorBorder).Render("   ")),
	)
}

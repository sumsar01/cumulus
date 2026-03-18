package ui

import (
	"strings"

	"github.com/charmbracelet/bubbles/spinner"
)

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

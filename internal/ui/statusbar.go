package ui

import (
	"strings"

	"github.com/charmbracelet/lipgloss"
)

// StatusBar holds the state rendered in the persistent bottom bar.
type StatusBar struct {
	Profile    string
	Region     string
	Breadcrumb []string
	Width      int
	Err        string
	Status     string // informational (non-error) message, rendered in green
}

// View renders the status bar. Errors appear on a dedicated second line so
// the full message is always visible.
func (s StatusBar) View() string {
	surfaceStyle := lipgloss.NewStyle().Background(ColorSurface)
	accentSep := StyleStatusBarSep.Background(ColorSurface).Render("  ›  ")
	dimSep := StyleStatusDimSep.Render("  ›  ")

	// Left side: profile pill › region › crumbs
	// The active (last) crumb is primary blue + bold; earlier crumbs are subtext.
	left := surfaceStyle.Render(" ") + StylePill.Render(s.Profile)

	if s.Region != "" {
		left += accentSep + StyleStatusBar.Foreground(ColorSubtext).Background(ColorSurface).Render(s.Region)
	}

	for i, crumb := range s.Breadcrumb {
		if i == len(s.Breadcrumb)-1 {
			// Active crumb — primary blue, bold
			left += dimSep + StyleStatusActiveCrumb.Render(crumb)
		} else {
			left += dimSep + StyleStatusBar.Foreground(ColorSubtext).Background(ColorSurface).Render(crumb)
		}
	}

	// Right side: hints (hidden when there is an error/status on the line below)
	right := ""
	if s.Err == "" && s.Status == "" {
		right = StyleStatusHints.Render("p  profile   t  theme   ?  help   ctrl+c  quit ")
	}

	gap := s.Width - lipgloss.Width(left) - lipgloss.Width(right)
	if gap < 1 {
		gap = 1
	}

	// Style the gap explicitly so the background is set and no terminal-default
	// black chars appear between the pre-rendered ANSI segments.
	gapStr := surfaceStyle.Render(strings.Repeat(" ", gap))

	main := lipgloss.JoinHorizontal(lipgloss.Top, left, gapStr, right)

	if s.Err != "" {
		errLine := StyleDanger.
			Background(ColorBg).
			Width(s.Width).
			Render("  " + s.Err)
		return main + "\n" + errLine
	}

	if s.Status != "" {
		statusLine := StyleSuccess.
			Background(ColorBg).
			Width(s.Width).
			Render("  " + s.Status)
		return main + "\n" + statusLine
	}

	blank := StyleStatusBar.Background(ColorSurface).Width(s.Width).Render("")
	return main + "\n" + blank
}

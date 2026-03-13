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
	accentSep := StyleStatusBarSep.Background(ColorSurface).Render("  ›  ")
	dimSep := lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorSurface).Render("  ›  ")

	// Left side: profile pill › region › crumbs
	// The active (last) crumb is primary blue + bold; earlier crumbs are subtext.
	left := " " + StylePill.Render(s.Profile)

	if s.Region != "" {
		left += accentSep + StyleStatusBar.Foreground(ColorSubtext).Render(s.Region)
	}

	for i, crumb := range s.Breadcrumb {
		if i == len(s.Breadcrumb)-1 {
			// Active crumb — primary blue, bold
			left += dimSep + lipgloss.NewStyle().
				Foreground(ColorPrimary).Bold(true).
				Background(ColorSurface).Render(crumb)
		} else {
			left += dimSep + StyleStatusBar.Foreground(ColorSubtext).Render(crumb)
		}
	}

	// Right side: hints (hidden when there is an error/status on the line below)
	right := ""
	if s.Err == "" && s.Status == "" {
		right = lipgloss.NewStyle().Foreground(ColorMuted).Background(ColorSurface).Render("p  profile   ?  help   q  quit ")
	}

	gap := s.Width - lipgloss.Width(left) - lipgloss.Width(right) - 0
	if gap < 1 {
		gap = 1
	}

	bar := left + strings.Repeat(" ", gap) + right
	main := StyleStatusBar.
		Background(ColorSurface).
		Width(s.Width).
		Render(bar)

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

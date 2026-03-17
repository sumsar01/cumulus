// Package ui provides shared lipgloss styles and theme constants used across
// all views in cumulus.
package ui

import (
	"github.com/charmbracelet/huh"
	"github.com/charmbracelet/lipgloss"
)

// CumulusHuhTheme returns a huh.Theme that matches the Tokyo Night dark palette
// used throughout the rest of the app. All huh fields and forms should call
// .WithTheme(ui.CumulusHuhTheme()) to stay visually consistent.
func CumulusHuhTheme() *huh.Theme {
	t := huh.ThemeBase()

	// ── Form container ────────────────────────────────────────────────────────
	t.Form.Base = lipgloss.NewStyle().Background(ColorSurface)

	// ── Group ─────────────────────────────────────────────────────────────────
	t.Group.Base = lipgloss.NewStyle().Background(ColorSurface)
	t.Group.Title = lipgloss.NewStyle().
		Foreground(ColorPrimary).
		Background(ColorSurface).
		Bold(true)
	t.Group.Description = lipgloss.NewStyle().
		Foreground(ColorMuted).
		Background(ColorSurface)

	// ── Focused field ─────────────────────────────────────────────────────────
	t.Focused.Base = lipgloss.NewStyle().
		Background(ColorSurface)

	t.Focused.Title = lipgloss.NewStyle().
		Foreground(ColorPrimary).
		Background(ColorSurface).
		Bold(true)

	t.Focused.Description = lipgloss.NewStyle().
		Foreground(ColorMuted).
		Background(ColorSurface)

	t.Focused.ErrorIndicator = lipgloss.NewStyle().
		Foreground(ColorDanger).
		Background(ColorSurface)

	t.Focused.ErrorMessage = lipgloss.NewStyle().
		Foreground(ColorDanger).
		Background(ColorSurface)

	t.Focused.SelectSelector = lipgloss.NewStyle().
		Foreground(ColorAccent).
		Background(ColorSurface).
		Bold(true)

	t.Focused.Option = lipgloss.NewStyle().
		Foreground(ColorSubtext).
		Background(ColorSurface)

	t.Focused.SelectedOption = lipgloss.NewStyle().
		Foreground(ColorText).
		Background(ColorHighlight).
		Bold(true)

	t.Focused.SelectedPrefix = lipgloss.NewStyle().
		Foreground(ColorAccent).
		Background(ColorSurface)

	t.Focused.UnselectedPrefix = lipgloss.NewStyle().
		Foreground(ColorMuted).
		Background(ColorSurface)

	t.Focused.UnselectedOption = lipgloss.NewStyle().
		Foreground(ColorSubtext).
		Background(ColorSurface)

	t.Focused.FocusedButton = lipgloss.NewStyle().
		Foreground(ColorBg).
		Background(ColorPrimary).
		Bold(true).
		PaddingLeft(2).PaddingRight(2)

	t.Focused.BlurredButton = lipgloss.NewStyle().
		Foreground(ColorMuted).
		Background(ColorSurface).
		PaddingLeft(2).PaddingRight(2)

	t.Focused.TextInput.Cursor = lipgloss.NewStyle().
		Foreground(ColorAccent).
		Background(ColorSurface)

	t.Focused.TextInput.Placeholder = lipgloss.NewStyle().
		Foreground(ColorMuted).
		Background(ColorSurface)

	t.Focused.TextInput.Prompt = lipgloss.NewStyle().
		Foreground(ColorAccent).
		Background(ColorSurface)

	t.Focused.TextInput.Text = lipgloss.NewStyle().
		Foreground(ColorText).
		Background(ColorSurface)

	// ── Blurred field ─────────────────────────────────────────────────────────
	t.Blurred.Base = lipgloss.NewStyle().
		Background(ColorSurface)

	t.Blurred.Title = lipgloss.NewStyle().
		Foreground(ColorMuted).
		Background(ColorSurface)

	t.Blurred.Description = lipgloss.NewStyle().
		Foreground(ColorMuted).
		Background(ColorSurface)

	t.Blurred.TextInput.Prompt = lipgloss.NewStyle().
		Foreground(ColorMuted).
		Background(ColorSurface)

	t.Blurred.TextInput.Text = lipgloss.NewStyle().
		Foreground(ColorSubtext).
		Background(ColorSurface)

	t.Blurred.TextInput.Placeholder = lipgloss.NewStyle().
		Foreground(ColorMuted).
		Background(ColorSurface)

	t.Blurred.SelectSelector = lipgloss.NewStyle().
		Foreground(ColorMuted).
		Background(ColorSurface)

	t.Blurred.Option = lipgloss.NewStyle().
		Foreground(ColorMuted).
		Background(ColorSurface)

	t.Blurred.FocusedButton = lipgloss.NewStyle().
		Foreground(ColorMuted).
		Background(ColorSurface).
		PaddingLeft(2).PaddingRight(2)

	t.Blurred.BlurredButton = lipgloss.NewStyle().
		Foreground(ColorMuted).
		Background(ColorSurface).
		PaddingLeft(2).PaddingRight(2)

	return t
}

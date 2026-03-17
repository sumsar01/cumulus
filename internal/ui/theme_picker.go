package ui

import (
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
)

// ThemeChangedMsg is emitted when the user confirms a new theme selection.
// App.Update persists the choice to config and closes the overlay.
type ThemeChangedMsg struct {
	// Name is the machine-readable theme name (e.g. "catppuccin-mocha").
	Name string
}

// ThemePicker is a modal overlay that lists available colour themes and
// provides live preview: switching the cursor changes the active theme
// immediately; Esc restores the previous theme.
type ThemePicker struct {
	cursor   int
	previous *Theme // theme active when the picker was opened; restored on Esc
}

// NewThemePicker constructs a ThemePicker pre-selected on the current theme.
func NewThemePicker() ThemePicker {
	cursor := 0
	for i, t := range AllThemes {
		if t.Name == ActiveTheme.Name {
			cursor = i
			break
		}
	}
	// Take a copy of the current theme so we can restore it on cancel.
	prev := *ActiveTheme
	return ThemePicker{cursor: cursor, previous: &prev}
}

// Update handles key events for the theme picker.
// Returns (picker, cmd, done). done=true means the overlay should close.
func (p *ThemePicker) Update(msg tea.KeyMsg) (*ThemePicker, tea.Cmd, bool) {
	switch msg.String() {
	case "esc":
		// Restore the theme that was active when the picker opened.
		ApplyTheme(p.previous)
		return nil, nil, true

	case "up", "k":
		if p.cursor > 0 {
			p.cursor--
			ApplyTheme(&AllThemes[p.cursor])
		}

	case "down", "j":
		if p.cursor < len(AllThemes)-1 {
			p.cursor++
			ApplyTheme(&AllThemes[p.cursor])
		}

	case "enter":
		chosen := AllThemes[p.cursor]
		return nil, func() tea.Msg {
			return ThemeChangedMsg{Name: chosen.Name}
		}, true
	}

	return p, nil, false
}

// View renders the theme picker as a centred overlay box.
func (p *ThemePicker) View(width, height int) string {
	bg := ColorSurface

	title := StyleTitle.Background(bg).Render("select theme")
	hint := StyleDimmed.Background(bg).Render("↑/↓  preview   enter  apply   esc  cancel")
	sep := StyleDimmed.Background(bg).Render(strings.Repeat("─", 46))

	var rows strings.Builder
	for i, t := range AllThemes {
		active := ""
		if t.Name == p.previous.Name {
			active = StyleDimmed.Background(bg).Render(" ·")
		}
		desc := StyleDimmed.Background(bg).Render("  " + t.Description)
		if i == p.cursor {
			rows.WriteString(
				StyleProfileCursorPrefix.Background(bg).Render("  › ") +
					StyleProfileSelectedName.Background(bg).Render(t.DisplayName) +
					desc + active + "\n",
			)
		} else {
			rows.WriteString(
				StyleDimmed.Background(bg).Render("    ") +
					StyleMuted.Background(bg).Render(t.DisplayName) +
					desc + active + "\n",
			)
		}
	}

	content := title + "\n" +
		sep + "\n" +
		rows.String() + "\n" +
		hint

	inner := StyleModalInner.Background(bg).Width(50).Render(content)
	box := StyleModalBox.Background(bg).Padding(1, 2).Render(inner)

	return lipgloss.Place(width, height,
		lipgloss.Center, lipgloss.Center,
		box,
		lipgloss.WithWhitespaceBackground(ColorBg),
	)
}

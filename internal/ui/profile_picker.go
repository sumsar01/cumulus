package ui

import (
	"fmt"
	"strings"

	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
)

// ProfilePicker is a modal overlay that lists AWS profiles.
type ProfilePicker struct {
	profiles []string
	cursor   int
	current  string // currently active profile
	loading  bool
}

// NewProfilePicker constructs a ProfilePicker and starts loading profiles.
func NewProfilePicker(currentProfile string) (ProfilePicker, tea.Cmd) {
	return ProfilePicker{loading: true, current: currentProfile}, loadProfilesCmd()
}

type profilesLoadedMsg struct{ profiles []string }

func loadProfilesCmd() tea.Cmd {
	return func() tea.Msg {
		profiles, err := awspkg.ListProfiles()
		if err != nil {
			return awspkg.ErrMsg{Err: err}
		}
		return profilesLoadedMsg{profiles: profiles}
	}
}

// Update handles key events for the profile picker.
// Returns (picker, cmd, done). done=true means the overlay should close.
func (p *ProfilePicker) Update(msg tea.KeyMsg) (*ProfilePicker, tea.Cmd, bool) {
	switch msg.String() {
	case "esc":
		return nil, nil, true
	case "up", "k":
		if p.cursor > 0 {
			p.cursor--
		}
	case "down", "j":
		if p.cursor < len(p.profiles)-1 {
			p.cursor++
		}
	case "enter":
		if len(p.profiles) == 0 {
			return p, nil, false
		}
		chosen := p.profiles[p.cursor]
		return nil, awspkg.LoadProfileCmd(chosen), true
	}
	return p, nil, false
}

// HandleMsg handles non-key messages (e.g. profiles loaded).
func (p *ProfilePicker) HandleMsg(msg tea.Msg) (*ProfilePicker, tea.Cmd) {
	switch m := msg.(type) {
	case profilesLoadedMsg:
		p.profiles = m.profiles
		p.loading = false
		// Pre-select the currently active profile.
		for i, pr := range m.profiles {
			if pr == p.current {
				p.cursor = i
				break
			}
		}
	}
	return p, nil
}

// View renders the profile picker as a centred overlay box.
func (p *ProfilePicker) View(width, height int) string {
	bg := ColorSurface

	title := StyleTitle.Background(bg).Render("switch profile")
	hint := StyleDimmed.Background(bg).Render("↑/↓  navigate   enter  select   esc  cancel")
	sep := StyleDimmed.Background(bg).Render(strings.Repeat("─", 40))

	var body string
	if p.loading {
		body = StyleMuted.Background(bg).Render("  loading profiles…")
	} else if len(p.profiles) == 0 {
		body = StyleDimmed.Background(bg).Render("  no profiles found")
	} else {
		var rows strings.Builder
		for i, pr := range p.profiles {
			active := ""
			if pr == p.current {
				active = StyleDimmed.Background(bg).Render(" ·")
			}
			if i == p.cursor {
				rows.WriteString(
					StyleProfileCursorPrefix.Background(bg).Render("  › ") +
						StyleProfileSelectedName.Background(bg).Render(pr) +
						active + "\n",
				)
			} else {
				rows.WriteString(
					StyleDimmed.Background(bg).Render("    ") +
						StyleMuted.Background(bg).Render(pr) +
						active + "\n",
				)
			}
		}
		body = rows.String()
	}

	count := fmt.Sprintf("%d profiles", len(p.profiles))
	if p.loading {
		count = ""
	}

	content := title + "  " + StyleDimmed.Background(bg).Render(count) + "\n" +
		sep + "\n" +
		body + "\n" +
		hint

	inner := StyleModalInner.Background(bg).Width(46).Render(content)

	box := StyleModalBox.Background(bg).Padding(1, 2).Render(inner)

	return lipgloss.Place(width, height,
		lipgloss.Center, lipgloss.Center,
		box,
		lipgloss.WithWhitespaceBackground(ColorBg),
	)
}

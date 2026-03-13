package ui

import (
	"fmt"

	"github.com/aws/aws-sdk-go-v2/aws"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/sumsar01/cumulus/internal/services"
)

// Navigator is the home screen: a hand-rolled service menu.
type Navigator struct {
	services []services.Service
	svcMap   map[string]services.Service
	cfg      aws.Config
	cursor   int
	width    int
	height   int
}

// NewNavigator builds the home-screen service picker.
func NewNavigator(cfg aws.Config) Navigator {
	svcs := services.All()
	svcMap := make(map[string]services.Service, len(svcs))
	for _, s := range svcs {
		svcMap[s.ShortName()] = s
	}
	return Navigator{
		services: svcs,
		svcMap:   svcMap,
		cfg:      cfg,
	}
}

func (n Navigator) Init() tea.Cmd { return nil }

func (n Navigator) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		n.width = msg.Width
		n.height = msg.Height

	case tea.KeyMsg:
		switch msg.String() {
		case "up", "k":
			if n.cursor > 0 {
				n.cursor--
			}
		case "down", "j":
			if n.cursor < len(n.services)-1 {
				n.cursor++
			}
		case "enter":
			if len(n.services) == 0 {
				return n, nil
			}
			svc := n.services[n.cursor]
			model, cmd := svc.Init(n.cfg)
			return n, tea.Batch(
				cmd,
				func() tea.Msg { return PushMsg{Model: model} },
				func() tea.Msg {
					return SetBreadcrumbMsg{Crumbs: []string{svc.Name()}}
				},
			)
		}
	}
	return n, nil
}

func (n Navigator) View() string {
	if n.width == 0 {
		return ""
	}

	// Wordmark
	wordmark := lipgloss.NewStyle().
		Foreground(ColorPrimary).
		Bold(true).
		Render("cumulus")
	tagline := StyleMuted.Render("AWS in your terminal")

	header := lipgloss.JoinVertical(lipgloss.Center, wordmark, tagline)

	// Service list
	var rows string
	for i, svc := range n.services {
		icon := svc.Icon()
		name := svc.Name()
		desc := svc.Description()

		var cursor, nameStyle, descStyle string
		if i == n.cursor {
			cursor = StyleKey.Render("›")
			nameStyle = lipgloss.NewStyle().Foreground(ColorText).Bold(true).Render(name)
			descStyle = StyleMuted.Render(desc)
		} else {
			cursor = "  "
			nameStyle = lipgloss.NewStyle().Foreground(ColorSubtext).Render(name)
			descStyle = StyleDimmed.Render(desc)
		}

		row := fmt.Sprintf("%s %s  %s  %s", cursor, icon, nameStyle, descStyle)
		rows += row + "\n"
	}

	hint := StyleDimmed.Render("↑/↓  navigate   enter  select   p  switch profile   ?  help   q  quit")

	inner := lipgloss.JoinVertical(lipgloss.Left,
		header,
		"",
		rows,
		hint,
	)

	box := StylePanel.
		Width(64).
		Render(inner)

	return lipgloss.Place(
		n.width, n.height,
		lipgloss.Center, lipgloss.Center,
		box,
		lipgloss.WithWhitespaceBackground(ColorBg),
	)
}

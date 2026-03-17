package ui

import (
	"io"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/charmbracelet/bubbles/list"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/sumsar01/cumulus/internal/services"
)

// ── list item ─────────────────────────────────────────────────────────────────

// listItem wraps a Service for use with bubbles/list.
type listItem struct{ svc services.Service }

func (i listItem) Title() string       { return i.svc.Icon() + "  " + i.svc.Name() }
func (i listItem) Description() string { return i.svc.Description() }
func (i listItem) FilterValue() string { return i.svc.Name() }

// ── custom delegate ───────────────────────────────────────────────────────────

// navDelegate renders each service row using the Tokyo Night navigator styles.
type navDelegate struct{}

func (d navDelegate) Height() int                             { return 2 }
func (d navDelegate) Spacing() int                            { return 0 }
func (d navDelegate) Update(_ tea.Msg, _ *list.Model) tea.Cmd { return nil }

func (d navDelegate) Render(w io.Writer, m list.Model, index int, raw list.Item) {
	item, ok := raw.(listItem)
	if !ok {
		return
	}

	bg := ColorBg

	iconAndName := item.svc.Icon() + "  " + item.svc.Name()
	desc := item.svc.Description()

	var cursor, nameStr, descStr string
	if index == m.Index() {
		cursor = StyleCursor.Background(bg).Render("›")
		nameStr = StyleNavRowSelected.Background(bg).Render(iconAndName)
		descStr = StyleMuted.Background(bg).Render(desc)
	} else {
		cursor = StyleDimmed.Background(bg).Render(" ")
		nameStr = StyleNavRowNormal.Background(bg).Render(iconAndName)
		descStr = StyleDimmed.Background(bg).Render(desc)
	}

	sp1 := lipgloss.NewStyle().Background(bg).Render(" ")
	sp2 := lipgloss.NewStyle().Background(bg).Render("  ")
	nameLine := lipgloss.JoinHorizontal(lipgloss.Top, cursor, sp1, nameStr)
	descLine := lipgloss.JoinHorizontal(lipgloss.Top, sp2, sp1, descStr)

	row := lipgloss.JoinVertical(lipgloss.Left, nameLine, descLine)
	_, _ = io.WriteString(w, row)
}

// ── Navigator ─────────────────────────────────────────────────────────────────

// Navigator is the home screen: a bubbles/list-based service menu.
type Navigator struct {
	list   list.Model
	svcMap map[string]services.Service
	cfg    aws.Config
	width  int
	height int
}

// NewNavigator builds the home-screen service picker.
func NewNavigator(cfg aws.Config) Navigator {
	svcs := services.All()
	svcMap := make(map[string]services.Service, len(svcs))
	items := make([]list.Item, len(svcs))
	for i, s := range svcs {
		svcMap[s.ShortName()] = s
		items[i] = listItem{svc: s}
	}

	l := list.New(items, navDelegate{}, 0, 0)
	l.SetShowTitle(false)
	l.SetShowStatusBar(false)
	l.SetShowHelp(false)
	l.SetFilteringEnabled(true)
	l.Styles.NoItems = StyleDimmed

	return Navigator{
		list:   l,
		svcMap: svcMap,
		cfg:    cfg,
	}
}

func (n Navigator) Init() tea.Cmd { return nil }

// IsTextInputActive implements ui.TextInputActive. Returns true while the
// list filter is active so that app.go passes all keys straight through.
func (n Navigator) IsTextInputActive() bool {
	return n.list.FilterState() == list.Filtering
}

func (n Navigator) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {
	case tea.WindowSizeMsg:
		n.width = msg.Width
		n.height = msg.Height

	case tea.KeyMsg:
		// When the list is filtering, let it handle all keys.
		if n.list.FilterState() == list.Filtering {
			l, cmd := n.list.Update(msg)
			n.list = l
			return n, cmd
		}

		if msg.String() == "enter" {
			selected := n.list.SelectedItem()
			if selected == nil {
				return n, nil
			}
			item := selected.(listItem)
			svc := item.svc
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

	l, cmd := n.list.Update(msg)
	n.list = l
	return n, cmd
}

func (n Navigator) View() string {
	if n.width == 0 {
		return ""
	}

	bg := ColorBg

	// Wordmark
	wordmark := StyleWordmark.Background(bg).Render("cumulus")
	tagline := StyleMuted.Background(bg).Render("AWS in your terminal")
	header := lipgloss.JoinVertical(lipgloss.Center, wordmark, tagline)

	// Size the list to fit inside the panel (width minus panel padding/border).
	panelInnerW := 60
	listH := len(n.list.Items()) * (navDelegate{}.Height() + navDelegate{}.Spacing())
	if listH < 2 {
		listH = 2
	}
	n.list.SetSize(panelInnerW, listH)

	hint := StyleDimmed.Background(bg).Render("↑/↓  navigate   enter  select   /  filter   p  switch profile   ?  help   q  quit")

	inner := lipgloss.JoinVertical(lipgloss.Left,
		header,
		"",
		n.list.View(),
		"",
		hint,
	)

	box := StylePanel.
		Background(bg).
		Width(64).
		Render(inner)

	return lipgloss.Place(
		n.width, n.height,
		lipgloss.Center, lipgloss.Center,
		box,
		lipgloss.WithWhitespaceBackground(ColorBg),
	)
}

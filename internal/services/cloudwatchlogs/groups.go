package cloudwatchlogs

import (
	"fmt"
	"strings"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
	"github.com/sumsar01/cumulus/internal/ui"
)

// logGroupsLoadedMsg carries the result of a DescribeLogGroups API call.
type logGroupsLoadedMsg struct{ groups []logGroup }

// logGroup is a lightweight representation of a CloudWatch log group.
type logGroup struct {
	name          string
	retentionDays int32 // 0 = never expire
	storedBytes   int64
	kmsKeyID      string
}

// LogGroupsModel lists all CloudWatch log groups and lets the user select one.
type LogGroupsModel struct {
	cfg       aws.Config
	groups    []logGroup
	cursor    int
	filter    string
	filtering bool
	spinner   spinner.Model
	loading   bool
	err       error
	width     int
	height    int
}

// NewLogGroupsModel constructs the log-groups view.
func NewLogGroupsModel(cfg aws.Config) LogGroupsModel {
	return LogGroupsModel{cfg: cfg, spinner: ui.NewSpinner()}
}

// Init starts loading log groups immediately.
func (m LogGroupsModel) Init() tea.Cmd {
	return tea.Batch(m.spinner.Tick, fetchLogGroupsCmd(m.cfg))
}

// Update handles messages for the log-groups view.
func (m LogGroupsModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height

	case awspkg.ProfileChangedMsg:
		m.cfg = msg.Cfg
		m.loading = true
		m.err = nil
		m.groups = nil
		m.cursor = 0
		m.filter = ""
		m.filtering = false
		return m, tea.Batch(m.spinner.Tick, fetchLogGroupsCmd(m.cfg))

	case logGroupsLoadedMsg:
		m.loading = false
		m.groups = msg.groups
		m.cursor = 0
		return m, nil

	case awspkg.ErrMsg:
		m.loading = false
		m.err = msg.Err
		return m, nil

	case spinner.TickMsg:
		if m.loading {
			sp, cmd := m.spinner.Update(msg)
			m.spinner = sp
			return m, cmd
		}

	case tea.KeyMsg:
		if m.filtering {
			switch msg.String() {
			case "esc":
				m.filtering = false
				m.filter = ""
				m.cursor = 0
			case "enter":
				m.filtering = false
			case "backspace":
				if len(m.filter) > 0 {
					m.filter = m.filter[:len(m.filter)-1]
					m.cursor = 0
				}
			default:
				if len(msg.String()) == 1 && msg.String() != "/" {
					m.filter += msg.String()
					m.cursor = 0
				}
			}
			return m, nil
		}

		switch msg.String() {
		case "/":
			m.filtering = true
		case "r":
			m.loading = true
			m.err = nil
			m.groups = nil
			m.cursor = 0
			m.filter = ""
			m.filtering = false
			return m, tea.Batch(m.spinner.Tick, fetchLogGroupsCmd(m.cfg))
		case "up", "k":
			if m.cursor > 0 {
				m.cursor--
			}
		case "down", "j":
			if m.cursor < len(m.visible())-1 {
				m.cursor++
			}
		case "esc":
			if m.filter != "" {
				m.filter = ""
				m.cursor = 0
				return m, nil
			}
		case "enter":
			vis := m.visible()
			if len(vis) == 0 {
				return m, nil
			}
			g := vis[m.cursor]
			streamsModel := NewLogStreamsModel(m.cfg, g.name)
			return m, tea.Batch(
				streamsModel.Init(),
				func() tea.Msg { return ui.PushMsg{Model: streamsModel} },
				func() tea.Msg {
					return ui.SetBreadcrumbMsg{Crumbs: []string{"CloudWatch Logs", g.name}}
				},
			)
		}
	}

	return m, nil
}

// IsTextInputActive implements ui.TextInputActive. Returns true while the user
// is typing a filter so that app.go does not intercept global shortcuts.
func (m LogGroupsModel) IsTextInputActive() bool { return m.filtering }

// visible returns the log groups matching the current filter.
func (m *LogGroupsModel) visible() []logGroup {
	if m.filter == "" {
		return m.groups
	}
	f := strings.ToLower(m.filter)
	var out []logGroup
	for _, g := range m.groups {
		if strings.Contains(strings.ToLower(g.name), f) {
			out = append(out, g)
		}
	}
	return out
}

// View renders the log-groups list.
func (m LogGroupsModel) View() string {
	if m.loading {
		content := fmt.Sprintf("%s  Loading log groups…", m.spinner.View())
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}
	if m.err != nil {
		if isCredentialError(m.err) {
			content := ui.StyleDanger.Background(ui.ColorBg).Render("No AWS credentials found.") +
				ui.StyleMuted.Background(ui.ColorBg).Render("\n\nMake sure a profile is configured in ~/.aws/config,\nor set AWS_PROFILE / AWS_ACCESS_KEY_ID in your environment.\n\nRun \"aws configure\" to set up credentials.") +
				ui.StyleDimmed.Background(ui.ColorBg).Render("\n\nr  retry   p  switch profile")
			return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
				lipgloss.WithWhitespaceBackground(ui.ColorBg))
		}
		content := ui.StyleDanger.Background(ui.ColorBg).Render(m.err.Error()) +
			ui.StyleDimmed.Background(ui.ColorBg).Render("\n\nr  retry")
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}

	vis := m.visible()
	total := len(m.groups)

	// Header
	title := ui.StyleTitle.Background(ui.ColorBg).Render("CloudWatch Logs")
	count := ui.StyleCount.Render(fmt.Sprintf("  %d / %d groups", len(vis), total))
	filterHint := ""
	if m.filtering {
		filterHint = "  " + ui.StyleKey.Background(ui.ColorBg).Render("/") +
			ui.StyleFilterActive.Render(" "+m.filter) +
			ui.StyleCount.Render("█")
	} else if m.filter != "" {
		filterHint = "  " + ui.StyleKey.Background(ui.ColorBg).Render("/") +
			ui.StyleFilterActive.Render(" "+m.filter)
	}
	header := ui.StyleItemsHeader.Width(m.width).Render(title + count + filterHint)

	sep := ui.HorizontalSep(m.width)

	maxRows := m.height - 4
	if maxRows < 1 {
		maxRows = 1
	}

	start := 0
	if m.cursor >= maxRows {
		start = m.cursor - maxRows + 1
	}
	end := start + maxRows
	if end > len(vis) {
		end = len(vis)
	}

	var rows strings.Builder
	for i := start; i < end; i++ {
		g := vis[i]
		label := g.name
		if g.retentionDays > 0 {
			label += ui.StyleDimmed.Background(ui.ColorBg).Render(
				fmt.Sprintf("  (%d days)", g.retentionDays),
			)
		}
		if i == m.cursor {
			hlSp := lipgloss.NewStyle().Background(ui.ColorHighlight).Render("  ")
			row := ui.StyleListRowSelected.Width(m.width).Render(
				hlSp +
					ui.StyleListRowSelectedCursor.Render("›") +
					hlSp +
					ui.StyleListRowSelectedName.Render(label),
			)
			rows.WriteString(row + "\n")
		} else {
			row := ui.StyleListRowNormal.Width(m.width).Render(
				"     " + ui.StyleListRowNormalName.Render(label),
			)
			rows.WriteString(row + "\n")
		}
	}

	if len(vis) == 0 && !m.filtering {
		content := ui.StyleEmptyState.Render("no log groups found")
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	} else if len(vis) == 0 {
		rows.WriteString(ui.StyleEmptyState.PaddingLeft(5).Render("no log groups found\n"))
	}

	var pairs [][2]string
	if m.filtering {
		pairs = [][2]string{{"esc", "cancel filter"}, {"enter", "confirm"}}
	} else {
		pairs = [][2]string{{"↑/↓", "navigate"}, {"enter", "open"}, {"/", "filter"}, {"r", "refresh"}}
	}
	hints := ui.RenderHints(m.width, pairs)

	rowAreaHeight := m.height - 3
	if rowAreaHeight < 1 {
		rowAreaHeight = 1
	}
	rowLines := strings.Count(rows.String(), "\n")
	for i := rowLines; i < rowAreaHeight; i++ {
		rows.WriteString(ui.StyleListRowNormal.Width(m.width).Render("") + "\n")
	}

	return header + "\n" + sep + "\n" + rows.String() + hints
}

// isCredentialError reports whether err looks like an AWS credential failure.
func isCredentialError(err error) bool {
	if err == nil {
		return false
	}
	msg := err.Error()
	return strings.Contains(msg, "get credentials") ||
		strings.Contains(msg, "no credentials") ||
		strings.Contains(msg, "failed to refresh cached credentials") ||
		strings.Contains(msg, "NoCredentialProviders")
}

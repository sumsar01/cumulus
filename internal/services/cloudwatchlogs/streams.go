package cloudwatchlogs

import (
	"fmt"
	"strings"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/charmbracelet/bubbles/spinner"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
	"github.com/sumsar01/cumulus/internal/ui"
)

// logStreamsLoadedMsg carries the result of a DescribeLogStreams API call.
type logStreamsLoadedMsg struct{ streams []logStream }

// logStream is a lightweight representation of a CloudWatch log stream.
type logStream struct {
	name          string
	lastEventTime time.Time // zero if never
}

// LogStreamsModel lists all streams within a log group.
type LogStreamsModel struct {
	cfg       aws.Config
	groupName string
	streams   []logStream
	cursor    int
	filter    string
	filtering bool
	spinner   spinner.Model
	loading   bool
	err       error
	width     int
	height    int
}

// NewLogStreamsModel constructs the log-streams view for the given log group.
func NewLogStreamsModel(cfg aws.Config, groupName string) LogStreamsModel {
	return LogStreamsModel{cfg: cfg, groupName: groupName, spinner: ui.NewSpinner()}
}

// Init starts loading log streams immediately.
func (m LogStreamsModel) Init() tea.Cmd {
	return tea.Batch(m.spinner.Tick, fetchLogStreamsCmd(m.cfg, m.groupName))
}

// Update handles messages for the log-streams view.
func (m LogStreamsModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height

	case awspkg.ProfileChangedMsg:
		m.cfg = msg.Cfg
		m.loading = true
		m.err = nil
		m.streams = nil
		m.cursor = 0
		m.filter = ""
		m.filtering = false
		return m, tea.Batch(m.spinner.Tick, fetchLogStreamsCmd(m.cfg, m.groupName))

	case logStreamsLoadedMsg:
		m.loading = false
		m.streams = msg.streams
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
			m.streams = nil
			m.cursor = 0
			m.filter = ""
			m.filtering = false
			return m, tea.Batch(m.spinner.Tick, fetchLogStreamsCmd(m.cfg, m.groupName))
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
			s := vis[m.cursor]
			eventsModel := NewLogEventsModel(m.cfg, m.groupName, s.name)
			return m, tea.Batch(
				eventsModel.Init(),
				func() tea.Msg { return ui.PushMsg{Model: eventsModel} },
				func() tea.Msg {
					return ui.SetBreadcrumbMsg{
						Crumbs: []string{"CloudWatch Logs", m.groupName, s.name},
					}
				},
			)
		}
	}

	return m, nil
}

// IsTextInputActive implements ui.TextInputActive.
func (m LogStreamsModel) IsTextInputActive() bool { return m.filtering }

// visible returns the streams matching the current filter.
func (m *LogStreamsModel) visible() []logStream {
	if m.filter == "" {
		return m.streams
	}
	f := strings.ToLower(m.filter)
	var out []logStream
	for _, s := range m.streams {
		if strings.Contains(strings.ToLower(s.name), f) {
			out = append(out, s)
		}
	}
	return out
}

// View renders the log-streams list.
func (m LogStreamsModel) View() string {
	if m.loading {
		content := fmt.Sprintf("%s  Loading log streams…", m.spinner.View())
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}
	if m.err != nil {
		content := ui.StyleDanger.Background(ui.ColorBg).Render(m.err.Error()) +
			ui.StyleDimmed.Background(ui.ColorBg).Render("\n\nr  retry")
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}

	vis := m.visible()
	total := len(m.streams)

	// Header
	title := ui.StyleTitle.Background(ui.ColorBg).Render(m.groupName)
	count := ui.StyleCount.Render(fmt.Sprintf("  %d / %d streams", len(vis), total))
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
		s := vis[i]
		label := s.name
		if !s.lastEventTime.IsZero() {
			label += ui.StyleDimmed.Background(ui.ColorBg).Render(
				"  " + s.lastEventTime.Local().Format("2006-01-02 15:04:05"),
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
		content := ui.StyleEmptyState.Render("no log streams found")
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	} else if len(vis) == 0 {
		rows.WriteString(ui.StyleEmptyState.PaddingLeft(5).Render("no log streams found\n"))
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

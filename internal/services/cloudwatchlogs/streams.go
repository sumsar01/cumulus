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
// prefix echoes the search prefix that was active when the request was made,
// so stale responses from a previous search can be discarded.
type logStreamsLoadedMsg struct {
	streams []logStream
	prefix  string
}

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
	prefix    string // active server-side prefix search
	filtering bool   // true while the user is typing a prefix
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
	return tea.Batch(m.spinner.Tick, fetchLogStreamsCmd(m.cfg, m.groupName, ""))
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
		m.prefix = ""
		m.filtering = false
		return m, tea.Batch(m.spinner.Tick, fetchLogStreamsCmd(m.cfg, m.groupName, ""))

	case awspkg.RegionChangedMsg:
		m.cfg = msg.Cfg
		m.loading = true
		m.err = nil
		m.streams = nil
		m.cursor = 0
		m.prefix = ""
		m.filtering = false
		return m, tea.Batch(m.spinner.Tick, fetchLogStreamsCmd(m.cfg, m.groupName, ""))

	case logStreamsLoadedMsg:
		// Discard stale responses from a previous prefix search.
		if msg.prefix != m.prefix {
			return m, nil
		}
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
				// Cancel search: revert to most-recent-first view.
				m.filtering = false
				m.prefix = ""
				m.cursor = 0
				m.loading = true
				m.err = nil
				return m, tea.Batch(m.spinner.Tick, fetchLogStreamsCmd(m.cfg, m.groupName, ""))
			case "enter":
				// Commit the current prefix and fetch from AWS.
				m.filtering = false
				m.loading = true
				m.err = nil
				m.streams = nil
				m.cursor = 0
				return m, tea.Batch(m.spinner.Tick, fetchLogStreamsCmd(m.cfg, m.groupName, m.prefix))
			case "backspace":
				if len(m.prefix) > 0 {
					m.prefix = m.prefix[:len(m.prefix)-1]
					m.cursor = 0
				}
			default:
				if len(msg.String()) == 1 && msg.String() != "/" {
					m.prefix += msg.String()
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
			m.prefix = ""
			m.filtering = false
			return m, tea.Batch(m.spinner.Tick, fetchLogStreamsCmd(m.cfg, m.groupName, ""))
		case "up", "k":
			if m.cursor > 0 {
				m.cursor--
			}
		case "down", "j":
			if m.cursor < len(m.streams)-1 {
				m.cursor++
			}
		case "esc":
			if m.prefix != "" {
				// Clear prefix search, reload default view.
				m.prefix = ""
				m.cursor = 0
				m.loading = true
				m.err = nil
				m.streams = nil
				return m, tea.Batch(m.spinner.Tick, fetchLogStreamsCmd(m.cfg, m.groupName, ""))
			}
		case "enter":
			if len(m.streams) == 0 {
				return m, nil
			}
			s := m.streams[m.cursor]
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

	streams := m.streams
	total := len(streams)

	// Header
	title := ui.StyleTitle.Background(ui.ColorBg).Render(m.groupName)
	var countStr string
	if m.prefix != "" {
		countStr = fmt.Sprintf("  %d streams", total)
	} else {
		countStr = fmt.Sprintf("  top %d streams", total)
	}
	count := ui.StyleCount.Render(countStr)
	prefixHint := ""
	if m.filtering {
		prefixHint = "  " + ui.StyleKey.Background(ui.ColorBg).Render("/") +
			ui.StyleFilterActive.Render(" "+m.prefix) +
			ui.StyleCount.Render("█")
	} else if m.prefix != "" {
		prefixHint = "  " + ui.StyleKey.Background(ui.ColorBg).Render("/") +
			ui.StyleFilterActive.Render(" "+m.prefix)
	}
	header := ui.StyleItemsHeader.Width(m.width).Render(title + count + prefixHint)

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
	if end > total {
		end = total
	}

	var rows strings.Builder
	for i := start; i < end; i++ {
		s := streams[i]
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

	if total == 0 && !m.filtering {
		content := ui.StyleEmptyState.Render("no log streams found")
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	} else if total == 0 {
		rows.WriteString(ui.StyleEmptyState.PaddingLeft(5).Render("no log streams found\n"))
	}

	var pairs [][2]string
	if m.filtering {
		pairs = [][2]string{{"esc", "cancel"}, {"enter", "search"}}
	} else {
		pairs = [][2]string{{"↑/↓", "navigate"}, {"enter", "open"}, {"/", "prefix search"}, {"r", "refresh"}}
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

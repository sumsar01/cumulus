package sqs

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

// queuesLoadedMsg carries the result of a ListQueues API call.
type queuesLoadedMsg struct {
	queues    []string // queue URLs
	nextToken *string  // nil when there is no further page
}

// QueuesModel lists SQS queue URLs and lets the user select one.
type QueuesModel struct {
	cfg       aws.Config
	queues    []string // all loaded queue URLs
	nextToken *string  // token for the next page; nil if no further pages loaded yet
	hasMore   bool     // true when the server indicated there is at least one more page

	cursor    int
	filter    string
	filtering bool

	spinner spinner.Model
	loading bool
	err     error
	width   int
	height  int
}

// NewQueuesModel constructs the queues view.
func NewQueuesModel(cfg aws.Config) QueuesModel {
	return QueuesModel{cfg: cfg, spinner: ui.NewSpinner(), loading: true}
}

// Init fires the initial queue fetch.
func (m QueuesModel) Init() tea.Cmd {
	return tea.Batch(m.spinner.Tick, fetchQueuesCmd(m.cfg, nil))
}

// Update handles all messages for the queues view.
func (m QueuesModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height

	case awspkg.ProfileChangedMsg:
		m.cfg = msg.Cfg
		m.loading = true
		m.err = nil
		m.queues = nil
		m.nextToken = nil
		m.hasMore = false
		m.cursor = 0
		m.filter = ""
		m.filtering = false
		return m, tea.Batch(m.spinner.Tick, fetchQueuesCmd(m.cfg, nil))

	case awspkg.RegionChangedMsg:
		m.cfg = msg.Cfg
		m.loading = true
		m.err = nil
		m.queues = nil
		m.nextToken = nil
		m.hasMore = false
		m.cursor = 0
		m.filter = ""
		m.filtering = false
		return m, tea.Batch(m.spinner.Tick, fetchQueuesCmd(m.cfg, nil))

	case queuesLoadedMsg:
		m.loading = false
		m.queues = append(m.queues, msg.queues...)
		m.nextToken = msg.nextToken
		m.hasMore = msg.nextToken != nil
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
		// While filtering, all input goes to the filter field.
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
			m.queues = nil
			m.nextToken = nil
			m.hasMore = false
			m.cursor = 0
			m.filter = ""
			m.filtering = false
			return m, tea.Batch(m.spinner.Tick, fetchQueuesCmd(m.cfg, nil))
		case "n":
			// Load the next page of queues.
			if m.hasMore && !m.loading {
				m.loading = true
				return m, tea.Batch(m.spinner.Tick, fetchQueuesCmd(m.cfg, m.nextToken))
			}
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
			queueURL := vis[m.cursor]
			msgsModel := NewMessagesModel(m.cfg, queueURL)
			return m, tea.Batch(
				msgsModel.Init(),
				func() tea.Msg { return ui.PushMsg{Model: msgsModel} },
				func() tea.Msg {
					return ui.SetBreadcrumbMsg{Crumbs: []string{"SQS", queueDisplayName(queueURL)}}
				},
			)
		}
	}

	return m, nil
}

// IsTextInputActive implements ui.TextInputActive.
func (m QueuesModel) IsTextInputActive() bool { return m.filtering }

// visible returns queue URLs matching the current filter.
func (m *QueuesModel) visible() []string {
	if m.filter == "" {
		return m.queues
	}
	var out []string
	f := strings.ToLower(m.filter)
	for _, q := range m.queues {
		if strings.Contains(strings.ToLower(q), f) {
			out = append(out, q)
		}
	}
	return out
}

// queueDisplayName extracts the queue name from its URL for display purposes.
// SQS queue URLs have the form https://sqs.<region>.amazonaws.com/<account>/<name>.
func queueDisplayName(url string) string {
	parts := strings.Split(url, "/")
	if len(parts) > 0 {
		return parts[len(parts)-1]
	}
	return url
}

// isCredentialError returns true when err looks like an AWS credential failure.
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

// View renders the queues list.
func (m QueuesModel) View() string {
	if m.loading {
		content := fmt.Sprintf("%s  Loading queues…", m.spinner.View())
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
	total := len(m.queues)

	// Header line.
	title := ui.StyleTitle.Background(ui.ColorBg).Render("SQS")
	countStr := fmt.Sprintf("  %d / %d queues", len(vis), total)
	if m.hasMore {
		countStr += "+"
	}
	count := ui.StyleCount.Render(countStr)
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

	// Separator.
	sep := ui.HorizontalSep(m.width)

	// Rows.
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
		q := vis[i]
		name := queueDisplayName(q)
		if i == m.cursor {
			hlSp := lipgloss.NewStyle().Background(ui.ColorHighlight).Render("  ")
			row := ui.StyleListRowSelected.Width(m.width).Render(
				hlSp +
					ui.StyleListRowSelectedCursor.Render("›") +
					hlSp +
					ui.StyleListRowSelectedName.Render(name),
			)
			rows.WriteString(row + "\n")
		} else {
			row := ui.StyleListRowNormal.Width(m.width).Render(
				"     " + ui.StyleListRowNormalName.Render(name),
			)
			rows.WriteString(row + "\n")
		}
	}

	if len(vis) == 0 && !m.filtering {
		content := ui.StyleEmptyState.Render("no queues found")
		return lipgloss.Place(m.width, m.height, lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	} else if len(vis) == 0 {
		rows.WriteString(ui.StyleEmptyState.PaddingLeft(5).Render("no queues match filter\n"))
	}

	// Footer hints.
	var pairs [][2]string
	if m.filtering {
		pairs = [][2]string{{"esc", "cancel filter"}, {"enter", "confirm"}}
	} else {
		pairs = [][2]string{{"↑/↓", "navigate"}, {"enter", "open"}, {"/", "filter"}, {"r", "refresh"}}
		if m.hasMore {
			pairs = append(pairs, [2]string{"n", "next page"})
		}
	}
	hints := ui.RenderHints(m.width, pairs)

	// Pad row area so hints are pinned to the bottom.
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

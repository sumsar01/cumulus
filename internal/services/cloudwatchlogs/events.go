package cloudwatchlogs

import (
	"strings"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/table"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
	"github.com/sumsar01/cumulus/internal/ui"
)

// logEventsLoadedMsg carries a page of log events.
type logEventsLoadedMsg struct {
	events    []logEvent
	nextToken string // empty = no more pages forward
	prevToken string // empty = already at the oldest page
}

// logEvent holds a single CloudWatch log event.
type logEvent struct {
	timestamp time.Time
	message   string
	ingested  time.Time
}

// LogEventsModel displays a paginated, filterable list of CloudWatch log events.
type LogEventsModel struct {
	cfg        aws.Config
	groupName  string
	streamName string

	events    []logEvent
	nextToken string
	prevToken string
	hasMore   bool

	// filter
	filterPattern string
	filtering     bool   // true while the user is actively typing the filter
	pendingFilter string // input being typed before confirmation

	// display
	table   table.Model
	rawRows []table.Row

	spinner    spinner.Model
	loading    bool
	err        error
	width      int
	height     int
	fullHeight int
}

// NewLogEventsModel constructs the log-events view for the given stream.
func NewLogEventsModel(cfg aws.Config, groupName, streamName string) LogEventsModel {
	t := table.New(
		table.WithFocused(true),
		table.WithStyles(ui.DynamoTableStyles),
	)
	return LogEventsModel{
		cfg:        cfg,
		groupName:  groupName,
		streamName: streamName,
		table:      t,
		spinner:    ui.NewSpinner(),
	}
}

// Init starts loading log events immediately.
func (m LogEventsModel) Init() tea.Cmd {
	return tea.Batch(m.spinner.Tick, fetchLogEventsCmd(m.cfg, m.groupName, m.streamName, "", m.filterPattern))
}

// Update handles messages for the log-events view.
func (m LogEventsModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.fullHeight = msg.Height + 2
		m.resizeTable()

	case awspkg.ProfileChangedMsg:
		m.cfg = msg.Cfg
		m.events = nil
		m.nextToken = ""
		m.prevToken = ""
		m.hasMore = false
		m.filterPattern = ""
		m.filtering = false
		m.pendingFilter = ""
		m.loading = true
		m.err = nil
		return m, tea.Batch(m.spinner.Tick, fetchLogEventsCmd(m.cfg, m.groupName, m.streamName, "", ""))

	case awspkg.RegionChangedMsg:
		m.cfg = msg.Cfg
		m.events = nil
		m.nextToken = ""
		m.prevToken = ""
		m.hasMore = false
		m.filterPattern = ""
		m.filtering = false
		m.pendingFilter = ""
		m.loading = true
		m.err = nil
		return m, tea.Batch(m.spinner.Tick, fetchLogEventsCmd(m.cfg, m.groupName, m.streamName, "", ""))

	case logEventsLoadedMsg:
		m.loading = false
		m.events = msg.events
		m.nextToken = msg.nextToken
		m.prevToken = msg.prevToken
		m.hasMore = msg.nextToken != ""
		m.rebuildTable()
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
		return m.handleKey(msg)
	}

	t, cmd := m.table.Update(msg)
	m.table = t
	m.injectCursor()
	return m, cmd
}

func (m LogEventsModel) handleKey(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	// While the filter input is open, route all keys to it.
	if m.filtering {
		switch msg.String() {
		case "esc":
			m.filtering = false
			m.pendingFilter = ""
		case "enter":
			m.filtering = false
			m.filterPattern = m.pendingFilter
			m.pendingFilter = ""
			m.loading = true
			m.err = nil
			m.events = nil
			m.nextToken = ""
			m.prevToken = ""
			return m, tea.Batch(m.spinner.Tick,
				fetchLogEventsCmd(m.cfg, m.groupName, m.streamName, "", m.filterPattern))
		case "backspace":
			if len(m.pendingFilter) > 0 {
				m.pendingFilter = m.pendingFilter[:len(m.pendingFilter)-1]
			}
		default:
			if len(msg.String()) == 1 {
				m.pendingFilter += msg.String()
			}
		}
		return m, nil
	}

	switch msg.String() {
	case "/":
		m.filtering = true
		m.pendingFilter = m.filterPattern // pre-fill with active pattern
		return m, nil

	case "r":
		m.loading = true
		m.err = nil
		m.events = nil
		m.nextToken = ""
		m.prevToken = ""
		return m, tea.Batch(m.spinner.Tick,
			fetchLogEventsCmd(m.cfg, m.groupName, m.streamName, "", m.filterPattern))

	case "n", "pgdown", "right":
		if m.hasMore && m.nextToken != "" {
			m.loading = true
			m.err = nil
			return m, tea.Batch(m.spinner.Tick,
				fetchLogEventsCmd(m.cfg, m.groupName, m.streamName, m.nextToken, m.filterPattern))
		}

	case "p", "pgup", "left":
		if m.prevToken != "" {
			m.loading = true
			m.err = nil
			return m, tea.Batch(m.spinner.Tick,
				fetchLogEventsCmd(m.cfg, m.groupName, m.streamName, m.prevToken, m.filterPattern))
		}

	case "enter":
		idx := m.table.Cursor()
		if idx < 0 || idx >= len(m.events) {
			return m, nil
		}
		ev := m.events[idx]
		detail := NewEventDetailModel(ev, m.groupName, m.streamName, m.width, m.height)
		return m, tea.Batch(
			detail.Init(),
			func() tea.Msg { return ui.PushMsg{Model: detail} },
		)
	}

	t, cmd := m.table.Update(msg)
	m.table = t
	m.injectCursor()
	return m, cmd
}

// IsTextInputActive implements ui.TextInputActive.
func (m LogEventsModel) IsTextInputActive() bool { return m.filtering }

// View renders the log-events view.
func (m LogEventsModel) View() string {
	if m.loading {
		content := ui.StyleMuted.Background(ui.ColorBg).Render(
			m.spinner.View() + "  Loading log events…",
		)
		return lipgloss.Place(m.width, m.fullHeight,
			lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}
	if m.err != nil {
		content := ui.StyleDanger.Background(ui.ColorBg).Render("  "+m.err.Error()) +
			"\n" + ui.StyleMuted.Background(ui.ColorBg).Render("  r  retry")
		return lipgloss.Place(m.width, m.fullHeight,
			lipgloss.Center, lipgloss.Center, content,
			lipgloss.WithWhitespaceBackground(ui.ColorBg))
	}

	header := m.headerView()
	sep := ui.HorizontalSep(m.width)

	var hints string
	if m.filtering {
		hints = ui.RenderHints(m.width, [][2]string{
			{"esc", "cancel"}, {"enter", "apply filter"},
		})
	} else {
		pairs := [][2]string{
			{"enter", "detail"}, {"/", "filter"}, {"r", "refresh"},
		}
		if m.hasMore {
			pairs = append(pairs, [2]string{"n", "next page"})
		}
		if m.prevToken != "" {
			pairs = append(pairs, [2]string{"p", "prev page"})
		}
		hints = ui.RenderHints(m.width, pairs)
	}

	lineStyle := lipgloss.NewStyle().Background(ui.ColorBg).MaxWidth(m.width).Width(m.width)
	selectedStyle := lipgloss.NewStyle().
		Background(ui.ColorHighlight).
		Foreground(ui.ColorText).
		Bold(true).
		MaxWidth(m.width).
		Width(m.width)
	selectedLine := 2 + m.table.Cursor()
	tableLines := strings.Split(m.table.View(), "\n")
	for i, line := range tableLines {
		if i == selectedLine {
			clean := ansi.Strip(line)
			rendered := selectedStyle.Render(clean)
			rendered = strings.Replace(rendered, "› ", ui.StyleCursor.Render("›")+" ", 1)
			tableLines[i] = rendered
		} else {
			tableLines[i] = lineStyle.Render(line)
		}
	}

	return header + "\n" + sep + "\n" + strings.Join(tableLines, "\n") + "\n" + hints
}

func (m LogEventsModel) headerView() string {
	title := ui.StyleTitle.Background(ui.ColorBg).Render(m.streamName)

	var filterBadge string
	if m.filtering {
		filterBadge = ui.StyleFilterLabel.Render("  / ") +
			ui.StyleFilterValue.Render(m.pendingFilter) +
			ui.StyleMuted.Render("█")
	} else if m.filterPattern != "" {
		filterBadge = ui.StyleFilterLabel.Render("  / ") +
			ui.StyleFilterValue.Render(m.filterPattern)
	}

	return ui.StyleItemsHeader.Width(m.width).Render(title + filterBadge)
}

// ── helpers ───────────────────────────────────────────────────────────────────

func (m *LogEventsModel) rebuildTable() {
	if len(m.events) == 0 {
		m.table.SetRows(nil)
		m.table.SetColumns(nil)
		return
	}

	// Two columns: Timestamp and Message.
	// Timestamp is fixed at 22 chars ("2006-01-02 15:04:05.000").
	// Message fills the rest, minus 2 chars per column padding from bubbles/table.
	const tsWidth = 22 + 2                    // column width + cursor prefix on col 0
	msgWidth := m.width - tsWidth - 2 - 2 - 4 // 2 borders, 2 padding per col, cursor
	if msgWidth < 10 {
		msgWidth = 10
	}

	tCols := []table.Column{
		{Title: "Timestamp", Width: tsWidth},
		{Title: "Message", Width: msgWidth},
	}

	raw := make([]table.Row, len(m.events))
	for i, ev := range m.events {
		ts := ""
		if !ev.timestamp.IsZero() {
			ts = ev.timestamp.Local().Format("2006-01-02 15:04:05")
		}
		// Collapse newlines so each event is a single table row.
		msg := strings.ReplaceAll(ev.message, "\n", " ")
		raw[i] = table.Row{ts, msg}
	}
	m.rawRows = raw
	m.table.SetColumns(tCols)
	m.injectCursor()
	m.resizeTable()
}

func (m *LogEventsModel) resizeTable() {
	if m.height > 4 {
		m.table.SetHeight(m.height - 4)
	}
	if m.width > 0 {
		m.table.SetWidth(m.width)
		m.table.SetStyles(ui.DynamoTableStyles)
	}
}

func (m *LogEventsModel) injectCursor() {
	if len(m.rawRows) == 0 {
		m.table.SetRows(nil)
		return
	}
	cursor := m.table.Cursor()
	display := make([]table.Row, len(m.rawRows))
	for i, raw := range m.rawRows {
		row := make(table.Row, len(raw))
		copy(row, raw)
		if len(row) > 0 {
			if i == cursor {
				row[0] = "› " + raw[0]
			} else {
				row[0] = "  " + raw[0]
			}
		}
		display[i] = row
	}
	m.table.SetRows(display)
}

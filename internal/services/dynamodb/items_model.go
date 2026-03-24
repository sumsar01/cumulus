package dynamodb

import (
	"encoding/json"
	"fmt"
	"sort"
	"strings"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/feature/dynamodb/attributevalue"
	ddbtypes "github.com/aws/aws-sdk-go-v2/service/dynamodb/types"
	"github.com/charmbracelet/bubbles/spinner"
	"github.com/charmbracelet/bubbles/table"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	"github.com/charmbracelet/x/ansi"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
	"github.com/sumsar01/cumulus/internal/config"
	"github.com/sumsar01/cumulus/internal/ui"
)

// itemsLoadedMsg carries a page of DynamoDB items.
type itemsLoadedMsg struct {
	items         []map[string]ddbtypes.AttributeValue
	lastKey       map[string]ddbtypes.AttributeValue
	tableKeyNames tableKeyInfo
}

// tableKeyInfo holds the key schema of a DynamoDB table.
type tableKeyInfo struct {
	pk string
	sk string // empty if table has no sort key
}

// itemDeletedMsg signals a successful DeleteItem.
type itemDeletedMsg struct{}

// itemSavedMsg signals a successful PutItem.
type itemSavedMsg struct{}

// scanMode describes the current fetch strategy.
type scanMode int

const (
	modeScan  scanMode = iota
	modeQuery          // query with PK (and optional SK)
)

// ItemsModel displays a paginated, filterable table of DynamoDB items.
type ItemsModel struct {
	cfg       aws.Config
	appCfg    config.Config
	tableName string
	keyInfo   tableKeyInfo

	// pagination
	mode        scanMode
	queryPK     string
	querySK     string
	filterExpr  string
	pages       [][]map[string]ddbtypes.AttributeValue // loaded pages
	currentPage int
	lastKey     map[string]ddbtypes.AttributeValue
	hasMore     bool

	// local (client-side) filter
	localFilter    string
	localFiltering bool                                 // true while the user is actively typing the filter
	localFilterCol int                                  // column index to match against; -1 = all columns
	filteredItems  []map[string]ddbtypes.AttributeValue // items matching localFilter

	// display
	table   table.Model
	columns []string    // current column set
	rawRows []table.Row // rows without cursor prefix (source of truth)

	spinner spinner.Model
	loading bool
	err     error

	// prompt overlay
	activePrompt  *Prompt
	promptPurpose promptPurpose

	width      int
	height     int
	fullHeight int // full terminal height before status bar deduction, used for prompt centering
}

type promptPurpose int

const (
	purposeNone promptPurpose = iota
	purposeFilter
	purposeQueryPK
	purposeQuerySK
	purposeDelete
)

// NewItemsModel constructs the items view for the given table.
func NewItemsModel(cfg aws.Config, appCfg config.Config, tableName string) ItemsModel {
	t := table.New(
		table.WithFocused(true),
		table.WithStyles(ui.DynamoTableStyles),
	)

	return ItemsModel{
		cfg:       cfg,
		appCfg:    appCfg,
		tableName: tableName,
		table:     t,
		spinner:   ui.NewSpinner(),
	}
}

func (m ItemsModel) Init() tea.Cmd {
	return tea.Batch(m.spinner.Tick, m.freshFetchCmd())
}

func (m ItemsModel) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
	// Handle prompt overlay first.
	if m.activePrompt != nil {
		if km, ok := msg.(tea.KeyMsg); ok {
			p, cmd, done := m.activePrompt.Update(km)
			if done {
				m.activePrompt = nil
				return m, cmd // cmd emits promptDoneMsg
			}
			m.activePrompt = &p
			return m, cmd
		}
		// Non-key messages fall through to be handled normally.
	}

	switch msg := msg.(type) {

	case tea.WindowSizeMsg:
		m.width = msg.Width
		m.height = msg.Height
		m.fullHeight = msg.Height + 2 // restore full height (app strips 2 for status bar)
		m.resizeTable()

	case awspkg.ProfileChangedMsg:
		m.cfg = msg.Cfg
		m.reset()
		return m, tea.Batch(m.spinner.Tick, m.freshFetchCmd())

	case awspkg.RegionChangedMsg:
		m.cfg = msg.Cfg
		m.reset()
		return m, tea.Batch(m.spinner.Tick, m.freshFetchCmd())

	case spinner.TickMsg:
		if m.loading {
			sp, cmd := m.spinner.Update(msg)
			m.spinner = sp
			return m, cmd
		}

	case itemsLoadedMsg:
		m.loading = false
		m.keyInfo = msg.tableKeyNames
		m.lastKey = msg.lastKey
		m.hasMore = len(msg.lastKey) > 0

		if m.currentPage < len(m.pages) {
			m.pages[m.currentPage] = msg.items
		} else {
			m.pages = append(m.pages, msg.items)
		}
		m.rebuildTable()
		return m, nil

	case itemDeletedMsg:
		m.loading = true
		return m, tea.Batch(m.spinner.Tick, m.currentFetchCmd(nil))

	case itemSavedMsg:
		m.loading = true
		return m, tea.Batch(m.spinner.Tick, m.currentFetchCmd(nil))

	case editorDoneMsg:
		if msg.Err != nil {
			return m, func() tea.Msg { return ui.SetErrorMsg{Err: msg.Err.Error()} }
		}
		if msg.Data == nil {
			return m, nil // editor opened but nothing changed (shouldn't happen)
		}
		return m, putItemCmd(m.cfg, m.tableName, msg.Data)

	case promptDoneMsg:
		if msg.Cancelled {
			m.activePrompt = nil
			m.promptPurpose = purposeNone
			return m, nil
		}
		return m.handlePromptDone(msg.Value)

	case awspkg.ErrMsg:
		m.loading = false
		m.err = msg.Err
		return m, nil

	case tea.KeyMsg:
		return m.handleKey(msg)
	}

	t, cmd := m.table.Update(msg)
	m.table = t
	m.injectCursor()
	return m, cmd
}

func (m ItemsModel) handleKey(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	// While the local filter is active, all input goes to the filter field.
	if m.localFiltering {
		switch msg.String() {
		case "esc":
			m.localFiltering = false
			m.localFilter = ""
			m.rebuildTable()
		case "enter":
			// Confirm filter — exit typing mode but keep the filter active.
			m.localFiltering = false
		case "backspace":
			if len(m.localFilter) > 0 {
				m.localFilter = m.localFilter[:len(m.localFilter)-1]
				m.rebuildTable()
			}
		case "tab":
			// Cycle forward through columns: -1 (all) → 0 → 1 → … → n-1 → -1
			if len(m.columns) > 0 {
				m.localFilterCol = (m.localFilterCol+2)%(len(m.columns)+1) - 1
				m.rebuildTable()
			}
		case "shift+tab":
			// Cycle backward through columns.
			if len(m.columns) > 0 {
				n := len(m.columns)
				m.localFilterCol = ((m.localFilterCol + 1 + n) % (n + 1)) - 1
				m.rebuildTable()
			}
		default:
			if len(msg.String()) == 1 {
				m.localFilter += msg.String()
				m.rebuildTable()
			}
		}
		return m, nil
	}

	switch msg.String() {
	case "r":
		m.reset()
		return m, tea.Batch(m.spinner.Tick, m.freshFetchCmd())

	case "/":
		// Start inline client-side filter. Default to first column (PK).
		m.localFiltering = true
		if len(m.columns) > 0 && m.localFilterCol < 0 {
			m.localFilterCol = 0
		}
		return m, nil

	case "F":
		// Open DynamoDB FilterExpression prompt (server-side, advanced).
		p := NewFilterPrompt()
		m.activePrompt = &p
		m.promptPurpose = purposeFilter
		return m, nil

	case "Q":
		p := NewQueryPKPrompt(m.keyInfo.pk)
		m.activePrompt = &p
		m.promptPurpose = purposeQueryPK
		return m, nil

	case "n":
		return m, startEditorCmd(m.appCfg, emptyItemJSON())

	case "e":
		item := m.selectedRawItem()
		if item == nil {
			return m, nil
		}
		jsonBytes, err := marshalItemToJSON(item)
		if err != nil {
			return m, func() tea.Msg { return ui.SetErrorMsg{Err: err.Error()} }
		}
		return m, startEditorCmd(m.appCfg, jsonBytes)

	case "d":
		item := m.selectedRawItem()
		if item == nil {
			return m, nil
		}
		p := NewConfirmPrompt(fmt.Sprintf("Delete item from %s?", m.tableName))
		m.activePrompt = &p
		m.promptPurpose = purposeDelete
		return m, nil

	case "enter":
		item := m.selectedRawItem()
		if item == nil {
			return m, nil
		}
		detail := NewDetailModel(item, m.tableName, m.width, m.height)
		return m, tea.Batch(
			detail.Init(),
			func() tea.Msg { return ui.PushMsg{Model: detail} },
		)

	case "pgdown", "right":
		if m.hasMore {
			m.currentPage++
			m.loading = true
			return m, tea.Batch(m.spinner.Tick, m.currentFetchCmd(m.lastKey))
		}

	case "pgup", "left":
		if m.currentPage > 0 {
			m.currentPage--
			m.rebuildTable()
		}
	}

	t, cmd := m.table.Update(msg)
	m.table = t
	m.injectCursor()
	return m, cmd
}

func (m *ItemsModel) handlePromptDone(value string) (tea.Model, tea.Cmd) {
	switch m.promptPurpose {
	case purposeFilter:
		m.filterExpr = value
		m.mode = modeScan
		m.reset()
		return m, tea.Batch(m.spinner.Tick, m.currentFetchCmd(nil))

	case purposeQueryPK:
		m.queryPK = value
		if m.keyInfo.sk != "" {
			p := NewQuerySKPrompt(m.keyInfo.sk)
			m.activePrompt = &p
			m.promptPurpose = purposeQuerySK
			return m, nil
		}
		m.mode = modeQuery
		m.reset()
		return m, tea.Batch(m.spinner.Tick, m.currentFetchCmd(nil))

	case purposeQuerySK:
		m.querySK = value
		m.mode = modeQuery
		m.reset()
		return m, tea.Batch(m.spinner.Tick, m.currentFetchCmd(nil))

	case purposeDelete:
		if strings.ToLower(value) == "yes" {
			item := m.selectedRawItem()
			if item == nil {
				return m, nil
			}
			return m, deleteItemCmd(m.cfg, m.tableName, item, m.keyInfo)
		}
	}
	return m, nil
}

func (m ItemsModel) View() string {
	if m.loading {
		content := ui.StyleMuted.Background(ui.ColorBg).Render(
			m.spinner.View() + "  Loading items…",
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

	if m.activePrompt != nil {
		return m.activePrompt.View(m.width, m.fullHeight)
	}

	header := m.headerView()
	sep := ui.HorizontalSep(m.width)

	var hints string
	if m.localFiltering {
		hints = ui.RenderHints(m.width, [][2]string{
			{"esc", "cancel"}, {"enter", "confirm"}, {"tab", "change col"},
		})
	} else {
		hints = ui.RenderHints(m.width, [][2]string{
			{"enter", "detail"}, {"e", "edit"}, {"n", "new"}, {"d", "delete"},
			{"/", "filter"}, {"F", "expr filter"}, {"Q", "query"}, {"r", "refresh"}, {"←/→", "pages"},
		})
	}

	// Re-style every line emitted by the table widget (including the blank
	// viewport-padding lines) to force the theme background colour.  Without
	// this, bubbles/table's internal viewport fills unused rows with a bare
	// lipgloss.NewStyle() — no background — which renders as terminal-default
	// black on Tokyo Night.
	//
	// The selected row requires special handling to get a full-width highlight:
	//
	//   1. bubbles/table's Selected style has no Width, so it never pads to the
	//      terminal edge.
	//   2. injectCursor() stores a plain "› " prefix (no pre-rendered ANSI) in
	//      column 0, so there are no embedded \x1b[0m resets inside the row text
	//      that would cancel a wrapping background mid-line.
	//   3. We strip any residual ANSI codes from the selected line, re-render it
	//      with the full-width selectedStyle, then re-inject the coloured cursor
	//      glyph so it retains its accent colour against the highlight background.
	//
	// bubbles/table output layout after strings.Split on "\n":
	//   line 0  — header row
	//   line 1  — header bottom border
	//   line 2+ — data rows  (index 2 + cursor = selected row)
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
			// Strip existing ANSI codes so no embedded resets fight the new bg.
			clean := ansi.Strip(line)
			rendered := selectedStyle.Render(clean)
			// Re-apply accent colour to the cursor glyph that injectCursor placed
			// as plain text at the start of column 0.
			rendered = strings.Replace(rendered, "› ", ui.StyleCursor.Render("›")+" ", 1)
			tableLines[i] = rendered
		} else {
			tableLines[i] = lineStyle.Render(line)
		}
	}

	body := header + "\n" + sep + "\n" + strings.Join(tableLines, "\n") + "\n" + hints
	return body
}

func (m ItemsModel) headerView() string {
	title := ui.StyleTitle.Background(ui.ColorBg).Render(m.tableName)

	var modeBadge string
	if m.mode == modeQuery {
		modeBadge = ui.StyleModeBadgeQuery.Render("query")
		q := ui.StyleKeyLabel.Render("pk=") +
			ui.StyleValueLabel.Render(m.queryPK)
		if m.querySK != "" {
			q += ui.StyleKeyLabel.Render("  sk=") +
				ui.StyleValueLabel.Render(m.querySK)
		}
		modeBadge += "  " + q
	} else {
		modeBadge = ui.StyleModeBadgeScan.Render("scan")
	}
	if m.filterExpr != "" {
		modeBadge += ui.StyleFilterLabel.Render("  expr: ") +
			ui.StyleFilterValue.Render(m.filterExpr)
	}
	if m.localFiltering || m.localFilter != "" {
		colName := "all"
		if m.localFilterCol >= 0 && m.localFilterCol < len(m.columns) {
			colName = m.columns[m.localFilterCol]
		}
		cursor := ""
		if m.localFiltering {
			cursor = "█"
		}
		modeBadge += ui.StyleFilterLabel.Render("  / ") +
			ui.StyleFilterValue.Render(m.localFilter+cursor) +
			ui.StyleMuted.Render("  ["+colName+"]")
	}
	page := fmt.Sprintf("page %d", m.currentPage+1)
	if m.hasMore {
		page += "+"
	}
	pagePart := ui.StylePageIndicator.Render("  " + page)

	return ui.StyleItemsHeader.Width(m.width).Render(title + "  " + modeBadge + pagePart)
}

// ── helpers ───────────────────────────────────────────────────────────────────

func (m *ItemsModel) reset() {
	m.pages = nil
	m.currentPage = 0
	m.lastKey = nil
	m.hasMore = false
	m.loading = true
	m.err = nil
	m.localFilter = ""
	m.localFiltering = false
	// localFilterCol is intentionally preserved — user likely wants the same column next time.
}

func (m *ItemsModel) currentItems() []map[string]ddbtypes.AttributeValue {
	if m.currentPage >= len(m.pages) {
		return nil
	}
	return m.pages[m.currentPage]
}

func (m *ItemsModel) selectedRawItem() map[string]ddbtypes.AttributeValue {
	// When a local filter is active, filteredItems is the visible subset; use
	// that so the cursor index always points to the correct underlying DDB item.
	items := m.filteredItems
	if items == nil {
		items = m.currentItems()
	}
	idx := m.table.Cursor()
	if idx < 0 || idx >= len(items) {
		return nil
	}
	return items[idx]
}

func (m *ItemsModel) rebuildTable() {
	items := m.currentItems()
	if len(items) == 0 {
		m.table.SetRows(nil)
		m.table.SetColumns(nil)
		return
	}

	// Collect all attribute names across items on this page.
	attrSet := map[string]struct{}{}
	for _, item := range items {
		for k := range item {
			attrSet[k] = struct{}{}
		}
	}

	// Sort columns: PK first, SK second, then alphabetical.
	cols := make([]string, 0, len(attrSet))
	for k := range attrSet {
		cols = append(cols, k)
	}
	sort.Slice(cols, func(i, j int) bool {
		pi, pj := colPriority(cols[i], m.keyInfo), colPriority(cols[j], m.keyInfo)
		if pi != pj {
			return pi < pj
		}
		return cols[i] < cols[j]
	})
	m.columns = cols

	// Build table columns — divide available width evenly, distributing any
	// remainder across the first N columns so the header fills the terminal
	// exactly with no unused space on the right.  Each cell gets 2 chars of
	// horizontal padding from bubbles/table's default style (Padding(0,1)),
	// so we subtract len(cols)*2 before dividing.
	colWidth := 20
	remainder := 0
	if m.width > 0 && len(cols) > 0 {
		available := m.width - 2 - len(cols)*2
		colWidth = available / len(cols)
		remainder = available % len(cols)
		if colWidth < 1 {
			colWidth = 1
			remainder = 0
		}
	}

	tCols := make([]table.Column, len(cols))
	for i, c := range cols {
		w := colWidth
		if i < remainder {
			w++ // distribute leftover chars to the first N columns
		}
		if i == 0 {
			w += 2 // extra width for "› " / "  " cursor prefix
		}
		tCols[i] = table.Column{Title: c, Width: w}
	}

	// Build raw rows (no cursor prefix); injectCursor adds "› " / "  " at render time.
	raw := make([]table.Row, len(items))
	for i, item := range items {
		row := make(table.Row, len(cols))
		for j, col := range cols {
			val := "—"
			if av, ok := item[col]; ok {
				val = attrValueString(av)
			}
			row[j] = val
		}
		raw[i] = row
	}

	// Apply the local (client-side) filter, keeping filteredItems in sync with rawRows
	// so that selectedRawItem() always returns the correct underlying DDB item.
	if m.localFilter == "" {
		m.rawRows = raw
		m.filteredItems = items
	} else {
		f := strings.ToLower(m.localFilter)
		var filtRows []table.Row
		var filtItems []map[string]ddbtypes.AttributeValue
		for i, row := range raw {
			if m.rowMatchesFilter(row, f) {
				filtRows = append(filtRows, row)
				filtItems = append(filtItems, items[i])
			}
		}
		m.rawRows = filtRows
		m.filteredItems = filtItems
	}

	m.table.SetRows(nil)
	m.table.SetColumns(tCols)
	m.injectCursor()
	m.resizeTable()
}

func (m *ItemsModel) resizeTable() {
	if m.height > 4 {
		m.table.SetHeight(m.height - 4)
	}
	if m.width > 0 {
		m.table.SetWidth(m.width)
		m.table.SetStyles(ui.DynamoTableStyles)
	}
}

// injectCursor builds the display rows from m.rawRows by prepending "› " to the
// first column of the selected row and "  " to all others, then sets them on
// m.table.  Call this whenever rawRows or the cursor position changes.
func (m *ItemsModel) injectCursor() {
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
				// Use plain text prefix here — no pre-rendered ANSI.  The cursor
				// glyph's accent colour is applied later in View() after the full
				// selected-row highlight has been rendered, avoiding embedded
				// \x1b[0m resets that would cancel the background mid-line.
				row[0] = "› " + raw[0]
			} else {
				row[0] = "  " + raw[0]
			}
		}
		display[i] = row
	}
	m.table.SetRows(display)
}

// IsTextInputActive implements ui.TextInputActive. Returns true while a prompt
// overlay is open or the user is typing a local filter, so that app.go does
// not intercept global shortcuts.
func (m ItemsModel) IsTextInputActive() bool { return m.activePrompt != nil || m.localFiltering }

func colPriority(name string, ki tableKeyInfo) int {
	switch name {
	case ki.pk:
		return 0
	case ki.sk:
		return 1
	default:
		return 2
	}
}

// rowMatchesFilter reports whether a table row matches the (already lowercased)
// filter string f.  When localFilterCol < 0 every cell is checked; otherwise
// only the cell at localFilterCol is checked.
func (m *ItemsModel) rowMatchesFilter(row table.Row, f string) bool {
	if m.localFilterCol < 0 {
		for _, cell := range row {
			if strings.Contains(strings.ToLower(cell), f) {
				return true
			}
		}
		return false
	}
	if m.localFilterCol < len(row) {
		return strings.Contains(strings.ToLower(row[m.localFilterCol]), f)
	}
	return false
}

// marshalItemToJSON converts a raw DynamoDB item to pretty-printed JSON.
func marshalItemToJSON(item map[string]ddbtypes.AttributeValue) ([]byte, error) {
	// attributevalue.UnmarshalMap converts DynamoDB types to Go native types.
	var generic map[string]interface{}
	if err := attributevalue.UnmarshalMap(item, &generic); err != nil {
		return nil, fmt.Errorf("unmarshalling item: %w", err)
	}
	b, err := json.MarshalIndent(generic, "", "  ")
	if err != nil {
		return nil, fmt.Errorf("marshalling item to JSON: %w", err)
	}
	return b, nil
}

// freshFetchCmd returns a scan from page 1, discarding the cached key info so
// that a fresh DescribeTable call is made (used when profile changes or on Init).
func (m ItemsModel) freshFetchCmd() tea.Cmd {
	return fetchItemsCmd(fetchParams{
		cfg:       m.cfg,
		tableName: m.tableName,
		mode:      modeScan,
	})
}

// currentFetchCmd returns a fetch command using the model's current mode,
// query params, filter, and cached key info.
func (m ItemsModel) currentFetchCmd(startKey map[string]ddbtypes.AttributeValue) tea.Cmd {
	return fetchItemsCmd(fetchParams{
		cfg:           m.cfg,
		tableName:     m.tableName,
		mode:          m.mode,
		queryPK:       m.queryPK,
		querySK:       m.querySK,
		filterExpr:    m.filterExpr,
		startKey:      startKey,
		cachedKeyInfo: m.keyInfo,
	})
}

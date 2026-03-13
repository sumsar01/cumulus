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

	// display
	table   table.Model
	columns []string // current column set

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
		table.WithStyles(tableStyles()),
	)

	return ItemsModel{
		cfg:       cfg,
		appCfg:    appCfg,
		tableName: tableName,
		table:     t,
		spinner:   ui.NewSpinner(),
	}
}

func tableStyles() table.Styles {
	s := table.DefaultStyles()
	s.Header = s.Header.
		BorderStyle(lipgloss.NormalBorder()).
		BorderForeground(ui.ColorBorder).
		BorderBottom(true).
		Foreground(ui.ColorMuted).
		Background(ui.ColorSurface).
		Bold(false)
	s.Selected = s.Selected.
		Foreground(ui.ColorText).
		Background(ui.ColorHighlight).
		Bold(true)
	s.Cell = s.Cell.
		Foreground(ui.ColorSubtext)
	return s
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
	return m, cmd
}

func (m ItemsModel) handleKey(msg tea.KeyMsg) (tea.Model, tea.Cmd) {
	switch msg.String() {
	case "r":
		m.reset()
		return m, tea.Batch(m.spinner.Tick, m.freshFetchCmd())

	case "/":
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
		return fmt.Sprintf("  %s  Loading items…", m.spinner.View())
	}
	if m.err != nil {
		return ui.StyleDanger.Render("  "+m.err.Error()) +
			ui.StyleMuted.Render("\n\n  r  retry")
	}

	if m.activePrompt != nil {
		return m.activePrompt.View(m.width, m.fullHeight)
	}

	header := m.headerView()
	sep := ui.HorizontalSep(m.width)

	hints := ui.RenderHints(m.width, [][2]string{
		{"enter", "detail"}, {"e", "edit"}, {"n", "new"}, {"d", "delete"},
		{"/", "filter"}, {"Q", "query"}, {"r", "refresh"}, {"←/→", "pages"},
	})

	return header + "\n" + sep + "\n" + m.table.View() + "\n" + hints
}

func (m ItemsModel) headerView() string {
	title := ui.StyleTitle.Render(m.tableName)

	var modeBadge string
	if m.mode == modeQuery {
		modeBadge = lipgloss.NewStyle().
			Foreground(ui.ColorBg).Background(ui.ColorPrimary).
			PaddingLeft(1).PaddingRight(1).Render("query")
		q := lipgloss.NewStyle().Foreground(ui.ColorAccent).Render("pk=") +
			lipgloss.NewStyle().Foreground(ui.ColorText).Render(m.queryPK)
		if m.querySK != "" {
			q += lipgloss.NewStyle().Foreground(ui.ColorAccent).Render("  sk=") +
				lipgloss.NewStyle().Foreground(ui.ColorText).Render(m.querySK)
		}
		modeBadge += "  " + q
	} else {
		modeBadge = lipgloss.NewStyle().
			Foreground(ui.ColorBg).Background(ui.ColorMuted).
			PaddingLeft(1).PaddingRight(1).Render("scan")
	}
	if m.filterExpr != "" {
		modeBadge += lipgloss.NewStyle().Foreground(ui.ColorMuted).Render("  filter: ") +
			lipgloss.NewStyle().Foreground(ui.ColorSubtext).Render(m.filterExpr)
	}
	page := fmt.Sprintf("page %d", m.currentPage+1)
	if m.hasMore {
		page += "+"
	}
	pagePart := lipgloss.NewStyle().Foreground(ui.ColorMuted).Render("  " + page)

	return lipgloss.NewStyle().PaddingLeft(2).Render(title + "  " + modeBadge + pagePart)
}

// ── helpers ───────────────────────────────────────────────────────────────────

func (m *ItemsModel) reset() {
	m.pages = nil
	m.currentPage = 0
	m.lastKey = nil
	m.hasMore = false
	m.loading = true
	m.err = nil
}

func (m *ItemsModel) currentItems() []map[string]ddbtypes.AttributeValue {
	if m.currentPage >= len(m.pages) {
		return nil
	}
	return m.pages[m.currentPage]
}

func (m *ItemsModel) selectedRawItem() map[string]ddbtypes.AttributeValue {
	items := m.currentItems()
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

	// Build table columns — divide width evenly, min 10 chars each.
	colWidth := 20
	if m.width > 0 && len(cols) > 0 {
		colWidth = (m.width - 4) / len(cols)
		if colWidth < 10 {
			colWidth = 10
		}
		if colWidth > 40 {
			colWidth = 40
		}
	}

	tCols := make([]table.Column, len(cols))
	for i, c := range cols {
		tCols[i] = table.Column{Title: c, Width: colWidth}
	}

	// Build rows.
	rows := make([]table.Row, len(items))
	for i, item := range items {
		row := make(table.Row, len(cols))
		for j, col := range cols {
			if av, ok := item[col]; ok {
				row[j] = attrValueString(av)
			} else {
				row[j] = "—"
			}
		}
		rows[i] = row
	}

	m.table.SetRows(nil)
	m.table.SetColumns(tCols)
	m.table.SetRows(rows)
	m.resizeTable()
}

func (m *ItemsModel) resizeTable() {
	if m.height > 4 {
		m.table.SetHeight(m.height - 4)
	}
	if m.width > 0 {
		m.table.SetWidth(m.width)
		m.table.SetStyles(tableStyles())
	}
}

// IsTextInputActive implements ui.TextInputActive. Returns true while a prompt
// overlay is open so that app.go does not intercept global shortcuts.
func (m ItemsModel) IsTextInputActive() bool { return m.activePrompt != nil }

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

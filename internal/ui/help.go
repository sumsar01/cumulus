package ui

import (
	"github.com/charmbracelet/lipgloss"
)

// HelpOverlay renders a keyboard shortcut reference as a centred overlay.
type HelpOverlay struct{}

// NewHelpOverlay constructs an empty HelpOverlay.
func NewHelpOverlay() HelpOverlay { return HelpOverlay{} }

// View renders the help content centred on screen.
func (h *HelpOverlay) View(width, height int) string {
	bg := ColorSurface

	styleTitle := StyleTitle.Background(bg)
	styleSep := StyleDimmed.Background(bg)
	styleHeader := StyleSubtitle.Background(bg)
	styleKey := StyleKey.Background(bg)
	styleDesc := StyleMuted.Background(bg)
	styleDimmed := StyleDimmed.Background(bg)

	header := func(s string) string {
		return "\n" + styleHeader.Render(s) + "\n"
	}
	row := func(key, desc string) string {
		k := styleKey.Width(22).Render(key)
		d := styleDesc.Render(desc)
		return k + d + "\n"
	}

	title := styleTitle.Render("keyboard shortcuts")
	sep := styleSep.Render("────────────────────────────────────────")

	global := header("Global") +
		row("q / ctrl+c", "quit") +
		row("p", "switch AWS profile") +
		row("t", "switch theme") +
		row("?", "toggle help") +
		row("esc", "go back / close overlay")

	nav := header("Navigation") +
		row("↑ / ↓  or  k / j", "move cursor") +
		row("enter", "select / drill in") +
		row("←  pgup  /  →  pgdn", "previous / next page")

	dynamo := header("DynamoDB — Tables") +
		row("r", "refresh") +
		row("/", "filter tables") +
		row("esc", "clear filter / go back")

	items := header("DynamoDB — Items") +
		row("r", "refresh") +
		row("Q", "query by partition key") +
		row("/", "filter expression") +
		row("n", "new item") +
		row("e", "edit selected item") +
		row("d", "delete selected item")

	detail := header("DynamoDB — Detail") +
		row("↑ / ↓", "scroll") +
		row("y", "copy JSON to clipboard")

	content := title + "\n" + sep + "\n" +
		global + sep + "\n" +
		nav + sep + "\n" +
		dynamo + sep + "\n" +
		items + sep + "\n" +
		detail + "\n" +
		styleDimmed.Render("?  or  esc  to close")

	// Wrap the entire content in a full-width surface-coloured style so any
	// gaps between pre-rendered spans also get the correct background.
	inner := StyleModalInner.Background(bg).Width(54).Render(content)

	box := StyleModalBox.Background(bg).Padding(1, 3).Render(inner)

	return lipgloss.Place(width, height,
		lipgloss.Center, lipgloss.Center,
		box,
		lipgloss.WithWhitespaceBackground(ColorBg),
	)
}

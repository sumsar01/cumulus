package ui

import (
	"strings"

	"github.com/aws/aws-sdk-go-v2/aws"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/charmbracelet/lipgloss"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
)

// awsRegion is a display entry in the region picker.
type awsRegion struct {
	id       string // e.g. "eu-central-1"
	location string // e.g. "Frankfurt"
}

// knownRegions is the list shown in the region picker.
// Ordered geographically: US → CA → EU → AP → SA.
var knownRegions = []awsRegion{
	{"us-east-1", "N. Virginia"},
	{"us-east-2", "Ohio"},
	{"us-west-1", "N. California"},
	{"us-west-2", "Oregon"},
	{"ca-central-1", "Canada"},
	{"eu-west-1", "Ireland"},
	{"eu-west-2", "London"},
	{"eu-west-3", "Paris"},
	{"eu-central-1", "Frankfurt"},
	{"eu-north-1", "Stockholm"},
	{"ap-southeast-1", "Singapore"},
	{"ap-southeast-2", "Sydney"},
	{"ap-northeast-1", "Tokyo"},
	{"ap-northeast-2", "Seoul"},
	{"ap-south-1", "Mumbai"},
	{"sa-east-1", "São Paulo"},
}

// RegionPicker is a modal overlay for switching AWS regions.
type RegionPicker struct {
	cfg     aws.Config
	regions []awsRegion
	cursor  int
	current string // currently active region id
}

// NewRegionPicker constructs a RegionPicker pre-selected on currentRegion.
func NewRegionPicker(cfg aws.Config, currentRegion string) RegionPicker {
	p := RegionPicker{
		cfg:     cfg,
		regions: knownRegions,
		current: currentRegion,
	}
	for i, r := range knownRegions {
		if r.id == currentRegion {
			p.cursor = i
			break
		}
	}
	return p
}

// Update handles key events for the region picker.
// Returns (picker, cmd, done). done=true means the overlay should close.
func (p *RegionPicker) Update(msg tea.KeyMsg) (*RegionPicker, tea.Cmd, bool) {
	switch msg.String() {
	case "esc":
		return nil, nil, true
	case "up", "k":
		if p.cursor > 0 {
			p.cursor--
		}
	case "down", "j":
		if p.cursor < len(p.regions)-1 {
			p.cursor++
		}
	case "enter":
		if len(p.regions) == 0 {
			return p, nil, false
		}
		chosen := p.regions[p.cursor].id
		return nil, awspkg.SwitchRegionCmd(p.cfg, chosen), true
	}
	return p, nil, false
}

// View renders the region picker as a centred overlay box.
func (p *RegionPicker) View(width, height int) string {
	bg := ColorSurface

	title := StyleTitle.Background(bg).Render("switch region")
	hint := StyleDimmed.Background(bg).Render("↑/↓  navigate   enter  select   esc  cancel")
	sep := StyleDimmed.Background(bg).Render(strings.Repeat("─", 46))

	var rows strings.Builder
	for i, r := range p.regions {
		active := ""
		if r.id == p.current {
			active = StyleDimmed.Background(bg).Render(" ·")
		}
		loc := StyleDimmed.Background(bg).Render("  " + r.location)
		if i == p.cursor {
			rows.WriteString(
				StyleProfileCursorPrefix.Background(bg).Render("  › ") +
					StyleProfileSelectedName.Background(bg).Render(r.id) +
					loc + active + "\n",
			)
		} else {
			rows.WriteString(
				StyleDimmed.Background(bg).Render("    ") +
					StyleMuted.Background(bg).Render(r.id) +
					loc + active + "\n",
			)
		}
	}

	content := title + "\n" +
		sep + "\n" +
		rows.String() + "\n" +
		hint

	inner := StyleModalInner.Background(bg).Width(52).Render(content)
	box := StyleModalBox.Background(bg).Padding(1, 2).Render(inner)

	return lipgloss.Place(width, height,
		lipgloss.Center, lipgloss.Center,
		box,
		lipgloss.WithWhitespaceBackground(ColorBg),
	)
}

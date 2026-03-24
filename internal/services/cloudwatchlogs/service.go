// Package cloudwatchlogs implements the CloudWatch Logs service plugin for cumulus.
// It registers itself via the services registry and provides views for browsing
// log groups, log streams, and log events.
package cloudwatchlogs

import (
	"github.com/aws/aws-sdk-go-v2/aws"
	tea "github.com/charmbracelet/bubbletea"
)

// Svc is the CloudWatch Logs service plugin. Register it in main.go with:
//
//	services.Register(cloudwatchlogs.Svc{})
type Svc struct{}

func (Svc) Name() string        { return "CloudWatch Logs" }
func (Svc) ShortName() string   { return "cloudwatch-logs" }
func (Svc) Description() string { return "Browse log groups, streams and events" }
func (Svc) Icon() string        { return "◈" }

// Init returns the log-groups model as the entry point for the CloudWatch Logs service.
func (Svc) Init(cfg aws.Config) (tea.Model, tea.Cmd) {
	m := NewLogGroupsModel(cfg)
	return m, m.Init()
}

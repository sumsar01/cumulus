// Package dynamodb implements the DynamoDB service plugin for cumulus.
// It registers itself via the services registry and provides views for
// listing tables, scanning/querying items, viewing item detail, and
// add/edit/delete operations.
package dynamodb

import (
	"github.com/aws/aws-sdk-go-v2/aws"
	tea "github.com/charmbracelet/bubbletea"
	"github.com/sumsar01/cumulus/internal/config"
)

// Svc is the DynamoDB service plugin. Register it in main.go with:
//
//	services.Register(dynamodb.Svc{AppCfg: appCfg})
type Svc struct {
	AppCfg config.Config
}

func (Svc) Name() string        { return "DynamoDB" }
func (Svc) ShortName() string   { return "dynamodb" }
func (Svc) Description() string { return "Browse, query and edit DynamoDB tables and items" }
func (Svc) Icon() string        { return "⬡" }

// Init returns the tables-list model as the entry point for the DynamoDB service.
func (s Svc) Init(cfg aws.Config) (tea.Model, tea.Cmd) {
	m := NewTablesModel(cfg, s.AppCfg)
	return m, m.Init()
}

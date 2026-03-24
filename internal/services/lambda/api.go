package lambda

import (
	"context"
	"fmt"

	"github.com/aws/aws-sdk-go-v2/aws"
	lambdasdk "github.com/aws/aws-sdk-go-v2/service/lambda"
	lambdatypes "github.com/aws/aws-sdk-go-v2/service/lambda/types"
	tea "github.com/charmbracelet/bubbletea"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
)

// functionsLoadedMsg carries a page of Lambda functions from ListFunctions.
type functionsLoadedMsg struct {
	functions []lambdatypes.FunctionConfiguration
	nextToken *string // nil when there are no more pages
}

// functionDetailLoadedMsg carries the full GetFunction response for one function.
type functionDetailLoadedMsg struct {
	output *lambdasdk.GetFunctionOutput
}

// fetchFunctionsCmd fetches one page of Lambda functions (up to 50).
// If marker is non-nil the call continues from that pagination token.
func fetchFunctionsCmd(cfg aws.Config, marker *string) tea.Cmd {
	return func() tea.Msg {
		client := lambdasdk.NewFromConfig(cfg)
		input := &lambdasdk.ListFunctionsInput{
			MaxItems: aws.Int32(50),
		}
		if marker != nil {
			input.Marker = marker
		}
		out, err := client.ListFunctions(context.Background(), input)
		if err != nil {
			return awspkg.ErrMsg{Err: fmt.Errorf("fetchFunctions: %w", err)}
		}
		return functionsLoadedMsg{
			functions: out.Functions,
			nextToken: out.NextMarker,
		}
	}
}

// fetchFunctionDetailCmd fetches the full configuration for a single Lambda function.
func fetchFunctionDetailCmd(cfg aws.Config, functionName string) tea.Cmd {
	return func() tea.Msg {
		client := lambdasdk.NewFromConfig(cfg)
		out, err := client.GetFunction(context.Background(), &lambdasdk.GetFunctionInput{
			FunctionName: aws.String(functionName),
		})
		if err != nil {
			return awspkg.ErrMsg{Err: fmt.Errorf("fetchFunctionDetail: %w", err)}
		}
		return functionDetailLoadedMsg{output: out}
	}
}

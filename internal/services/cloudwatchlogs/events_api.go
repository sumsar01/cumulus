package cloudwatchlogs

import (
	"context"
	"fmt"
	"time"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/service/cloudwatchlogs"
	cwltypes "github.com/aws/aws-sdk-go-v2/service/cloudwatchlogs/types"
	tea "github.com/charmbracelet/bubbletea"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
)

// fetchLogGroupsCmd fetches all CloudWatch log groups via paginated DescribeLogGroups.
func fetchLogGroupsCmd(cfg aws.Config) tea.Cmd {
	return func() tea.Msg {
		client := cloudwatchlogs.NewFromConfig(cfg)
		ctx := context.Background()

		var groups []logGroup
		var nextToken *string

		for {
			out, err := client.DescribeLogGroups(ctx, &cloudwatchlogs.DescribeLogGroupsInput{
				NextToken: nextToken,
			})
			if err != nil {
				return awspkg.ErrMsg{Err: fmt.Errorf("DescribeLogGroups: %w", err)}
			}
			for _, g := range out.LogGroups {
				lg := logGroup{
					name:        aws.ToString(g.LogGroupName),
					storedBytes: aws.ToInt64(g.StoredBytes),
					kmsKeyID:    aws.ToString(g.KmsKeyId),
				}
				if g.RetentionInDays != nil {
					lg.retentionDays = *g.RetentionInDays
				}
				groups = append(groups, lg)
			}
			if out.NextToken == nil {
				break
			}
			nextToken = out.NextToken
		}

		return logGroupsLoadedMsg{groups: groups}
	}
}

// fetchLogStreamsCmd fetches log streams for a group.
//
// When prefix is empty, the first page (up to 50 streams) is returned ordered
// by LastEventTime descending so the most-recently-active streams appear first.
// Note: LastEventTime is eventually consistent and may lag up to ~1 hour, so
// very recently created or active streams may not yet rank at the top.
//
// When prefix is non-empty, streams whose names begin with prefix are returned
// ordered by LogStreamName (AWS does not allow LastEventTime ordering together
// with a name prefix). Only the first page is fetched in either case.
func fetchLogStreamsCmd(cfg aws.Config, groupName, prefix string) tea.Cmd {
	return func() tea.Msg {
		client := cloudwatchlogs.NewFromConfig(cfg)
		ctx := context.Background()

		input := &cloudwatchlogs.DescribeLogStreamsInput{
			LogGroupName: aws.String(groupName),
		}
		if prefix != "" {
			// LogStreamNamePrefix is incompatible with OrderBy:LastEventTime;
			// omitting OrderBy defaults to LogStreamName ordering.
			input.LogStreamNamePrefix = aws.String(prefix)
		} else {
			// Order by most-recently-active first (eventually consistent).
			input.OrderBy = cwltypes.OrderByLastEventTime
			input.Descending = aws.Bool(true)
		}

		out, err := client.DescribeLogStreams(ctx, input)
		if err != nil {
			return awspkg.ErrMsg{Err: fmt.Errorf("DescribeLogStreams %s: %w", groupName, err)}
		}

		streams := make([]logStream, 0, len(out.LogStreams))
		for _, s := range out.LogStreams {
			ls := logStream{
				name: aws.ToString(s.LogStreamName),
			}
			if s.LastEventTimestamp != nil {
				ls.lastEventTime = time.UnixMilli(*s.LastEventTimestamp)
			}
			streams = append(streams, ls)
		}

		return logStreamsLoadedMsg{streams: streams, prefix: prefix}
	}
}

// fetchLogEventsCmd fetches a page of log events from a stream.
// When filterPattern is non-empty it uses FilterLogEvents (cross-stream capable).
// token is the nextForwardToken / nextBackwardToken from a prior response;
// pass "" for the first load.
func fetchLogEventsCmd(cfg aws.Config, groupName, streamName, token, filterPattern string) tea.Cmd {
	return func() tea.Msg {
		client := cloudwatchlogs.NewFromConfig(cfg)
		ctx := context.Background()

		const limit = int32(100)

		if filterPattern != "" {
			// Use FilterLogEvents so the server applies the pattern.
			input := &cloudwatchlogs.FilterLogEventsInput{
				LogGroupName:   aws.String(groupName),
				LogStreamNames: []string{streamName},
				FilterPattern:  aws.String(filterPattern),
				Limit:          aws.Int32(limit),
			}
			if token != "" {
				input.NextToken = aws.String(token)
			}
			out, err := client.FilterLogEvents(ctx, input)
			if err != nil {
				return awspkg.ErrMsg{Err: fmt.Errorf("FilterLogEvents: %w", err)}
			}
			events := make([]logEvent, 0, len(out.Events))
			for _, e := range out.Events {
				ev := logEvent{message: aws.ToString(e.Message)}
				if e.Timestamp != nil {
					ev.timestamp = time.UnixMilli(*e.Timestamp)
				}
				if e.IngestionTime != nil {
					ev.ingested = time.UnixMilli(*e.IngestionTime)
				}
				events = append(events, ev)
			}
			nextTok := aws.ToString(out.NextToken)
			return logEventsLoadedMsg{
				events:    events,
				nextToken: nextTok,
				// FilterLogEvents has no backward token; treat as one-way
				prevToken: "",
			}
		}

		// Normal GetLogEvents with forward/backward paging.
		input := &cloudwatchlogs.GetLogEventsInput{
			LogGroupName:  aws.String(groupName),
			LogStreamName: aws.String(streamName),
			Limit:         aws.Int32(limit),
			StartFromHead: aws.Bool(false), // newest first
		}
		if token != "" {
			input.NextToken = aws.String(token)
		}
		out, err := client.GetLogEvents(ctx, input)
		if err != nil {
			return awspkg.ErrMsg{Err: fmt.Errorf("GetLogEvents: %w", err)}
		}

		events := make([]logEvent, 0, len(out.Events))
		for _, e := range out.Events {
			ev := logEvent{message: aws.ToString(e.Message)}
			if e.Timestamp != nil {
				ev.timestamp = time.UnixMilli(*e.Timestamp)
			}
			if e.IngestionTime != nil {
				ev.ingested = time.UnixMilli(*e.IngestionTime)
			}
			events = append(events, ev)
		}

		// GetLogEvents returns the same token when there are no more events,
		// so treat equal forward tokens as "no more pages".
		nextTok := aws.ToString(out.NextForwardToken)
		prevTok := aws.ToString(out.NextBackwardToken)
		if nextTok == token {
			nextTok = ""
		}

		return logEventsLoadedMsg{
			events:    events,
			nextToken: nextTok,
			prevToken: prevTok,
		}
	}
}

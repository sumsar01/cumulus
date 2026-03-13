package dynamodb

import (
	"context"
	"encoding/json"
	"fmt"

	"github.com/aws/aws-sdk-go-v2/aws"
	"github.com/aws/aws-sdk-go-v2/feature/dynamodb/attributevalue"
	"github.com/aws/aws-sdk-go-v2/service/dynamodb"
	ddbtypes "github.com/aws/aws-sdk-go-v2/service/dynamodb/types"
	tea "github.com/charmbracelet/bubbletea"
	awspkg "github.com/sumsar01/cumulus/internal/aws"
)

// fetchParams holds the parameters for a fetchItemsCmd call.
type fetchParams struct {
	cfg           aws.Config
	tableName     string
	mode          scanMode
	queryPK       string
	querySK       string
	filterExpr    string
	startKey      map[string]ddbtypes.AttributeValue
	cachedKeyInfo tableKeyInfo
}

// fetchItemsCmd fetches one page of items using Scan or Query.
func fetchItemsCmd(p fetchParams) tea.Cmd {
	return func() tea.Msg {
		client := dynamodb.NewFromConfig(p.cfg)
		ctx := context.Background()

		// Fetch key schema only on the first load; reuse the cached value afterward.
		ki := p.cachedKeyInfo
		if ki.pk == "" {
			var err error
			ki, err = describeTableKeys(ctx, client, p.tableName)
			if err != nil {
				return awspkg.ErrMsg{Err: err}
			}
		}

		var items []map[string]ddbtypes.AttributeValue
		var lastKey map[string]ddbtypes.AttributeValue
		var err error

		if p.mode == modeQuery && p.queryPK != "" {
			items, lastKey, err = runQuery(ctx, client, p.tableName, ki, p.queryPK, p.querySK, p.filterExpr, p.startKey)
		} else {
			items, lastKey, err = runScan(ctx, client, p.tableName, p.filterExpr, p.startKey)
		}
		if err != nil {
			return awspkg.ErrMsg{Err: err}
		}

		return itemsLoadedMsg{
			items:         items,
			lastKey:       lastKey,
			tableKeyNames: ki,
		}
	}
}

func describeTableKeys(ctx context.Context, client *dynamodb.Client, tableName string) (tableKeyInfo, error) {
	out, err := client.DescribeTable(ctx, &dynamodb.DescribeTableInput{
		TableName: aws.String(tableName),
	})
	if err != nil {
		return tableKeyInfo{}, fmt.Errorf("DescribeTable %s: %w", tableName, err)
	}
	ki := tableKeyInfo{}
	for _, ks := range out.Table.KeySchema {
		switch ks.KeyType {
		case ddbtypes.KeyTypeHash:
			ki.pk = aws.ToString(ks.AttributeName)
		case ddbtypes.KeyTypeRange:
			ki.sk = aws.ToString(ks.AttributeName)
		}
	}
	return ki, nil
}

func runScan(
	ctx context.Context,
	client *dynamodb.Client,
	tableName, filterExpr string,
	startKey map[string]ddbtypes.AttributeValue,
) ([]map[string]ddbtypes.AttributeValue, map[string]ddbtypes.AttributeValue, error) {
	input := &dynamodb.ScanInput{
		TableName: aws.String(tableName),
		Limit:     aws.Int32(50),
	}
	if len(startKey) > 0 {
		input.ExclusiveStartKey = startKey
	}
	// FilterExpression: user-supplied expression string is used as-is in the
	// ExpressionAttributeValues field — not string-interpolated into the query.
	if filterExpr != "" {
		input.FilterExpression = aws.String(filterExpr)
	}

	out, err := client.Scan(ctx, input)
	if err != nil {
		return nil, nil, fmt.Errorf("Scan %s: %w", tableName, err)
	}
	return out.Items, out.LastEvaluatedKey, nil
}

func runQuery(
	ctx context.Context,
	client *dynamodb.Client,
	tableName string,
	ki tableKeyInfo,
	pkValue, skValue, filterExpr string,
	startKey map[string]ddbtypes.AttributeValue,
) ([]map[string]ddbtypes.AttributeValue, map[string]ddbtypes.AttributeValue, error) {
	// Parameterised expression — user input goes into ExpressionAttributeValues,
	// never into the expression string itself.
	exprNames := map[string]string{"#pk": ki.pk}
	exprValues := map[string]ddbtypes.AttributeValue{
		":pkval": &ddbtypes.AttributeValueMemberS{Value: pkValue},
	}
	keyCondition := "#pk = :pkval"

	if skValue != "" && ki.sk != "" {
		exprNames["#sk"] = ki.sk
		exprValues[":skval"] = &ddbtypes.AttributeValueMemberS{Value: skValue}
		keyCondition += " AND #sk = :skval"
	}

	input := &dynamodb.QueryInput{
		TableName:                 aws.String(tableName),
		KeyConditionExpression:    aws.String(keyCondition),
		ExpressionAttributeNames:  exprNames,
		ExpressionAttributeValues: exprValues,
		Limit:                     aws.Int32(50),
	}
	if len(startKey) > 0 {
		input.ExclusiveStartKey = startKey
	}
	if filterExpr != "" {
		input.FilterExpression = aws.String(filterExpr)
	}

	out, err := client.Query(ctx, input)
	if err != nil {
		return nil, nil, fmt.Errorf("Query %s: %w", tableName, err)
	}
	return out.Items, out.LastEvaluatedKey, nil
}

// putItemCmd marshals the JSON data into DynamoDB AttributeValues and calls PutItem.
func putItemCmd(cfg aws.Config, tableName string, jsonData []byte) tea.Cmd {
	return func() tea.Msg {
		var generic map[string]interface{}
		if err := json.Unmarshal(jsonData, &generic); err != nil {
			return awspkg.ErrMsg{Err: fmt.Errorf("invalid JSON: %w", err)}
		}

		item, err := attributevalue.MarshalMap(generic)
		if err != nil {
			return awspkg.ErrMsg{Err: fmt.Errorf("marshalling item: %w", err)}
		}

		client := dynamodb.NewFromConfig(cfg)
		_, err = client.PutItem(context.Background(), &dynamodb.PutItemInput{
			TableName: aws.String(tableName),
			Item:      item,
		})
		if err != nil {
			return awspkg.ErrMsg{Err: fmt.Errorf("PutItem: %w", err)}
		}
		return itemSavedMsg{}
	}
}

// deleteItemCmd extracts the key from the item and calls DeleteItem.
func deleteItemCmd(
	cfg aws.Config,
	tableName string,
	item map[string]ddbtypes.AttributeValue,
	ki tableKeyInfo,
) tea.Cmd {
	return func() tea.Msg {
		key := map[string]ddbtypes.AttributeValue{
			ki.pk: item[ki.pk],
		}
		if ki.sk != "" {
			if skVal, ok := item[ki.sk]; ok {
				key[ki.sk] = skVal
			}
		}

		client := dynamodb.NewFromConfig(cfg)
		_, err := client.DeleteItem(context.Background(), &dynamodb.DeleteItemInput{
			TableName: aws.String(tableName),
			Key:       key,
		})
		if err != nil {
			return awspkg.ErrMsg{Err: fmt.Errorf("DeleteItem: %w", err)}
		}
		return itemDeletedMsg{}
	}
}

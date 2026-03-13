package dynamodb

import (
	"encoding/base64"
	"encoding/json"
	"fmt"

	ddbtypes "github.com/aws/aws-sdk-go-v2/service/dynamodb/types"
)

// attrValueString returns a compact human-readable representation of a
// DynamoDB AttributeValue for display in table cells.
func attrValueString(av ddbtypes.AttributeValue) string {
	switch v := av.(type) {
	case *ddbtypes.AttributeValueMemberS:
		return v.Value
	case *ddbtypes.AttributeValueMemberN:
		return v.Value
	case *ddbtypes.AttributeValueMemberBOOL:
		if v.Value {
			return "true"
		}
		return "false"
	case *ddbtypes.AttributeValueMemberNULL:
		return "null"
	case *ddbtypes.AttributeValueMemberL:
		return fmt.Sprintf("[…%d]", len(v.Value))
	case *ddbtypes.AttributeValueMemberM:
		return fmt.Sprintf("{…%d}", len(v.Value))
	case *ddbtypes.AttributeValueMemberSS:
		return fmt.Sprintf("SS(%d)", len(v.Value))
	case *ddbtypes.AttributeValueMemberNS:
		return fmt.Sprintf("NS(%d)", len(v.Value))
	case *ddbtypes.AttributeValueMemberBS:
		return fmt.Sprintf("BS(%d)", len(v.Value))
	case *ddbtypes.AttributeValueMemberB:
		return fmt.Sprintf("<binary %dB>", len(v.Value))
	default:
		return "?"
	}
}

// attrValueToGeneric converts a DynamoDB AttributeValue to a Go-native type
// suitable for JSON marshalling. All AttributeValue member types are handled,
// including MemberBS (binary set), which is encoded as a slice of base64 strings.
func attrValueToGeneric(av ddbtypes.AttributeValue) interface{} {
	switch v := av.(type) {
	case *ddbtypes.AttributeValueMemberS:
		return v.Value
	case *ddbtypes.AttributeValueMemberN:
		return json.Number(v.Value)
	case *ddbtypes.AttributeValueMemberBOOL:
		return v.Value
	case *ddbtypes.AttributeValueMemberNULL:
		return nil
	case *ddbtypes.AttributeValueMemberL:
		list := make([]interface{}, len(v.Value))
		for i, item := range v.Value {
			list[i] = attrValueToGeneric(item)
		}
		return list
	case *ddbtypes.AttributeValueMemberM:
		m := make(map[string]interface{}, len(v.Value))
		for k, val := range v.Value {
			m[k] = attrValueToGeneric(val)
		}
		return m
	case *ddbtypes.AttributeValueMemberSS:
		return v.Value
	case *ddbtypes.AttributeValueMemberNS:
		return v.Value
	case *ddbtypes.AttributeValueMemberB:
		return v.Value
	case *ddbtypes.AttributeValueMemberBS:
		// Encode each binary blob as a base64 string so the set is JSON-safe.
		strs := make([]string, len(v.Value))
		for i, b := range v.Value {
			strs[i] = base64.StdEncoding.EncodeToString(b)
		}
		return strs
	default:
		return fmt.Sprintf("<%T>", av)
	}
}

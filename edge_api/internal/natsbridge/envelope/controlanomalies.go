package envelope

import "google.golang.org/protobuf/encoding/protowire"

type ListAnomalyRulesRequest struct {
	CorrelationID string
	Paging        PagingRequest
	ScopeType     string
	ScopeID       string
}

type CreateAnomalyRuleRequest struct {
	CorrelationID string
	Name          string
	Kind          string
	ScopeType     string
	ScopeID       string
	ConfigJSON    string
	IsActive      bool
	Audit         AuditContext
}

type UpdateAnomalyRuleRequest struct {
	CorrelationID string
	AnomalyRuleID string
	Name          string
	ConfigJSON    string
	IsActive      bool
	Audit         AuditContext
}

type ListAnomalyInstancesRequest struct {
	CorrelationID string
	Paging        PagingRequest
	AnomalyRuleID string
	ClusterID     string
	Status        string
}

type ControlAnomalyRule struct {
	AnomalyRuleID string
	Name          string
	Kind          string
	ScopeType     string
	ScopeID       string
	ConfigJSON    string
	IsActive      bool
	CreatedAt     string
	UpdatedAt     string
	CreatedBy     string
	UpdatedBy     string
}

type ControlListAnomalyRulesResponse struct {
	Rules  []ControlAnomalyRule
	Paging PagingResponse
}

type ControlAnomalyInstance struct {
	AnomalyInstanceID string
	AnomalyRuleID     string
	ClusterID         string
	Severity          string
	Status            string
	StartedAt         string
	ResolvedAt        string
	PayloadJSON       string
}

type ControlListAnomalyInstancesResponse struct {
	Instances []ControlAnomalyInstance
	Paging    PagingResponse
}

func EncodeListAnomalyRulesRequest(request ListAnomalyRulesRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendMessageField(out, 2, encodePagingRequest(request.Paging))
	out = appendStringField(out, 3, request.ScopeType)
	out = appendStringField(out, 4, request.ScopeID)
	return out
}

func EncodeGetAnomalyRuleRequest(correlationID, anomalyRuleID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, anomalyRuleID)
	return out
}

func EncodeCreateAnomalyRuleRequest(request CreateAnomalyRuleRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.Name)
	out = appendStringField(out, 3, request.Kind)
	out = appendStringField(out, 4, request.ScopeType)
	out = appendStringField(out, 5, request.ScopeID)
	out = appendStringField(out, 6, request.ConfigJSON)
	out = appendBoolField(out, 7, request.IsActive)
	out = appendMessageField(out, 8, encodeAuditContext(request.Audit))
	return out
}

func EncodeUpdateAnomalyRuleRequest(request UpdateAnomalyRuleRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.AnomalyRuleID)
	out = appendStringField(out, 3, request.Name)
	out = appendStringField(out, 4, request.ConfigJSON)
	out = appendBoolField(out, 5, request.IsActive)
	out = appendMessageField(out, 6, encodeAuditContext(request.Audit))
	return out
}

func EncodeListAnomalyInstancesRequest(request ListAnomalyInstancesRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendMessageField(out, 2, encodePagingRequest(request.Paging))
	out = appendStringField(out, 3, request.AnomalyRuleID)
	out = appendStringField(out, 4, request.ClusterID)
	out = appendStringField(out, 5, request.Status)
	return out
}

func EncodeGetAnomalyInstanceRequest(correlationID, anomalyInstanceID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, anomalyInstanceID)
	return out
}

func DecodeControlAnomalyRule(data []byte) (ControlAnomalyRule, error) {
	var out ControlAnomalyRule
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.AnomalyRuleID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Name = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Kind = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ScopeType = value
		case 5:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ScopeID = value
		case 6:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ConfigJSON = value
		case 7:
			value, err := consumeBool(kind, raw)
			if err != nil {
				return err
			}
			out.IsActive = value
		case 8:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CreatedAt = value
		case 9:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.UpdatedAt = value
		case 10:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CreatedBy = value
		case 11:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.UpdatedBy = value
		}
		return nil
	})
	return out, err
}

func DecodeControlListAnomalyRulesResponse(data []byte) (ControlListAnomalyRulesResponse, error) {
	var out ControlListAnomalyRulesResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlAnomalyRule(value)
			if err != nil {
				return err
			}
			out.Rules = append(out.Rules, item)
		case 2:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			paging, err := DecodeControlPagingResponse(value)
			if err != nil {
				return err
			}
			out.Paging = paging
		}
		return nil
	})
	return out, err
}

func DecodeControlAnomalyInstance(data []byte) (ControlAnomalyInstance, error) {
	var out ControlAnomalyInstance
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.AnomalyInstanceID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.AnomalyRuleID = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ClusterID = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Severity = value
		case 5:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Status = value
		case 6:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.StartedAt = value
		case 7:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ResolvedAt = value
		case 8:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.PayloadJSON = value
		}
		return nil
	})
	return out, err
}

func DecodeControlListAnomalyInstancesResponse(data []byte) (ControlListAnomalyInstancesResponse, error) {
	var out ControlListAnomalyInstancesResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlAnomalyInstance(value)
			if err != nil {
				return err
			}
			out.Instances = append(out.Instances, item)
		case 2:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			paging, err := DecodeControlPagingResponse(value)
			if err != nil {
				return err
			}
			out.Paging = paging
		}
		return nil
	})
	return out, err
}

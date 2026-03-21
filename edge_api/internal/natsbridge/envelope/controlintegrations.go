package envelope

import "google.golang.org/protobuf/encoding/protowire"

type ListIntegrationsRequest struct {
	CorrelationID string
	Paging        PagingRequest
}

type CreateIntegrationRequest struct {
	CorrelationID string
	Name          string
	Kind          string
	Description   string
	ConfigJSON    string
	IsActive      bool
	Audit         AuditContext
}

type UpdateIntegrationRequest struct {
	CorrelationID string
	IntegrationID string
	Name          string
	Description   string
	ConfigJSON    string
	IsActive      bool
	Audit         AuditContext
}

type BindIntegrationRequest struct {
	CorrelationID     string
	IntegrationID     string
	ScopeType         string
	ScopeID           string
	EventTypesJSON    string
	SeverityThreshold string
	IsActive          bool
	Audit             AuditContext
}

type UnbindIntegrationRequest struct {
	CorrelationID        string
	IntegrationBindingID string
	Audit                AuditContext
}

type ControlIntegration struct {
	IntegrationID string
	Name          string
	Kind          string
	Description   string
	ConfigJSON    string
	IsActive      bool
	CreatedAt     string
	UpdatedAt     string
	CreatedBy     string
	UpdatedBy     string
}

type ControlIntegrationBinding struct {
	IntegrationBindingID string
	IntegrationID        string
	ScopeType            string
	ScopeID              string
	EventTypesJSON       string
	SeverityThreshold    string
	IsActive             bool
	CreatedAt            string
	UpdatedAt            string
}

type ControlListIntegrationsResponse struct {
	Integrations []ControlIntegration
	Paging       PagingResponse
}

type ControlGetIntegrationResponse struct {
	Integration *ControlIntegration
	Bindings    []ControlIntegrationBinding
}

func EncodeListIntegrationsRequest(request ListIntegrationsRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendMessageField(out, 2, encodePagingRequest(request.Paging))
	return out
}

func EncodeGetIntegrationRequest(correlationID, integrationID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, integrationID)
	return out
}

func EncodeCreateIntegrationRequest(request CreateIntegrationRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.Name)
	out = appendStringField(out, 3, request.Kind)
	out = appendStringField(out, 4, request.Description)
	out = appendStringField(out, 5, request.ConfigJSON)
	out = appendBoolField(out, 6, request.IsActive)
	out = appendMessageField(out, 7, encodeAuditContext(request.Audit))
	return out
}

func EncodeUpdateIntegrationRequest(request UpdateIntegrationRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.IntegrationID)
	out = appendStringField(out, 3, request.Name)
	out = appendStringField(out, 4, request.Description)
	out = appendStringField(out, 5, request.ConfigJSON)
	out = appendBoolField(out, 6, request.IsActive)
	out = appendMessageField(out, 7, encodeAuditContext(request.Audit))
	return out
}

func EncodeBindIntegrationRequest(request BindIntegrationRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.IntegrationID)
	out = appendStringField(out, 3, request.ScopeType)
	out = appendStringField(out, 4, request.ScopeID)
	out = appendStringField(out, 5, request.EventTypesJSON)
	out = appendStringField(out, 6, request.SeverityThreshold)
	out = appendBoolField(out, 7, request.IsActive)
	out = appendMessageField(out, 8, encodeAuditContext(request.Audit))
	return out
}

func EncodeUnbindIntegrationRequest(request UnbindIntegrationRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.IntegrationBindingID)
	out = appendMessageField(out, 3, encodeAuditContext(request.Audit))
	return out
}

func DecodeControlIntegration(data []byte) (ControlIntegration, error) {
	var out ControlIntegration
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.IntegrationID = value
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
			out.Description = value
		case 5:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ConfigJSON = value
		case 6:
			value, err := consumeBool(kind, raw)
			if err != nil {
				return err
			}
			out.IsActive = value
		case 7:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CreatedAt = value
		case 8:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.UpdatedAt = value
		case 9:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CreatedBy = value
		case 10:
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

func DecodeControlIntegrationBinding(data []byte) (ControlIntegrationBinding, error) {
	var out ControlIntegrationBinding
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.IntegrationBindingID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.IntegrationID = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ScopeType = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ScopeID = value
		case 5:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.EventTypesJSON = value
		case 6:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.SeverityThreshold = value
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
		}
		return nil
	})
	return out, err
}

func DecodeControlListIntegrationsResponse(data []byte) (ControlListIntegrationsResponse, error) {
	var out ControlListIntegrationsResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlIntegration(value)
			if err != nil {
				return err
			}
			out.Integrations = append(out.Integrations, item)
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

func DecodeControlGetIntegrationResponse(data []byte) (ControlGetIntegrationResponse, error) {
	var out ControlGetIntegrationResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlIntegration(value)
			if err != nil {
				return err
			}
			out.Integration = &item
		case 2:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlIntegrationBinding(value)
			if err != nil {
				return err
			}
			out.Bindings = append(out.Bindings, item)
		}
		return nil
	})
	return out, err
}

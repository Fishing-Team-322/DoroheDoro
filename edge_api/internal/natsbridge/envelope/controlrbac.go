package envelope

import "google.golang.org/protobuf/encoding/protowire"

type ListRolesRequest struct {
	CorrelationID string
	Paging        PagingRequest
}

type CreateRoleRequest struct {
	CorrelationID string
	Name          string
	Slug          string
	Description   string
	Audit         AuditContext
}

type UpdateRoleRequest struct {
	CorrelationID string
	RoleID        string
	Name          string
	Description   string
	Audit         AuditContext
}

type SetRolePermissionsRequest struct {
	CorrelationID   string
	RoleID          string
	PermissionCodes []string
	Audit           AuditContext
}

type ListRoleBindingsRequest struct {
	CorrelationID string
	UserID        string
	RoleID        string
	ScopeType     string
	ScopeID       string
	Paging        PagingRequest
}

type CreateRoleBindingRequest struct {
	CorrelationID string
	UserID        string
	RoleID        string
	ScopeType     string
	ScopeID       string
	Audit         AuditContext
}

type DeleteRoleBindingRequest struct {
	CorrelationID string
	RoleBindingID string
	Audit         AuditContext
}

type ControlPermission struct {
	PermissionID string
	Code         string
	Description  string
}

type ControlRole struct {
	RoleID      string
	Name        string
	Slug        string
	Description string
	IsSystem    bool
	CreatedAt   string
	UpdatedAt   string
	CreatedBy   string
	UpdatedBy   string
}

type ControlListRolesResponse struct {
	Roles  []ControlRole
	Paging PagingResponse
}

type ControlRolePermissionsResponse struct {
	Role        *ControlRole
	Permissions []ControlPermission
}

type ControlRoleBinding struct {
	RoleBindingID string
	UserID        string
	RoleID        string
	ScopeType     string
	ScopeID       string
	CreatedAt     string
}

type ControlListRoleBindingsResponse struct {
	Bindings []ControlRoleBinding
	Paging   PagingResponse
}

func EncodeListRolesRequest(request ListRolesRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendMessageField(out, 2, encodePagingRequest(request.Paging))
	return out
}

func EncodeGetRoleRequest(correlationID, roleID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, roleID)
	return out
}

func EncodeCreateRoleRequest(request CreateRoleRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.Name)
	out = appendStringField(out, 3, request.Slug)
	out = appendStringField(out, 4, request.Description)
	out = appendMessageField(out, 5, encodeAuditContext(request.Audit))
	return out
}

func EncodeUpdateRoleRequest(request UpdateRoleRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.RoleID)
	out = appendStringField(out, 3, request.Name)
	out = appendStringField(out, 4, request.Description)
	out = appendMessageField(out, 5, encodeAuditContext(request.Audit))
	return out
}

func EncodeGetRolePermissionsRequest(correlationID, roleID string) []byte {
	return EncodeGetRoleRequest(correlationID, roleID)
}

func EncodeSetRolePermissionsRequest(request SetRolePermissionsRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.RoleID)
	out = appendRepeatedStringField(out, 3, request.PermissionCodes)
	out = appendMessageField(out, 4, encodeAuditContext(request.Audit))
	return out
}

func EncodeListRoleBindingsRequest(request ListRoleBindingsRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.UserID)
	out = appendStringField(out, 3, request.RoleID)
	out = appendStringField(out, 4, request.ScopeType)
	out = appendStringField(out, 5, request.ScopeID)
	out = appendMessageField(out, 6, encodePagingRequest(request.Paging))
	return out
}

func EncodeCreateRoleBindingRequest(request CreateRoleBindingRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.UserID)
	out = appendStringField(out, 3, request.RoleID)
	out = appendStringField(out, 4, request.ScopeType)
	out = appendStringField(out, 5, request.ScopeID)
	out = appendMessageField(out, 6, encodeAuditContext(request.Audit))
	return out
}

func EncodeDeleteRoleBindingRequest(request DeleteRoleBindingRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.RoleBindingID)
	out = appendMessageField(out, 3, encodeAuditContext(request.Audit))
	return out
}

func DecodeControlPermission(data []byte) (ControlPermission, error) {
	var out ControlPermission
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.PermissionID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Code = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Description = value
		}
		return nil
	})
	return out, err
}

func DecodeControlRole(data []byte) (ControlRole, error) {
	var out ControlRole
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.RoleID = value
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
			out.Slug = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Description = value
		case 5:
			value, err := consumeBool(kind, raw)
			if err != nil {
				return err
			}
			out.IsSystem = value
		case 6:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CreatedAt = value
		case 7:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.UpdatedAt = value
		case 8:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CreatedBy = value
		case 9:
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

func DecodeControlListRolesResponse(data []byte) (ControlListRolesResponse, error) {
	var out ControlListRolesResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlRole(value)
			if err != nil {
				return err
			}
			out.Roles = append(out.Roles, item)
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

func DecodeControlGetRolePermissionsResponse(data []byte) (ControlRolePermissionsResponse, error) {
	var out ControlRolePermissionsResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlRole(value)
			if err != nil {
				return err
			}
			out.Role = &item
		case 2:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlPermission(value)
			if err != nil {
				return err
			}
			out.Permissions = append(out.Permissions, item)
		}
		return nil
	})
	return out, err
}

func DecodeControlRoleBinding(data []byte) (ControlRoleBinding, error) {
	var out ControlRoleBinding
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.RoleBindingID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.UserID = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.RoleID = value
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
			out.CreatedAt = value
		}
		return nil
	})
	return out, err
}

func DecodeControlListRoleBindingsResponse(data []byte) (ControlListRoleBindingsResponse, error) {
	var out ControlListRoleBindingsResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlRoleBinding(value)
			if err != nil {
				return err
			}
			out.Bindings = append(out.Bindings, item)
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

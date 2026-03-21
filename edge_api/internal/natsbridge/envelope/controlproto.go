package envelope

import "google.golang.org/protobuf/encoding/protowire"

type ControlReplyEnvelope struct {
	Status        string
	Code          string
	Message       string
	Payload       []byte
	CorrelationID string
}

type ControlPolicy struct {
	PolicyID         string
	Name             string
	Description      string
	IsActive         bool
	LatestRevisionID string
	LatestRevision   string
	PolicyBodyJSON   string
	CreatedAt        string
	UpdatedAt        string
}

type ControlPolicyRevision struct {
	PolicyRevisionID string
	PolicyID         string
	Revision         string
	PolicyBodyJSON   string
	CreatedAt        string
}

type ControlListPoliciesResponse struct {
	Policies []ControlPolicy
}

type HostInput struct {
	Hostname   string
	IP         string
	SSHPort    uint32
	RemoteUser string
	Labels     map[string]string
}

type ControlHost struct {
	HostID     string
	Hostname   string
	IP         string
	SSHPort    uint32
	RemoteUser string
	Labels     map[string]string
	CreatedAt  string
	UpdatedAt  string
}

type ControlListHostsResponse struct {
	Hosts []ControlHost
}

type ControlHostGroupMember struct {
	HostGroupMemberID string
	HostGroupID       string
	HostID            string
	Hostname          string
}

type ControlHostGroup struct {
	HostGroupID string
	Name        string
	Description string
	CreatedAt   string
	UpdatedAt   string
	Members     []ControlHostGroupMember
}

type ControlListHostGroupsResponse struct {
	Groups []ControlHostGroup
}

type ControlCredentialProfileMetadata struct {
	CredentialsProfileID string
	Name                 string
	Kind                 string
	Description          string
	VaultRef             string
	CreatedAt            string
	UpdatedAt            string
}

type ControlListCredentialsResponse struct {
	Profiles []ControlCredentialProfileMetadata
}

func DecodeControlReplyEnvelope(data []byte) (ControlReplyEnvelope, error) {
	var out ControlReplyEnvelope
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Status = value
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
			out.Message = value
		case 4:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			out.Payload = value
		case 5:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CorrelationID = value
		}
		return nil
	})
	return out, err
}

func EncodeControlListPoliciesRequest(correlationID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	return out
}

func EncodeControlGetPolicyRequest(correlationID, policyID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, policyID)
	return out
}

func EncodeControlCreatePolicyRequest(correlationID, name, description, policyBodyJSON string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, name)
	out = appendStringField(out, 3, description)
	out = appendStringField(out, 4, policyBodyJSON)
	return out
}

func EncodeControlUpdatePolicyRequest(correlationID, policyID, description, policyBodyJSON string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, policyID)
	out = appendStringField(out, 3, description)
	out = appendStringField(out, 4, policyBodyJSON)
	return out
}

func EncodeControlGetPolicyRevisionsRequest(correlationID, policyID string) []byte {
	return EncodeControlGetPolicyRequest(correlationID, policyID)
}

func EncodeControlListHostsRequest(correlationID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	return out
}

func EncodeControlGetHostRequest(correlationID, hostID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, hostID)
	return out
}

func EncodeControlCreateHostRequest(correlationID string, host HostInput) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendMessageField(out, 2, encodeHostInput(host))
	return out
}

func EncodeControlUpdateHostRequest(correlationID, hostID string, host HostInput) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, hostID)
	out = appendMessageField(out, 3, encodeHostInput(host))
	return out
}

func EncodeControlListHostGroupsRequest(correlationID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	return out
}

func EncodeControlGetHostGroupRequest(correlationID, hostGroupID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, hostGroupID)
	return out
}

func EncodeControlCreateHostGroupRequest(correlationID, name, description string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, name)
	out = appendStringField(out, 3, description)
	return out
}

func EncodeControlUpdateHostGroupRequest(correlationID, hostGroupID, name, description string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, hostGroupID)
	out = appendStringField(out, 3, name)
	out = appendStringField(out, 4, description)
	return out
}

func EncodeControlAddHostGroupMemberRequest(correlationID, hostGroupID, hostID string, audit AuditContext) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, hostGroupID)
	out = appendStringField(out, 3, hostID)
	out = appendMessageField(out, 4, encodeAuditContext(audit))
	return out
}

func EncodeControlRemoveHostGroupMemberRequest(correlationID, hostGroupID, hostID string, audit AuditContext) []byte {
	return EncodeControlAddHostGroupMemberRequest(correlationID, hostGroupID, hostID, audit)
}

func EncodeControlListCredentialsRequest(correlationID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	return out
}

func EncodeControlGetCredentialsRequest(correlationID, credentialsProfileID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, credentialsProfileID)
	return out
}

func EncodeControlCreateCredentialsRequest(correlationID, name, kind, description, vaultRef string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, name)
	out = appendStringField(out, 3, kind)
	out = appendStringField(out, 4, description)
	out = appendStringField(out, 5, vaultRef)
	return out
}

func encodeHostInput(host HostInput) []byte {
	var out []byte
	out = appendStringField(out, 1, host.Hostname)
	out = appendStringField(out, 2, host.IP)
	out = appendUint32Field(out, 3, host.SSHPort)
	out = appendStringField(out, 4, host.RemoteUser)
	for key, value := range host.Labels {
		out = appendStringMapEntry(out, 5, key, value)
	}
	return out
}

func DecodeControlPolicy(data []byte) (ControlPolicy, error) {
	var out ControlPolicy
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.PolicyID = value
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
			out.Description = value
		case 4:
			value, err := consumeBool(kind, raw)
			if err != nil {
				return err
			}
			out.IsActive = value
		case 5:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.LatestRevisionID = value
		case 6:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.LatestRevision = value
		case 7:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.PolicyBodyJSON = value
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

func DecodeControlListPoliciesResponse(data []byte) (ControlListPoliciesResponse, error) {
	var out ControlListPoliciesResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		if num != 1 {
			return nil
		}
		value, err := consumeBytes(kind, raw)
		if err != nil {
			return err
		}
		item, err := DecodeControlPolicy(value)
		if err != nil {
			return err
		}
		out.Policies = append(out.Policies, item)
		return nil
	})
	return out, err
}

func DecodeControlPolicyRevision(data []byte) (ControlPolicyRevision, error) {
	var out ControlPolicyRevision
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.PolicyRevisionID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.PolicyID = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Revision = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.PolicyBodyJSON = value
		case 5:
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

func DecodeControlGetPolicyRevisionsResponse(data []byte) ([]ControlPolicyRevision, error) {
	out := make([]ControlPolicyRevision, 0)
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		if num != 1 {
			return nil
		}
		value, err := consumeBytes(kind, raw)
		if err != nil {
			return err
		}
		item, err := DecodeControlPolicyRevision(value)
		if err != nil {
			return err
		}
		out = append(out, item)
		return nil
	})
	return out, err
}

func DecodeControlHost(data []byte) (ControlHost, error) {
	var out ControlHost
	out.Labels = map[string]string{}
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.HostID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Hostname = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.IP = value
		case 4:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.SSHPort = value
		case 5:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.RemoteUser = value
		case 6:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			key, mapValue, err := decodeStringMapEntry(value)
			if err != nil {
				return err
			}
			out.Labels[key] = mapValue
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
		}
		return nil
	})
	return out, err
}

func DecodeControlListHostsResponse(data []byte) (ControlListHostsResponse, error) {
	var out ControlListHostsResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		if num != 1 {
			return nil
		}
		value, err := consumeBytes(kind, raw)
		if err != nil {
			return err
		}
		item, err := DecodeControlHost(value)
		if err != nil {
			return err
		}
		out.Hosts = append(out.Hosts, item)
		return nil
	})
	return out, err
}

func DecodeControlHostGroupMember(data []byte) (ControlHostGroupMember, error) {
	var out ControlHostGroupMember
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.HostGroupMemberID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.HostGroupID = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.HostID = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Hostname = value
		}
		return nil
	})
	return out, err
}

func DecodeControlHostGroup(data []byte) (ControlHostGroup, error) {
	var out ControlHostGroup
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.HostGroupID = value
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
			out.Description = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CreatedAt = value
		case 5:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.UpdatedAt = value
		case 6:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			member, err := DecodeControlHostGroupMember(value)
			if err != nil {
				return err
			}
			out.Members = append(out.Members, member)
		}
		return nil
	})
	return out, err
}

func DecodeControlListHostGroupsResponse(data []byte) (ControlListHostGroupsResponse, error) {
	var out ControlListHostGroupsResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		if num != 1 {
			return nil
		}
		value, err := consumeBytes(kind, raw)
		if err != nil {
			return err
		}
		item, err := DecodeControlHostGroup(value)
		if err != nil {
			return err
		}
		out.Groups = append(out.Groups, item)
		return nil
	})
	return out, err
}

func DecodeControlCredentialProfileMetadata(data []byte) (ControlCredentialProfileMetadata, error) {
	var out ControlCredentialProfileMetadata
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CredentialsProfileID = value
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
			out.VaultRef = value
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
		}
		return nil
	})
	return out, err
}

func DecodeControlListCredentialsResponse(data []byte) (ControlListCredentialsResponse, error) {
	var out ControlListCredentialsResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		if num != 1 {
			return nil
		}
		value, err := consumeBytes(kind, raw)
		if err != nil {
			return err
		}
		item, err := DecodeControlCredentialProfileMetadata(value)
		if err != nil {
			return err
		}
		out.Profiles = append(out.Profiles, item)
		return nil
	})
	return out, err
}

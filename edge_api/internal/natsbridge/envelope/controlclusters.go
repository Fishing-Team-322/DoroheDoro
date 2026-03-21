package envelope

import "google.golang.org/protobuf/encoding/protowire"

type ListClustersRequest struct {
	CorrelationID  string
	Paging         PagingRequest
	Query          string
	HostID         string
	IncludeMembers bool
}

type CreateClusterRequest struct {
	CorrelationID string
	Name          string
	Slug          string
	Description   string
	IsActive      bool
	MetadataJSON  string
	Audit         AuditContext
}

type UpdateClusterRequest struct {
	CorrelationID string
	ClusterID     string
	Name          string
	Slug          string
	Description   string
	IsActive      bool
	MetadataJSON  string
	Audit         AuditContext
}

type ClusterHostMutationRequest struct {
	CorrelationID string
	ClusterID     string
	HostID        string
	Audit         AuditContext
}

type ControlCluster struct {
	ClusterID    string
	Name         string
	Slug         string
	Description  string
	IsActive     bool
	CreatedAt    string
	UpdatedAt    string
	CreatedBy    string
	UpdatedBy    string
	MetadataJSON string
	HostCount    uint32
	AgentCount   uint32
}

type ControlClusterHostBinding struct {
	ClusterHostID string
	HostID        string
	Hostname      string
	CreatedAt     string
}

type ControlClusterAgentBinding struct {
	ClusterAgentID string
	AgentID        string
	CreatedAt      string
}

type ControlClusterDetails struct {
	Cluster ControlCluster
	Hosts   []ControlClusterHostBinding
	Agents  []ControlClusterAgentBinding
}

type ControlListClustersResponse struct {
	Clusters []ControlCluster
	Paging   PagingResponse
}

func EncodeListClustersRequest(request ListClustersRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendMessageField(out, 2, encodePagingRequest(request.Paging))
	out = appendStringField(out, 3, request.Query)
	out = appendStringField(out, 4, request.HostID)
	out = appendBoolField(out, 5, request.IncludeMembers)
	return out
}

func EncodeGetClusterRequest(correlationID, clusterID string, includeMembers bool) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, clusterID)
	out = appendBoolField(out, 3, includeMembers)
	return out
}

func EncodeCreateClusterRequest(request CreateClusterRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.Name)
	out = appendStringField(out, 3, request.Slug)
	out = appendStringField(out, 4, request.Description)
	out = appendBoolField(out, 5, request.IsActive)
	out = appendStringField(out, 6, request.MetadataJSON)
	out = appendMessageField(out, 7, encodeAuditContext(request.Audit))
	return out
}

func EncodeUpdateClusterRequest(request UpdateClusterRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.ClusterID)
	out = appendStringField(out, 3, request.Name)
	out = appendStringField(out, 4, request.Slug)
	out = appendStringField(out, 5, request.Description)
	out = appendBoolField(out, 6, request.IsActive)
	out = appendStringField(out, 7, request.MetadataJSON)
	out = appendMessageField(out, 8, encodeAuditContext(request.Audit))
	return out
}

func EncodeClusterHostMutationRequest(request ClusterHostMutationRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.ClusterID)
	out = appendStringField(out, 3, request.HostID)
	out = appendMessageField(out, 4, encodeAuditContext(request.Audit))
	return out
}

func DecodeControlCluster(data []byte) (ControlCluster, error) {
	var out ControlCluster
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ClusterID = value
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
			out.IsActive = value
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
		case 10:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.MetadataJSON = value
		case 11:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.HostCount = value
		case 12:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.AgentCount = value
		}
		return nil
	})
	return out, err
}

func DecodeControlClusterHostBinding(data []byte) (ControlClusterHostBinding, error) {
	var out ControlClusterHostBinding
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ClusterHostID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.HostID = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Hostname = value
		case 4:
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

func DecodeControlClusterAgentBinding(data []byte) (ControlClusterAgentBinding, error) {
	var out ControlClusterAgentBinding
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ClusterAgentID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.AgentID = value
		case 3:
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

func DecodeControlClusterDetails(data []byte) (ControlClusterDetails, error) {
	var out ControlClusterDetails
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlCluster(value)
			if err != nil {
				return err
			}
			out.Cluster = item
		case 2:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlClusterHostBinding(value)
			if err != nil {
				return err
			}
			out.Hosts = append(out.Hosts, item)
		case 3:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlClusterAgentBinding(value)
			if err != nil {
				return err
			}
			out.Agents = append(out.Agents, item)
		}
		return nil
	})
	return out, err
}

func DecodeControlListClustersResponse(data []byte) (ControlListClustersResponse, error) {
	var out ControlListClustersResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlCluster(value)
			if err != nil {
				return err
			}
			out.Clusters = append(out.Clusters, item)
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

func DecodeControlGetClusterResponse(data []byte) (ControlClusterDetails, error) {
	var out ControlClusterDetails
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		if num != 1 {
			return nil
		}
		value, err := consumeBytes(kind, raw)
		if err != nil {
			return err
		}
		item, err := DecodeControlClusterDetails(value)
		if err != nil {
			return err
		}
		out = item
		return nil
	})
	return out, err
}

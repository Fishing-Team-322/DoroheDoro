package envelope

import (
	"encoding/json"

	"google.golang.org/protobuf/encoding/protowire"
)

type DeploymentReplyEnvelope struct {
	Status        string
	Code          string
	Message       string
	Payload       []byte
	CorrelationID string
}

type CreateDeploymentSpec struct {
	JobType             int32
	PolicyID            string
	TargetHostIDs       []string
	TargetHostGroupIDs  []string
	CredentialProfileID string
	RequestedBy         string
	PreserveState       bool
	Force               bool
	DryRun              bool
}

type ListDeploymentJobsRequest struct {
	CorrelationID string
	Status        int32
	JobType       int32
	RequestedBy   string
	CreatedAfter  string
	CreatedBefore string
	Limit         uint32
	Offset        uint64
}

type RetryDeploymentJobRequest struct {
	CorrelationID string
	JobID         string
	Strategy      int32
	TriggeredBy   string
	Reason        string
}

type CancelDeploymentJobRequest struct {
	CorrelationID string
	JobID         string
	RequestedBy   string
	Reason        string
}

type DeploymentJobSummary struct {
	JobID               string
	JobType             int32
	Status              int32
	RequestedBy         string
	PolicyID            string
	PolicyRevisionID    string
	CredentialProfileID string
	ExecutorKind        int32
	CurrentPhase        string
	TotalTargets        uint32
	PendingTargets      uint32
	RunningTargets      uint32
	SucceededTargets    uint32
	FailedTargets       uint32
	CancelledTargets    uint32
	AttemptCount        uint32
	CreatedAt           string
	StartedAt           string
	FinishedAt          string
	UpdatedAt           string
}

type DeploymentAttemptSummary struct {
	DeploymentAttemptID string
	AttemptNo           uint32
	Status              int32
	TriggeredBy         string
	Reason              string
	CreatedAt           string
	StartedAt           string
	FinishedAt          string
}

type DeploymentTargetSummary struct {
	DeploymentTargetID  string
	DeploymentAttemptID string
	HostID              string
	HostnameSnapshot    string
	Status              int32
	ErrorMessage        string
	CreatedAt           string
	StartedAt           string
	FinishedAt          string
	UpdatedAt           string
}

type DeploymentStepSummary struct {
	DeploymentStepID    string
	DeploymentAttemptID string
	DeploymentTargetID  string
	StepName            string
	Status              int32
	Message             string
	PayloadJSON         string
	CreatedAt           string
	UpdatedAt           string
}

type DeploymentPlanTarget struct {
	HostID     string
	Hostname   string
	IP         string
	SSHPort    uint32
	RemoteUser string
}

type BootstrapPreview struct {
	HostID        string
	Hostname      string
	BootstrapYAML string
}

type CreateDeploymentJobResponse struct {
	Job *DeploymentJobSummary
}

type GetDeploymentJobResponse struct {
	Job      *DeploymentJobSummary
	Attempts []DeploymentAttemptSummary
	Targets  []DeploymentTargetSummary
	Steps    []DeploymentStepSummary
}

type ListDeploymentJobsResponse struct {
	Jobs   []DeploymentJobSummary
	Limit  uint32
	Offset uint64
	Total  uint64
}

type RetryDeploymentJobResponse struct {
	Job *DeploymentJobSummary
}

type CancelDeploymentJobResponse struct {
	Job *DeploymentJobSummary
}

type CreateDeploymentPlanResponse struct {
	JobType             int32
	PolicyID            string
	PolicyRevisionID    string
	PolicyRevision      string
	CredentialProfileID string
	CredentialSummary   string
	ExecutorKind        int32
	ActionSummary       string
	Targets             []DeploymentPlanTarget
	BootstrapPreviews   []BootstrapPreview
	Warnings            []string
}

type DeploymentStatusEvent struct {
	JobID               string
	DeploymentAttemptID string
	Status              int32
	CurrentPhase        string
	PendingTargets      uint32
	RunningTargets      uint32
	SucceededTargets    uint32
	FailedTargets       uint32
	CancelledTargets    uint32
	UpdatedAt           string
}

type DeploymentStepEvent struct {
	JobID               string
	DeploymentAttemptID string
	DeploymentStepID    string
	DeploymentTargetID  string
	StepName            string
	Status              int32
	Message             string
	UpdatedAt           string
}

func DecodeDeploymentReplyEnvelope(data []byte) (DeploymentReplyEnvelope, error) {
	var out DeploymentReplyEnvelope
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

func EncodeCreateDeploymentJobRequest(correlationID string, spec CreateDeploymentSpec) []byte {
	return encodeCreateDeploymentSpec(1, correlationID, spec)
}

func EncodeGetDeploymentJobRequest(correlationID, jobID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, jobID)
	return out
}

func EncodeListDeploymentJobsRequest(request ListDeploymentJobsRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendInt32Field(out, 2, request.Status)
	out = appendInt32Field(out, 3, request.JobType)
	out = appendStringField(out, 4, request.RequestedBy)
	out = appendStringField(out, 5, request.CreatedAfter)
	out = appendStringField(out, 6, request.CreatedBefore)
	out = appendUint32Field(out, 7, request.Limit)
	out = appendUint64Field(out, 8, request.Offset)
	return out
}

func EncodeRetryDeploymentJobRequest(request RetryDeploymentJobRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.JobID)
	out = appendInt32Field(out, 3, request.Strategy)
	out = appendStringField(out, 4, request.TriggeredBy)
	out = appendStringField(out, 5, request.Reason)
	return out
}

func EncodeCancelDeploymentJobRequest(request CancelDeploymentJobRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.JobID)
	out = appendStringField(out, 3, request.RequestedBy)
	out = appendStringField(out, 4, request.Reason)
	return out
}

func EncodeCreateDeploymentPlanRequest(correlationID string, spec CreateDeploymentSpec) []byte {
	return encodeCreateDeploymentSpec(1, correlationID, spec)
}

func encodeCreateDeploymentSpec(correlationField protowire.Number, correlationID string, spec CreateDeploymentSpec) []byte {
	var out []byte
	out = appendStringField(out, correlationField, correlationID)
	out = appendInt32Field(out, 2, spec.JobType)
	out = appendStringField(out, 3, spec.PolicyID)
	out = appendRepeatedStringField(out, 4, spec.TargetHostIDs)
	out = appendRepeatedStringField(out, 5, spec.TargetHostGroupIDs)
	out = appendStringField(out, 6, spec.CredentialProfileID)
	out = appendStringField(out, 7, spec.RequestedBy)
	out = appendBoolField(out, 8, spec.PreserveState)
	out = appendBoolField(out, 9, spec.Force)
	out = appendBoolField(out, 10, spec.DryRun)
	return out
}

func appendInt32Field(dst []byte, num protowire.Number, value int32) []byte {
	if value == 0 {
		return dst
	}
	dst = protowire.AppendTag(dst, num, protowire.VarintType)
	return protowire.AppendVarint(dst, uint64(uint32(value)))
}

func DecodeDeploymentJobSummary(data []byte) (DeploymentJobSummary, error) {
	var out DeploymentJobSummary
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.JobID = value
		case 2:
			value, err := consumeInt32(kind, raw)
			if err != nil {
				return err
			}
			out.JobType = value
		case 3:
			value, err := consumeInt32(kind, raw)
			if err != nil {
				return err
			}
			out.Status = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.RequestedBy = value
		case 5:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.PolicyID = value
		case 6:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.PolicyRevisionID = value
		case 7:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CredentialProfileID = value
		case 8:
			value, err := consumeInt32(kind, raw)
			if err != nil {
				return err
			}
			out.ExecutorKind = value
		case 9:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CurrentPhase = value
		case 10:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.TotalTargets = value
		case 11:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.PendingTargets = value
		case 12:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.RunningTargets = value
		case 13:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.SucceededTargets = value
		case 14:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.FailedTargets = value
		case 15:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.CancelledTargets = value
		case 16:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.AttemptCount = value
		case 17:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CreatedAt = value
		case 18:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.StartedAt = value
		case 19:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.FinishedAt = value
		case 20:
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

func DecodeDeploymentAttemptSummary(data []byte) (DeploymentAttemptSummary, error) {
	var out DeploymentAttemptSummary
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.DeploymentAttemptID = value
		case 2:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.AttemptNo = value
		case 3:
			value, err := consumeInt32(kind, raw)
			if err != nil {
				return err
			}
			out.Status = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.TriggeredBy = value
		case 5:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Reason = value
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
			out.StartedAt = value
		case 8:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.FinishedAt = value
		}
		return nil
	})
	return out, err
}

func DecodeDeploymentTargetSummary(data []byte) (DeploymentTargetSummary, error) {
	var out DeploymentTargetSummary
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.DeploymentTargetID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.DeploymentAttemptID = value
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
			out.HostnameSnapshot = value
		case 5:
			value, err := consumeInt32(kind, raw)
			if err != nil {
				return err
			}
			out.Status = value
		case 6:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ErrorMessage = value
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
			out.StartedAt = value
		case 9:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.FinishedAt = value
		case 10:
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

func DecodeDeploymentStepSummary(data []byte) (DeploymentStepSummary, error) {
	var out DeploymentStepSummary
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.DeploymentStepID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.DeploymentAttemptID = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.DeploymentTargetID = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.StepName = value
		case 5:
			value, err := consumeInt32(kind, raw)
			if err != nil {
				return err
			}
			out.Status = value
		case 6:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Message = value
		case 7:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.PayloadJSON = value
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

func DecodeDeploymentPlanTarget(data []byte) (DeploymentPlanTarget, error) {
	var out DeploymentPlanTarget
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
		}
		return nil
	})
	return out, err
}

func DecodeBootstrapPreview(data []byte) (BootstrapPreview, error) {
	var out BootstrapPreview
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
			out.BootstrapYAML = value
		}
		return nil
	})
	return out, err
}

func DecodeCreateDeploymentJobResponse(data []byte) (CreateDeploymentJobResponse, error) {
	var out CreateDeploymentJobResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		if num != 1 {
			return nil
		}
		value, err := consumeBytes(kind, raw)
		if err != nil {
			return err
		}
		item, err := DecodeDeploymentJobSummary(value)
		if err != nil {
			return err
		}
		out.Job = &item
		return nil
	})
	return out, err
}

func DecodeGetDeploymentJobResponse(data []byte) (GetDeploymentJobResponse, error) {
	var out GetDeploymentJobResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		value, err := consumeBytes(kind, raw)
		if err != nil {
			return err
		}
		switch num {
		case 1:
			item, err := DecodeDeploymentJobSummary(value)
			if err != nil {
				return err
			}
			out.Job = &item
		case 2:
			item, err := DecodeDeploymentAttemptSummary(value)
			if err != nil {
				return err
			}
			out.Attempts = append(out.Attempts, item)
		case 3:
			item, err := DecodeDeploymentTargetSummary(value)
			if err != nil {
				return err
			}
			out.Targets = append(out.Targets, item)
		case 4:
			item, err := DecodeDeploymentStepSummary(value)
			if err != nil {
				return err
			}
			out.Steps = append(out.Steps, item)
		}
		return nil
	})
	return out, err
}

func DecodeListDeploymentJobsResponse(data []byte) (ListDeploymentJobsResponse, error) {
	var out ListDeploymentJobsResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeDeploymentJobSummary(value)
			if err != nil {
				return err
			}
			out.Jobs = append(out.Jobs, item)
		case 2:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.Limit = value
		case 3:
			value, err := consumeUint64(kind, raw)
			if err != nil {
				return err
			}
			out.Offset = value
		case 4:
			value, err := consumeUint64(kind, raw)
			if err != nil {
				return err
			}
			out.Total = value
		}
		return nil
	})
	return out, err
}

func DecodeRetryDeploymentJobResponse(data []byte) (RetryDeploymentJobResponse, error) {
	created, err := DecodeCreateDeploymentJobResponse(data)
	if err != nil {
		return RetryDeploymentJobResponse{}, err
	}
	return RetryDeploymentJobResponse{Job: created.Job}, nil
}

func DecodeCancelDeploymentJobResponse(data []byte) (CancelDeploymentJobResponse, error) {
	created, err := DecodeCreateDeploymentJobResponse(data)
	if err != nil {
		return CancelDeploymentJobResponse{}, err
	}
	return CancelDeploymentJobResponse{Job: created.Job}, nil
}

func DecodeCreateDeploymentPlanResponse(data []byte) (CreateDeploymentPlanResponse, error) {
	var out CreateDeploymentPlanResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeInt32(kind, raw)
			if err != nil {
				return err
			}
			out.JobType = value
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
			out.PolicyRevisionID = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.PolicyRevision = value
		case 5:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CredentialProfileID = value
		case 6:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CredentialSummary = value
		case 7:
			value, err := consumeInt32(kind, raw)
			if err != nil {
				return err
			}
			out.ExecutorKind = value
		case 8:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ActionSummary = value
		case 9:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeDeploymentPlanTarget(value)
			if err != nil {
				return err
			}
			out.Targets = append(out.Targets, item)
		case 10:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeBootstrapPreview(value)
			if err != nil {
				return err
			}
			out.BootstrapPreviews = append(out.BootstrapPreviews, item)
		case 11:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Warnings = append(out.Warnings, value)
		}
		return nil
	})
	return out, err
}

func DecodeDeploymentStatusEvent(data []byte) (DeploymentStatusEvent, error) {
	var out DeploymentStatusEvent
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.JobID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.DeploymentAttemptID = value
		case 3:
			value, err := consumeInt32(kind, raw)
			if err != nil {
				return err
			}
			out.Status = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CurrentPhase = value
		case 5:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.PendingTargets = value
		case 6:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.RunningTargets = value
		case 7:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.SucceededTargets = value
		case 8:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.FailedTargets = value
		case 9:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.CancelledTargets = value
		case 10:
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

func DecodeDeploymentStepEvent(data []byte) (DeploymentStepEvent, error) {
	var out DeploymentStepEvent
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.JobID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.DeploymentAttemptID = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.DeploymentStepID = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.DeploymentTargetID = value
		case 5:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.StepName = value
		case 6:
			value, err := consumeInt32(kind, raw)
			if err != nil {
				return err
			}
			out.Status = value
		case 7:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Message = value
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

func DecodeDeploymentStatusEventJSON(data []byte) ([]byte, error) {
	event, err := DecodeDeploymentStatusEvent(data)
	if err != nil {
		return nil, err
	}
	return json.Marshal(map[string]any{
		"job_id":                event.JobID,
		"deployment_attempt_id": event.DeploymentAttemptID,
		"status":                deploymentJobStatusString(event.Status),
		"current_phase":         event.CurrentPhase,
		"pending_targets":       event.PendingTargets,
		"running_targets":       event.RunningTargets,
		"succeeded_targets":     event.SucceededTargets,
		"failed_targets":        event.FailedTargets,
		"cancelled_targets":     event.CancelledTargets,
		"updated_at":            event.UpdatedAt,
	})
}

func DecodeDeploymentStepEventJSON(data []byte) ([]byte, error) {
	event, err := DecodeDeploymentStepEvent(data)
	if err != nil {
		return nil, err
	}
	return json.Marshal(map[string]any{
		"job_id":                event.JobID,
		"deployment_attempt_id": event.DeploymentAttemptID,
		"deployment_step_id":    event.DeploymentStepID,
		"deployment_target_id":  event.DeploymentTargetID,
		"step_name":             event.StepName,
		"status":                deploymentStepStatusString(event.Status),
		"message":               event.Message,
		"updated_at":            event.UpdatedAt,
	})
}

func deploymentJobTypeString(value int32) string {
	switch value {
	case 1:
		return "install"
	case 2:
		return "reinstall"
	case 3:
		return "upgrade"
	case 4:
		return "uninstall"
	default:
		return "unspecified"
	}
}

func deploymentJobStatusString(value int32) string {
	switch value {
	case 1:
		return "queued"
	case 2:
		return "running"
	case 3:
		return "partial_success"
	case 4:
		return "succeeded"
	case 5:
		return "failed"
	case 6:
		return "cancelled"
	default:
		return "unspecified"
	}
}

func deploymentTargetStatusString(value int32) string {
	switch value {
	case 1:
		return "pending"
	case 2:
		return "running"
	case 3:
		return "succeeded"
	case 4:
		return "failed"
	case 5:
		return "cancelled"
	default:
		return "unspecified"
	}
}

func deploymentStepStatusString(value int32) string {
	switch value {
	case 1:
		return "pending"
	case 2:
		return "running"
	case 3:
		return "succeeded"
	case 4:
		return "failed"
	case 5:
		return "skipped"
	default:
		return "unspecified"
	}
}

func deploymentExecutorKindString(value int32) string {
	switch value {
	case 1:
		return "mock"
	case 2:
		return "ansible"
	default:
		return "unspecified"
	}
}

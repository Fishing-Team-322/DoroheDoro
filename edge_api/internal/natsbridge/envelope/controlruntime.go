package envelope

import "google.golang.org/protobuf/encoding/protowire"

type PagingRequest struct {
	Limit  uint32
	Offset uint64
	Query  string
}

type PagingResponse struct {
	Limit  uint32
	Offset uint64
	Total  uint64
}

type AuditContext struct {
	ActorID   string
	ActorType string
	RequestID string
	Reason    string
}

func DecodeControlPagingResponse(data []byte) (PagingResponse, error) {
	var out PagingResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeUint32(kind, raw)
			if err != nil {
				return err
			}
			out.Limit = value
		case 2:
			value, err := consumeUint64(kind, raw)
			if err != nil {
				return err
			}
			out.Offset = value
		case 3:
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

func encodePagingRequest(request PagingRequest) []byte {
	var out []byte
	out = appendUint32Field(out, 1, request.Limit)
	out = appendUint64Field(out, 2, request.Offset)
	out = appendStringField(out, 3, request.Query)
	return out
}

func encodeAuditContext(audit AuditContext) []byte {
	var out []byte
	out = appendStringField(out, 1, audit.ActorID)
	out = appendStringField(out, 2, audit.ActorType)
	out = appendStringField(out, 3, audit.RequestID)
	out = appendStringField(out, 4, audit.Reason)
	return out
}

package envelope

import "google.golang.org/protobuf/encoding/protowire"

type ListTicketsRequest struct {
	CorrelationID  string
	Paging         PagingRequest
	ClusterID      string
	Status         string
	Severity       string
	AssigneeUserID string
}

type CreateTicketRequest struct {
	CorrelationID string
	Title         string
	Description   string
	ClusterID     string
	SourceType    string
	SourceID      string
	Severity      string
	Audit         AuditContext
}

type AssignTicketRequest struct {
	CorrelationID  string
	TicketID       string
	AssigneeUserID string
	Audit          AuditContext
}

type UnassignTicketRequest struct {
	CorrelationID string
	TicketID      string
	Audit         AuditContext
}

type AddTicketCommentRequest struct {
	CorrelationID string
	TicketID      string
	Body          string
	Audit         AuditContext
}

type ChangeTicketStatusRequest struct {
	CorrelationID string
	TicketID      string
	Status        string
	Resolution    string
	Audit         AuditContext
}

type CloseTicketRequest struct {
	CorrelationID string
	TicketID      string
	Resolution    string
	Audit         AuditContext
}

type ControlTicket struct {
	TicketID       string
	TicketKey      string
	Title          string
	Description    string
	ClusterID      string
	ClusterName    string
	SourceType     string
	SourceID       string
	Severity       string
	Status         string
	AssigneeUserID string
	CreatedBy      string
	Resolution     string
	CreatedAt      string
	UpdatedAt      string
	ResolvedAt     string
	ClosedAt       string
}

type ControlTicketComment struct {
	TicketCommentID string
	TicketID        string
	AuthorUserID    string
	Body            string
	CreatedAt       string
}

type ControlTicketEvent struct {
	TicketEventID string
	TicketID      string
	EventType     string
	PayloadJSON   string
	CreatedAt     string
}

type ControlTicketDetails struct {
	Ticket   *ControlTicket
	Comments []ControlTicketComment
	Events   []ControlTicketEvent
}

type ControlListTicketsResponse struct {
	Tickets []ControlTicket
	Paging  PagingResponse
}

func EncodeListTicketsRequest(request ListTicketsRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendMessageField(out, 2, encodePagingRequest(request.Paging))
	out = appendStringField(out, 3, request.ClusterID)
	out = appendStringField(out, 4, request.Status)
	out = appendStringField(out, 5, request.Severity)
	out = appendStringField(out, 6, request.AssigneeUserID)
	return out
}

func EncodeGetTicketRequest(correlationID, ticketID string) []byte {
	var out []byte
	out = appendStringField(out, 1, correlationID)
	out = appendStringField(out, 2, ticketID)
	return out
}

func EncodeCreateTicketRequest(request CreateTicketRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.Title)
	out = appendStringField(out, 3, request.Description)
	out = appendStringField(out, 4, request.ClusterID)
	out = appendStringField(out, 5, request.SourceType)
	out = appendStringField(out, 6, request.SourceID)
	out = appendStringField(out, 7, request.Severity)
	out = appendMessageField(out, 8, encodeAuditContext(request.Audit))
	return out
}

func EncodeAssignTicketRequest(request AssignTicketRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.TicketID)
	out = appendStringField(out, 3, request.AssigneeUserID)
	out = appendMessageField(out, 4, encodeAuditContext(request.Audit))
	return out
}

func EncodeUnassignTicketRequest(request UnassignTicketRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.TicketID)
	out = appendMessageField(out, 3, encodeAuditContext(request.Audit))
	return out
}

func EncodeAddTicketCommentRequest(request AddTicketCommentRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.TicketID)
	out = appendStringField(out, 3, request.Body)
	out = appendMessageField(out, 4, encodeAuditContext(request.Audit))
	return out
}

func EncodeChangeTicketStatusRequest(request ChangeTicketStatusRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.TicketID)
	out = appendStringField(out, 3, request.Status)
	out = appendStringField(out, 4, request.Resolution)
	out = appendMessageField(out, 5, encodeAuditContext(request.Audit))
	return out
}

func EncodeCloseTicketRequest(request CloseTicketRequest) []byte {
	var out []byte
	out = appendStringField(out, 1, request.CorrelationID)
	out = appendStringField(out, 2, request.TicketID)
	out = appendStringField(out, 3, request.Resolution)
	out = appendMessageField(out, 4, encodeAuditContext(request.Audit))
	return out
}

func DecodeControlTicket(data []byte) (ControlTicket, error) {
	var out ControlTicket
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.TicketID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.TicketKey = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Title = value
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
			out.ClusterID = value
		case 6:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ClusterName = value
		case 7:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.SourceType = value
		case 8:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.SourceID = value
		case 9:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Severity = value
		case 10:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Status = value
		case 11:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.AssigneeUserID = value
		case 12:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CreatedBy = value
		case 13:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Resolution = value
		case 14:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.CreatedAt = value
		case 15:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.UpdatedAt = value
		case 16:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ResolvedAt = value
		case 17:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.ClosedAt = value
		}
		return nil
	})
	return out, err
}

func DecodeControlTicketComment(data []byte) (ControlTicketComment, error) {
	var out ControlTicketComment
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.TicketCommentID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.TicketID = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.AuthorUserID = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.Body = value
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

func DecodeControlTicketEvent(data []byte) (ControlTicketEvent, error) {
	var out ControlTicketEvent
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.TicketEventID = value
		case 2:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.TicketID = value
		case 3:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.EventType = value
		case 4:
			value, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			out.PayloadJSON = value
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

func DecodeControlTicketDetails(data []byte) (ControlTicketDetails, error) {
	var out ControlTicketDetails
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlTicket(value)
			if err != nil {
				return err
			}
			out.Ticket = &item
		case 2:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlTicketComment(value)
			if err != nil {
				return err
			}
			out.Comments = append(out.Comments, item)
		case 3:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlTicketEvent(value)
			if err != nil {
				return err
			}
			out.Events = append(out.Events, item)
		}
		return nil
	})
	return out, err
}

func DecodeControlListTicketsResponse(data []byte) (ControlListTicketsResponse, error) {
	var out ControlListTicketsResponse
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			value, err := consumeBytes(kind, raw)
			if err != nil {
				return err
			}
			item, err := DecodeControlTicket(value)
			if err != nil {
				return err
			}
			out.Tickets = append(out.Tickets, item)
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

func DecodeControlGetTicketResponse(data []byte) (ControlTicketDetails, error) {
	var out ControlTicketDetails
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		if num != 1 {
			return nil
		}
		value, err := consumeBytes(kind, raw)
		if err != nil {
			return err
		}
		item, err := DecodeControlTicketDetails(value)
		if err != nil {
			return err
		}
		out = item
		return nil
	})
	return out, err
}

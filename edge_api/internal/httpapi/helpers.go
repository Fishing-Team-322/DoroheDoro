package httpapi

import (
	"context"
	"encoding/json"
	"errors"
	"net/http"

	"go.uber.org/zap"

	"github.com/example/dorohedoro/internal/middleware"
	"github.com/example/dorohedoro/internal/natsbridge"
	"github.com/example/dorohedoro/internal/natsbridge/envelope"
)

func decodeJSONBody(r *http.Request, dst any) error {
	defer r.Body.Close()
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(dst); err != nil {
		return err
	}
	if decoder.More() {
		return errors.New("multiple JSON documents are not allowed")
	}
	return nil
}

func requestJSONEnvelope[T any](
	ctx context.Context,
	bridge *natsbridge.Bridge,
	logger *zap.Logger,
	subject string,
	request any,
) (T, envelope.AgentReplyEnvelope, error) {
	var zero T

	requestBytes, err := json.Marshal(request)
	if err != nil {
		return zero, envelope.AgentReplyEnvelope{}, err
	}

	replyMsg, err := bridge.Request(ctx, subject, requestBytes)
	if err != nil {
		return zero, envelope.AgentReplyEnvelope{}, err
	}

	reply, err := envelope.DecodeAgentReplyEnvelope(replyMsg.Data)
	if err != nil {
		if logger != nil {
			logger.Error("decode upstream reply envelope failed",
				zap.String("subject", subject),
				zap.String("request_id", middleware.GetRequestID(ctx)),
				zap.Error(err),
			)
		}
		return zero, envelope.AgentReplyEnvelope{}, err
	}
	if reply.Status == "error" {
		return zero, reply, nil
	}
	if len(reply.Payload) == 0 {
		return zero, reply, nil
	}
	if err := json.Unmarshal(reply.Payload, &zero); err != nil {
		if logger != nil {
			logger.Error("decode upstream json payload failed",
				zap.String("subject", subject),
				zap.String("request_id", middleware.GetRequestID(ctx)),
				zap.Error(err),
			)
		}
		return zero, reply, err
	}
	return zero, reply, nil
}

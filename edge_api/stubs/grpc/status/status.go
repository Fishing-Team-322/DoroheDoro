package status

import (
	"fmt"

	"google.golang.org/grpc/codes"
)

type StatusError struct {
	CodeValue codes.Code
	Message   string
}

func (e *StatusError) Error() string { return e.Message }
func Error(code codes.Code, message string) error {
	return &StatusError{CodeValue: code, Message: message}
}
func Errorf(code codes.Code, format string, a ...any) error {
	return &StatusError{CodeValue: code, Message: fmt.Sprintf(format, a...)}
}
func Code(err error) codes.Code {
	if err == nil {
		return codes.OK
	}
	if se, ok := err.(*StatusError); ok {
		return se.CodeValue
	}
	return codes.Unknown
}

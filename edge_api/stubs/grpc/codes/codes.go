package codes

type Code int

const (
	OK Code = iota
	Canceled
	Unknown
	InvalidArgument
	DeadlineExceeded
	NotFound
	AlreadyExists
	PermissionDenied
	ResourceExhausted
	FailedPrecondition
	Aborted
	OutOfRange
	Unimplemented
	Internal
	Unavailable
	DataLoss
	Unauthenticated
)

func (c Code) String() string {
	switch c {
	case InvalidArgument:
		return "InvalidArgument"
	case PermissionDenied:
		return "PermissionDenied"
	case Unauthenticated:
		return "Unauthenticated"
	case Unavailable:
		return "Unavailable"
	case Internal:
		return "Internal"
	default:
		return "Unknown"
	}
}

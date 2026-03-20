package zapcore

type Level int8

const (
	DebugLevel Level = -1
	InfoLevel  Level = 0
	WarnLevel  Level = 1
	ErrorLevel Level = 2
)

type PrimitiveArrayEncoder interface{ AppendString(string) }

type TimeEncoder func(any, PrimitiveArrayEncoder)

var ISO8601TimeEncoder TimeEncoder = func(any, PrimitiveArrayEncoder) {}

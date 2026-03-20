package zap

import (
	"fmt"
	"log"
	"time"

	"go.uber.org/zap/zapcore"
)

type Field struct {
	Key   string
	Value any
}

type Logger struct{}

const (
	DebugLevel = zapcore.DebugLevel
	InfoLevel  = zapcore.InfoLevel
	WarnLevel  = zapcore.WarnLevel
	ErrorLevel = zapcore.ErrorLevel
)

type AtomicLevel struct{ level zapcore.Level }

type Config struct {
	Encoding      string
	EncoderConfig EncoderConfig
	Level         AtomicLevel
}

type EncoderConfig struct {
	TimeKey    string
	EncodeTime zapcore.TimeEncoder
}

func NewProductionConfig() Config                      { return Config{} }
func NewAtomicLevelAt(level zapcore.Level) AtomicLevel { return AtomicLevel{level: level} }
func (c Config) Build(options ...any) (*Logger, error) { return &Logger{}, nil }
func (l *Logger) Sync() error                          { return nil }
func (l *Logger) Info(msg string, fields ...Field)     { log.Println(format("INFO", msg, fields...)) }
func (l *Logger) Error(msg string, fields ...Field)    { log.Println(format("ERROR", msg, fields...)) }
func (l *Logger) Warn(msg string, fields ...Field)     { log.Println(format("WARN", msg, fields...)) }
func String(key, value string) Field                   { return Field{Key: key, Value: value} }
func Int(key string, value int) Field                  { return Field{Key: key, Value: value} }
func Bool(key string, value bool) Field                { return Field{Key: key, Value: value} }
func Duration(key string, value time.Duration) Field   { return Field{Key: key, Value: value} }
func Any(key string, value any) Field                  { return Field{Key: key, Value: value} }
func ByteString(key string, value []byte) Field        { return Field{Key: key, Value: string(value)} }
func Error(err error) Field {
	if err == nil {
		return Field{Key: "error", Value: nil}
	}
	return Field{Key: "error", Value: err.Error()}
}

func format(level, msg string, fields ...Field) string {
	out := level + " " + msg
	for _, f := range fields {
		out += fmt.Sprintf(" %s=%v", f.Key, f.Value)
	}
	return out
}

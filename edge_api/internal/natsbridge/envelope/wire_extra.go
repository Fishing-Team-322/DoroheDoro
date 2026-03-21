package envelope

import "google.golang.org/protobuf/encoding/protowire"

func appendUint32Field(dst []byte, num protowire.Number, value uint32) []byte {
	if value == 0 {
		return dst
	}
	dst = protowire.AppendTag(dst, num, protowire.VarintType)
	return protowire.AppendVarint(dst, uint64(value))
}

func appendUint64Field(dst []byte, num protowire.Number, value uint64) []byte {
	if value == 0 {
		return dst
	}
	dst = protowire.AppendTag(dst, num, protowire.VarintType)
	return protowire.AppendVarint(dst, value)
}

func appendBoolField(dst []byte, num protowire.Number, value bool) []byte {
	if !value {
		return dst
	}
	dst = protowire.AppendTag(dst, num, protowire.VarintType)
	return protowire.AppendVarint(dst, 1)
}

func appendMessageField(dst []byte, num protowire.Number, value []byte) []byte {
	return appendBytesField(dst, num, value)
}

func appendRepeatedStringField(dst []byte, num protowire.Number, values []string) []byte {
	for _, value := range values {
		dst = appendStringField(dst, num, value)
	}
	return dst
}

func appendStringMapEntry(dst []byte, num protowire.Number, key, value string) []byte {
	var entry []byte
	entry = appendStringField(entry, 1, key)
	entry = appendStringField(entry, 2, value)
	return appendBytesField(dst, num, entry)
}

func consumeUint32(kind protowire.Type, value []byte) (uint32, error) {
	decoded, err := consumeVarint(kind, value)
	return uint32(decoded), err
}

func consumeUint64(kind protowire.Type, value []byte) (uint64, error) {
	return consumeVarint(kind, value)
}

func consumeInt32(kind protowire.Type, value []byte) (int32, error) {
	decoded, err := consumeVarint(kind, value)
	return int32(decoded), err
}

func consumeBool(kind protowire.Type, value []byte) (bool, error) {
	decoded, err := consumeVarint(kind, value)
	return decoded != 0, err
}

func consumeVarint(kind protowire.Type, value []byte) (uint64, error) {
	if kind != protowire.VarintType {
		return 0, protowire.ParseError(int(kind))
	}
	decoded, n := protowire.ConsumeVarint(value)
	if n < 0 {
		return 0, protowire.ParseError(n)
	}
	return decoded, nil
}

func decodeStringMapEntry(data []byte) (string, string, error) {
	var key, value string
	err := walkFields(data, func(num protowire.Number, kind protowire.Type, raw []byte) error {
		switch num {
		case 1:
			decoded, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			key = decoded
		case 2:
			decoded, err := consumeString(kind, raw)
			if err != nil {
				return err
			}
			value = decoded
		}
		return nil
	})
	return key, value, err
}

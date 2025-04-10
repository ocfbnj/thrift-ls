include "ThriftTes.thrift"

struc InvalidStruct {
  1: required string name
}

enum MissingBraceEnum {
  VALUE1 = 1
  VALUE2 = 2

struct MissingType {
  1: name
}

service MissingBraceService
  void ping()

struct InvalidModifier {
  1: invalid string name
}

const i32 missing_equal 42

struct InvalidContainer {
  1: optional list<string, string> invalid_map
}

typedef map<string string> StringMap

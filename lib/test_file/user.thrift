include "./base.thrift"
// cpp_include "./base.thrift"

namespace go cn.ocfbnj.user
namespace java cn.ocfbnj.user

const double PI = 3.14159265358979323846;
typedef i32 int32

// Create User Request
struct CreateUserRequest {
    1: required string name;
    2: required i32 age;
    3: required common.Gender gender;
    4: optional list<string> emails;
    5: optional map<string, string> phones;
    6: optional set<string> hobbies;
}

// Create User Response
struct CreateUserResponse {
    255: base.BaseResp base_resp;
}

struct CreateUserRespData {
    1: i64 id;
    2: bool is_success;
    3: optional FilaedReason reason;
}

enum FilaedReason {
    NAME_EMPTY = 1,
    AGE_ILLEGAL = 2,
    GENDER_ILLEGAL = 3,
}

    /* tod
    o */ // line comment

    /*
    /*
    nested block comment
    */
    */

service UserService {
    CreateUserResponse CreateUser(1: CreateUserRequest req) (api.category="user")
}

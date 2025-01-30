#[derive(Debug, Clone, PartialEq)]
pub enum Syntax {
    Proto2,
    Proto3,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Reserved {
    Number(i32),
    Range(i32, i32),
    FieldName(String),
}

// Examples for possible OptionValue enum members:
// Identifier: option foo = bar;
// String: option foo = "bar";
// Int: option foo = 42;
// Float: option foo = 3.14;
// Bool: option foo = true;
// List: option foo = [1, 2, 3];
// Map: option foo = {key: "value", another_key: 123};
// Enum: option foo = ENUM_VALUE;
// Message: option (google.api.http) = {
//     get: "/v1/messages/{message_id}"
//     additional_bindings {
//         get: "/v1/users/{user_id}/messages/{message_id}"
//     }
// };
#[derive(Debug, Clone, PartialEq)]
pub enum OptionValue {
    Identifier(String),
    String(String),
    DecimalInt(i64),
    Float(f64),
    Octal(i64),
    Hex(i64),
    Bool(bool),
    List(Vec<OptionValue>),
    Map(Vec<(OptionValue, OptionValue)>),
    Enum(String, String),                // (enum type, enum value)
    Message(Vec<(String, OptionValue)>), // For nested message options
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtoOption {
    pub name: String,
    pub value: OptionValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProtoFile {
    pub syntax: Syntax,
    pub package: Option<String>,
    pub imports: Vec<Import>,
    pub options: Vec<ProtoOption>,
    pub messages: Vec<Message>,
    pub enums: Vec<Enum>,
    pub services: Vec<Service>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub path: String,
    pub kind: ImportKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ImportKind {
    Default,
    Public,
    Weak,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Message {
    pub name: String,
    pub fields: Vec<Field>,
    pub oneofs: Vec<OneOf>,
    pub nested_messages: Vec<Message>,
    pub nested_enums: Vec<Enum>,
    pub options: Vec<ProtoOption>,
    pub reserved: Vec<Reserved>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NumberValue {
    DecimalInt(i64),
    Octal(i64),
    Hex(i64),
    Float(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: String,
    pub number: NumberValue,
    pub label: FieldLabel,
    pub typ: FieldType,
    pub options: Vec<ProtoOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldLabel {
    Optional,
    Required,
    Repeated,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Double,
    Float,
    Int32,
    Int64,
    UInt32,
    UInt64,
    SInt32,
    SInt64,
    Fixed32,
    Fixed64,
    SFixed32,
    SFixed64,
    Bool,
    String,
    Bytes,
    MessageOrEnum(String),
    Map(Box<FieldType>, Box<FieldType>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct OneOf {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub name: String,
    pub values: Vec<EnumValue>,
    pub options: Vec<EnumValueOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumValue {
    pub name: String,
    pub number: NumberValue,
    pub options: Vec<EnumValueOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumValueOption {
    pub name: String,
    pub value: EnumValueOptionValue,
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumValueOptionValue {
    String(String),
    DecimalInt(i64),
    Octal(i64),
    Hex(i64),
    Float(f64),
    Bool(bool),
    Identifier(String), // For referencing other enum values or custom identifiers
}

impl EnumValueOption {
    pub fn new(name: String, value: EnumValueOptionValue) -> Self {
        EnumValueOption { name, value }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Service {
    pub name: String,
    pub methods: Vec<Method>,
    pub options: Vec<ProtoOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Method {
    pub name: String,
    pub input_type: String,
    pub output_type: String,
    pub client_streaming: bool,
    pub server_streaming: bool,
    pub options: Vec<ProtoOption>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReservedRange {
    pub start: i32,
    pub end: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Constant {
    Int(i64),
    Float(f64),
    Bool(bool),
    String(String),
    Identifier(String),
}

impl ProtoFile {
    pub fn new() -> Self {
        ProtoFile {
            syntax: Syntax::Proto3, // Default to Proto3
            package: None,
            imports: Vec::new(),
            options: Vec::new(),
            messages: Vec::new(),
            enums: Vec::new(),
            services: Vec::new(),
        }
    }
}

impl Message {
    pub fn new(name: String) -> Self {
        Message {
            name,
            fields: Vec::new(),
            oneofs: Vec::new(),
            nested_messages: Vec::new(),
            nested_enums: Vec::new(),
            options: Vec::new(),
            reserved: Vec::new(),
        }
    }
}

impl Enum {
    pub fn new(name: String) -> Self {
        Enum {
            name,
            values: Vec::new(),
            options: Vec::new(),
        }
    }
}

impl Service {
    pub fn new(name: String) -> Self {
        Service {
            name,
            methods: Vec::new(),
            options: Vec::new(),
        }
    }
}

impl ProtoOption {
    pub fn new(name: String, value: OptionValue) -> Self {
        ProtoOption { name, value }
    }
}

// TODO: Add support for 'extend' keyword
// TODO: Add support for 'Any' type
// TODO: Add support for 'Timestamp' type
// TODO: Add support for 'Duration' type
// TODO: Add support for 'Empty' type
// TODO: Add support for Well-Known Types (e.g., DoubleValue, FloatValue, Int64Value, etc.)

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_proto_file() {
        let mut proto_file = ProtoFile::new();
        proto_file.syntax = Syntax::Proto3;
        proto_file.package = Some("example.package".to_string());

        let import = Import {
            path: "google/protobuf/any.proto".to_string(),
            kind: ImportKind::Default,
        };
        proto_file.imports.push(import);

        let mut message = Message::new("Person".to_string());
        message.fields.push(Field {
            name: "name".to_string(),
            number: NumberValue::DecimalInt(1),
            label: FieldLabel::Optional,
            typ: FieldType::MessageOrEnum(String::new()),
            options: Vec::new(),
        });

        message.fields.push(Field {
            name: "age".to_string(),
            number: NumberValue::DecimalInt(2),
            label: FieldLabel::Optional,
            typ: FieldType::String,
            options: Vec::new(),
        });
        proto_file.messages.push(message);

        let mut enum_def = Enum::new("Gender".to_string());
        enum_def.values.push(EnumValue {
            name: "UNKNOWN".to_string(),
            number: NumberValue::DecimalInt(0),
            options: Vec::new(),
        });
        enum_def.values.push(EnumValue {
            name: "MALE".to_string(),
            number: NumberValue::DecimalInt(1),
            options: Vec::new(),
        });
        enum_def.values.push(EnumValue {
            name: "FEMALE".to_string(),
            number: NumberValue::DecimalInt(2),
            options: Vec::new(),
        });
        enum_def.values.push(EnumValue {
            name: "OTHER".to_string(),
            number: NumberValue::DecimalInt(3),
            options: Vec::new(),
        });
        proto_file.enums.push(enum_def);

        assert_eq!(proto_file.syntax, Syntax::Proto3);
        assert_eq!(proto_file.package, Some("example.package".to_string()));
        assert_eq!(proto_file.imports.len(), 1);
        assert_eq!(proto_file.messages.len(), 1);
        assert_eq!(proto_file.enums.len(), 1);
        assert_eq!(proto_file.messages[0].fields.len(), 2);
        assert_eq!(proto_file.enums[0].values.len(), 4);
    }
}

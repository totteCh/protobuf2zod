use crate::parser::ast::{Enum, EnumValue, Field, FieldType, Message, ProtoFile};
use heck::ToLowerCamelCase;
use std::fmt::Write;

pub fn generate_zod_schemas(proto_file: &ProtoFile) -> String {
    let mut output = String::new();

    // Generate enums before messages to prevent variable used before declaration errors
    for enum_def in &proto_file.enums {
        generate_enum_schema(&mut output, enum_def);
    }

    for message in &proto_file.messages {
        generate_message_schema(&mut output, message);
    }

    output
}

fn generate_message_schema(output: &mut String, message: &Message) {
    writeln!(output, "const {} = z.object({{", message.name).unwrap();
    for field in &message.fields {
        generate_field_schema(output, field);
    }
    writeln!(output, "}});").unwrap();
}

fn generate_field_schema(output: &mut String, field: &Field) {
    let field_type = match &field.typ {
        FieldType::Double | FieldType::Float => "z.number()",
        FieldType::Int32
        | FieldType::Int64
        | FieldType::UInt32
        | FieldType::UInt64
        | FieldType::SInt32
        | FieldType::SInt64
        | FieldType::Fixed32
        | FieldType::Fixed64
        | FieldType::SFixed32
        | FieldType::SFixed64 => "z.number().int()",
        FieldType::Bool => "z.boolean()",
        FieldType::String => "z.string()",
        FieldType::Bytes => "z.instanceof(Uint8Array)",
        FieldType::MessageOrEnum(ref name) => name,
        FieldType::Map(_, _) => "z.record(z.string(), z.any())", // Simplified for now
    };

    let field_name = to_camel_case(&field.name);
    writeln!(output, "  {}: {},", field_name, field_type).unwrap();
}

fn generate_enum_schema(output: &mut String, enum_def: &Enum) {
    writeln!(output, "const {} = z.enum([", enum_def.name).unwrap();
    let prefix = find_common_prefix(&enum_def.values);
    for value in &enum_def.values {
        let stripped_value = value.name.strip_prefix(&prefix).unwrap_or(&value.name);
        writeln!(output, "  \"{}\",", stripped_value).unwrap();
    }
    writeln!(output, "]);").unwrap();
}

fn to_camel_case(s: &str) -> String {
    s.to_lower_camel_case()
}

fn find_common_prefix(values: &[EnumValue]) -> String {
    if values.is_empty() {
        return String::new();
    }

    let first = &values[0].name;
    let mut prefix_len = first.len();

    for value in values.iter().skip(1) {
        prefix_len = prefix_len.min(value.name.len());
        for (i, (c1, c2)) in first.chars().zip(value.name.chars()).enumerate() {
            if c1 != c2 {
                prefix_len = i;
                break;
            }
        }
    }

    first[..prefix_len].to_string()
}

use crate::parser::ast::{Enum, Field, FieldType, Message, ProtoFile};
use std::fmt::Write;

pub fn generate_zod_schemas(proto_file: &ProtoFile) -> String {
    let mut output = String::new();

    for message in &proto_file.messages {
        generate_message_schema(&mut output, message);
    }

    for enum_def in &proto_file.enums {
        generate_enum_schema(&mut output, enum_def);
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

    writeln!(output, "  {}: {},", field.name, field_type).unwrap();
}

fn generate_enum_schema(output: &mut String, enum_def: &Enum) {
    writeln!(output, "const {} = z.enum([", enum_def.name).unwrap();
    for value in &enum_def.values {
        writeln!(output, "  \"{}\",", value.name).unwrap();
    }
    writeln!(output, "]);").unwrap();
}

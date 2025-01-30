//! Parser module for Protobuf to Zod converter
//!
//! This module contains the lexer, AST definitions, and parsing logic
//! for processing Protobuf files.

pub mod ast;
pub mod error;
mod lexer;

use crate::parser::ast::{
    Enum, EnumValue, Field, FieldLabel, Import, ImportKind, Message, Method, OneOf, OptionValue,
    ProtoFile, ProtoOption, Service, Syntax,
};

use ast::{EnumValueOption, EnumValueOptionValue, FieldType, NumberValue};
use error::Location;
pub use error::{ParseError, ParseResult};
pub use lexer::{tokenize, Token, TokenWithLocation};

use log::debug;
use std::iter::Peekable;

/// Parse a Protobuf file content into an AST representation
///
/// This function takes a string slice containing the Protobuf file content,
/// tokenizes it, and then parses the tokens to create an Abstract Syntax Tree (AST)
/// representation of the Protobuf file. It handles various Protobuf elements such as
/// syntax declarations, package declarations, imports, messages, enums, and services.
///
/// # Arguments
///
/// * `input` - A string slice containing the Protobuf file content
///
/// # Returns
///
/// * `Result<ProtoFile, ParseError>` - The parsed AST representation of the Protobuf file,
///   or a ParseError if any parsing errors occur during the process
// * `Result<ProtoFile, ParseError>` - The parsed AST or an error if parsing failed
pub fn parse_proto_file(input: &str) -> Result<ProtoFile, ParseError> {
    let tokens = tokenize(input)?;

    for (index, token_with_location) in tokens.iter().enumerate() {
        debug!(
            "Token {}: {:?} at {}",
            index, token_with_location.token, token_with_location.location
        );
    }

    let mut token_iter = tokens.into_iter().peekable();

    parse_tokenized_input(&mut token_iter)
}

fn parse_tokenized_input<'a, I>(tokens: I) -> Result<ProtoFile, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    let mut tokens = tokens.peekable();
    let mut proto_file = ProtoFile::new();

    // Skip initial comments
    skip_comments_and_whitespace(&mut tokens);

    // required
    parse_syntax(&mut tokens, &mut proto_file)?;

    while let Some(token) = tokens.peek() {
        match &token.token {
            Token::Package => parse_package(&mut tokens, &mut proto_file)?,
            Token::Comment(_) => {
                tokens.next(); // Skip comments
            }
            _ => break,
        }
    }

    // Parse imports that might follow package
    while let Some(token) = tokens.peek() {
        match &token.token {
            Token::Import => parse_import(&mut tokens, &mut proto_file)?,
            Token::Comment(_) => {
                tokens.next(); // Skip comments
            }
            _ => break,
        }
    }

    // Parse options that might follow imports
    while let Some(token) = tokens.peek() {
        match &token.token {
            Token::Option => parse_option(&mut tokens, &mut proto_file.options)?,
            Token::Comment(_) => {
                tokens.next(); // Skip comments
            }
            _ => break,
        }
    }

    while let Some(current_token) = tokens.peek() {
        match &current_token.token {
            Token::Message => {
                let message = parse_message(&mut tokens)?;
                proto_file.messages.push(message);
            }
            Token::Enum => {
                let enum_def = parse_enum(&mut tokens)?;
                proto_file.enums.push(enum_def);
            }
            Token::Service => {
                let service = parse_service(&mut tokens)?;
                proto_file.services.push(service);
            }
            Token::Comment(_) => {
                tokens.next(); // Skip comments
            }
            _ => {
                let loc = current_token.location;
                return Err(ParseError::UnexpectedToken(
                    format!("{:?}", current_token.token),
                    loc,
                ));
            }
        }

        // Skip comments between top-level elements
        skip_comments_and_whitespace(&mut tokens);
    }

    Ok(proto_file)
}

fn skip_comments_and_whitespace<'a, I>(tokens: &mut Peekable<I>)
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    while let Some(token) = tokens.peek() {
        match token.token {
            Token::Comment(_) => {
                tokens.next(); // Consume the token
            }
            _ => break,
        }
    }
}
/// Parses the syntax declaration of a Protobuf file.
///
/// This function expects to find a syntax declaration at the beginning of the file,
/// which specifies whether the file uses Proto2 or Proto3 syntax.
///
/// # Arguments
///
/// * `tokens` - A mutable reference to a peekable iterator of TokenWithLocation.
/// * `proto_file` - A mutable reference to the ProtoFile being constructed.
///
/// # Returns
///
/// * `Result<(), ParseError>` - Ok(()) if parsing succeeds, or a ParseError if any issues occur.
fn parse_syntax<'a, I>(
    tokens: &mut Peekable<I>,
    proto_file: &mut ProtoFile,
) -> Result<(), ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    // Consume 'syntax' token
    let syntax_token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?;
    match syntax_token.token {
        Token::Identifier(s) if s == "syntax" => {
            debug!("Found 'syntax' identifier");
        }
        Token::Syntax => {
            debug!("Found 'syntax' token");
        }
        _ => {
            debug!("Expected 'syntax', found {:?}", syntax_token.token);
            return Err(ParseError::UnexpectedToken(
                format!("Expected 'syntax', found {:?}", syntax_token.token),
                syntax_token.location,
            ));
        }
    }

    // Expect '=' token
    let equals_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(syntax_token.location))?;
    if equals_token.token != Token::Equals {
        return Err(ParseError::UnexpectedToken(
            format!("Expected '=', found {:?}", equals_token.token),
            equals_token.location,
        ));
    }

    // Parse syntax version
    let version_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(equals_token.location))?;
    debug!("Parsing syntax version: {:?}", version_token);
    match version_token.token {
        Token::StringLiteral("proto2") => proto_file.syntax = Syntax::Proto2,
        Token::StringLiteral("proto3") => proto_file.syntax = Syntax::Proto3,
        _ => {
            return Err(ParseError::InvalidSyntax(
                "Expected \"proto2\" or \"proto3\"".to_string(),
                version_token.location,
            ))
        }
    }

    // Expect semicolon
    debug!("Expecting semicolon after syntax version");
    if let Some(token) = tokens.peek() {
        debug!("Next token: {:?}", token);
        if token.token != Token::Semicolon {
            return Err(ParseError::UnexpectedToken(
                format!("Expected ';', found {:?}", token.token),
                token.location,
            ));
        }
        tokens.next(); // Consume the semicolon
        debug!("Consumed semicolon");
    } else {
        return Err(ParseError::UnexpectedEndOfInput(version_token.location));
    }

    Ok(())
}

/// Parses the package declaration of a Protobuf file.
///
/// This function expects to find a package declaration in the token stream.
/// It consumes the 'package' keyword, parses the package name, and updates
/// the ProtoFile struct with the parsed package name.
///
/// # Arguments
///
/// * `tokens` - A mutable reference to a peekable iterator of TokenWithLocation.
/// * `proto_file` - A mutable reference to the ProtoFile being constructed.
///
/// # Returns
///
/// * `Result<(), ParseError>` - Ok(()) if parsing succeeds, or a ParseError if any issues occur.
fn parse_package<'a, I>(
    tokens: &mut Peekable<I>,
    proto_file: &mut ProtoFile,
) -> Result<(), ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    // Consume 'package' token
    let package_token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::Package)?;

    let package_name = match tokens.next() {
        Some(TokenWithLocation {
            token: Token::Identifier(name),
            ..
        }) => {
            let mut full_name = name.to_string();
            while let Some(TokenWithLocation { token, .. }) = tokens.peek() {
                match token {
                    Token::Dot => {
                        tokens.next(); // Consume the dot
                        match tokens.next() {
                            Some(TokenWithLocation {
                                token: Token::Identifier(next_part),
                                ..
                            }) => {
                                full_name.push('.');
                                full_name.push_str(next_part);
                            }
                            _ => {
                                return Err(ParseError::UnexpectedToken(
                                    "Expected identifier after dot in package name".to_string(),
                                    package_token.location,
                                ))
                            }
                        }
                    }
                    Token::Semicolon => break,
                    _ => {
                        return Err(ParseError::UnexpectedToken(
                            format!("Unexpected token in package name: {:?}", token),
                            package_token.location,
                        ))
                    }
                }
            }
            full_name
        }
        Some(t) => {
            return Err(ParseError::UnexpectedToken(
                format!("Expected package name, found {:?}", t.token),
                t.location,
            ))
        }
        None => return Err(ParseError::UnexpectedEndOfInput(package_token.location)),
    };

    // Expect semicolon
    tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(package_token.location))?
        .expect(Token::Semicolon)?;

    proto_file.package = Some(package_name);
    Ok(())
}

/// Parses an import statement from the token stream.
///
/// This function expects to find an import declaration in the token stream.
/// It consumes the 'import' keyword, parses the optional import kind (weak or public),
/// and then parses the import path as a string literal. It updates the ProtoFile
/// struct with the parsed import information.
///
/// # Arguments
///
/// * `tokens` - A mutable reference to a peekable iterator of TokenWithLocation.
/// * `proto_file` - A mutable reference to the ProtoFile being constructed.
///
/// # Returns
///
/// * `Result<(), ParseError>` - Ok(()) if parsing succeeds, or a ParseError if any issues occur.
fn parse_import<'a, I>(
    tokens: &mut Peekable<I>,
    proto_file: &mut ProtoFile,
) -> Result<(), ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    let import_token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?;
    if import_token.token != Token::Import {
        return Err(ParseError::UnexpectedToken(
            format!("Expected 'import', found {:?}", import_token.token),
            import_token.location,
        ));
    }

    let mut kind = ImportKind::Default;
    let next_token = tokens
        .peek()
        .ok_or(ParseError::UnexpectedEndOfInput(import_token.location))?;

    match next_token.token {
        Token::Public => {
            tokens.next(); // Consume 'public' token
            kind = ImportKind::Public;
        }
        Token::Weak => {
            tokens.next(); // Consume 'weak' token
            kind = ImportKind::Weak;
        }
        _ => {} // Default import, no modifier
    }

    // Parse import path
    let path_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(import_token.location))?;
    let path = match path_token.token {
        Token::StringLiteral(path) => path.to_string(),
        // Token::Identifier(path) | Token::FullyQualifiedIdentifier(path) => path.to_string(),
        _ => {
            return Err(ParseError::UnexpectedToken(
                format!(
                    "Expected string literal or identifier for import path, found {:?}",
                    path_token.token
                ),
                path_token.location,
            ));
        }
    };

    // Expect semicolon
    let semicolon_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(path_token.location))?;
    if semicolon_token.token != Token::Semicolon {
        return Err(ParseError::UnexpectedToken(
            format!("Expected ';', found {:?}", semicolon_token.token),
            semicolon_token.location,
        ));
    }

    // Add the import to the proto file
    proto_file.imports.push(Import { path, kind });

    Ok(())
}

/// Parses a message definition from the token stream.
///
/// It parses the message name, opening brace, message body (including nested messages,
/// enums, fields, options, and reserved statements), and closing brace.
///
/// # Arguments
///
/// * `tokens` - A mutable reference to a peekable iterator of TokenWithLocation.
///
/// # Returns
///
/// * `Result<Message, ParseError>` - A Result containing the parsed Message on success,
///   or a ParseError on failure.
fn parse_message<'a, I>(tokens: &mut Peekable<I>) -> Result<Message, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    // Expect 'message' keyword
    let message_token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?;

    match &message_token.token {
        Token::Message => {}
        _ => {
            return Err(ParseError::UnexpectedToken(
                format!("Expected 'message', found {:?}", message_token.token),
                message_token.location,
            ));
        }
    }

    // Expect message name
    let name_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(message_token.location))?;
    let name = match &name_token.token {
        Token::Identifier(s) => s.to_string(),
        _ => {
            return Err(ParseError::UnexpectedToken(
                format!("Expected message name, found {:?}", name_token.token),
                name_token.location,
            ));
        }
    };

    // Expect opening brace
    let open_brace_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(name_token.location))?;
    if open_brace_token.token != Token::OpenBrace {
        return Err(ParseError::UnexpectedToken(
            format!("Expected '{{', found {:?}", open_brace_token.token),
            open_brace_token.location,
        ));
    }

    let mut message = Message::new(name);

    // Parse message body
    while let Some(_token_with_location) = tokens.peek() {
        skip_comments_and_whitespace(tokens);

        if let Some(token_with_location) = tokens.peek() {
            match &token_with_location.token {
                Token::CloseBrace => {
                    tokens.next(); // Consume closing brace
                    return Ok(message);
                }
                Token::Message => {
                    let nested_message = parse_message(tokens)?;
                    message.nested_messages.push(nested_message);
                }
                Token::Enum => {
                    let nested_enum = parse_enum(tokens)?;
                    message.nested_enums.push(nested_enum);
                }
                Token::Option => {
                    parse_option(tokens, &mut message.options)?;
                }
                Token::Reserved => {
                    parse_reserved(tokens, &mut message.reserved)?;
                }
                Token::Oneof => {
                    let oneof = parse_oneof(tokens)?;
                    message.oneofs.push(oneof);
                }
                _ => {
                    let field = parse_field(tokens)?;
                    message.fields.push(field);
                }
            }
        } else {
            break;
        }
    }

    Err(ParseError::UnexpectedEndOfInput(open_brace_token.location))
}

/// Parses a `oneof` block from the token stream.
///
/// # Arguments
///
/// * `tokens` - A mutable reference to a `Peekable` iterator over tokens.
///
/// # Returns
///
/// A `Result` containing a `OneOf` struct if successful, or a `ParseError` if an error occurs.
fn parse_oneof<'a, I>(tokens: &mut Peekable<I>) -> Result<OneOf, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    expect_token(tokens, Token::Oneof)?;
    let name = expect_identifier(tokens)?;
    expect_token(tokens, Token::OpenBrace)?;

    let mut oneof = OneOf {
        name,
        fields: Vec::new(),
    };

    while let Some(token) = tokens.peek() {
        match token.token {
            Token::CloseBrace => {
                expect_token(tokens, Token::CloseBrace)?;
                break;
            }
            _ => {
                let field = parse_field(tokens)?;
                oneof.fields.push(field);
            }
        }
    }

    Ok(oneof)
}

/// Expects a specific token from the token stream.
///
/// # Arguments
///
/// * `tokens` - A mutable reference to a `Peekable` iterator over tokens.
/// * `expected` - The expected token.
///
/// # Returns
///
/// A `Result` containing a `TokenWithLocation` if the expected token is found, or a `ParseError` if an error occurs.
fn expect_token<'a, I>(
    tokens: &mut Peekable<I>,
    expected: Token,
) -> Result<TokenWithLocation<'a>, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    if let Some(token) = tokens.next() {
        if token.token == expected {
            Ok(token)
        } else {
            Err(ParseError::UnexpectedToken(
                format!("Expected {:?}, found {:?}", expected, token.token),
                token.location,
            ))
        }
    } else {
        Err(ParseError::UnexpectedEndOfInput(Location::new(0, 0)))
    }
}

/// Expects an identifier token from the token stream.
///
/// # Arguments
///
/// * `tokens` - A mutable reference to a `Peekable` iterator over tokens.
///
/// # Returns
///
/// A `Result` containing the identifier as a `String` if successful, or a `ParseError` if an error occurs.
fn expect_identifier<'a, I>(tokens: &mut Peekable<I>) -> Result<String, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    if let Some(token) = tokens.next() {
        if let Token::Identifier(name) = token.token {
            Ok(name.to_string())
        } else {
            Err(ParseError::UnexpectedToken(
                format!("Expected identifier, found {:?}", token.token),
                token.location,
            ))
        }
    } else {
        Err(ParseError::UnexpectedEndOfInput(Location::new(0, 0)))
    }
}

/// Parses a message definition from the token stream.
///
/// This function expects the 'message' keyword to have already been consumed.
/// It parses the message name, opening brace, message body (including nested messages,
/// enums, fields, options, and reserved statements), and closing brace.
///
/// # Arguments
///
/// * `tokens` - A mutable reference to a peekable iterator of TokenWithLocation.
///
/// # Returns
///
/// * `Result<Message, ParseError>` - A Result containing the parsed Message on success,
///   or a ParseError on failure.
fn parse_field<'a, I>(tokens: &mut Peekable<I>) -> Result<Field, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    let start_location = tokens
        .peek()
        .map(|t| t.location)
        .unwrap_or(Location::new(0, 0));

    skip_comments_and_whitespace(tokens);

    // Parse field label (optional, repeated, required)
    let label = match tokens.peek() {
        Some(TokenWithLocation {
            token: Token::Repeated,
            ..
        }) => {
            tokens.next(); // Consume 'repeated'
            FieldLabel::Repeated
        }
        Some(TokenWithLocation {
            token: Token::Required,
            ..
        }) => {
            tokens.next(); // Consume 'required'
            FieldLabel::Required
        }
        _ => FieldLabel::Optional,
    };

    skip_comments_and_whitespace(tokens);

    let (typ, name) = if let Some(TokenWithLocation {
        token: Token::Map, ..
    }) = tokens.peek()
    {
        tokens.next(); // Consume 'map'
        parse_map_field(tokens)?
    } else {
        // Parse field type
        let type_token = tokens
            .next()
            .ok_or(ParseError::UnexpectedEndOfInput(start_location))?;

        debug!("Parsing field type: {:?}", type_token);

        let typ = parse_field_type(&type_token)?;

        // Parse field name
        let name = parse_field_name(tokens)?;

        (typ, name)
    };

    // Expect '=' token
    tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(start_location))?
        .expect(Token::Equals)?;

    // Parse field number
    let number_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(start_location))?;
    let number = match number_token.token {
        Token::DecimalIntLiteral(num) => NumberValue::DecimalInt(num),
        Token::FloatLiteral(num) => NumberValue::Float(num),
        Token::HexIntLiteral(num) => NumberValue::Hex(num),
        Token::OctalIntLiteral(num) => NumberValue::Octal(num),
        _ => {
            return Err(ParseError::UnexpectedToken(
                format!("Expected field number, found {:?}", number_token.token),
                number_token.location,
            ));
        }
    };

    // Expect semicolon
    tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(start_location))?
        .expect(Token::Semicolon)?;

    Ok(Field {
        name,
        label,
        typ,
        number,
        options: Vec::new(), // Add options parsing if needed
    })
}

/// Parses a field name from the token stream.
///
/// This function iterates through tokens, building up the field name.
/// It handles multi-part names (e.g., "message_field") and special cases like "message".
/// The parsing stops when it encounters an '=' token, which signifies the end of the field name.
///
/// # Arguments
///
/// * `tokens` - A mutable reference to a peekable iterator of TokenWithLocation.
///
/// # Returns
///
/// * `Result<String, ParseError>` - The parsed field name on success, or a ParseError on failure.
///
/// # Errors
///
/// Returns a ParseError if:
/// - Unexpected end of input is encountered
/// - An unexpected token is found (neither identifier, 'message', nor '=')
fn parse_field_name<'a, I>(tokens: &mut Peekable<I>) -> Result<String, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    let mut name_parts = Vec::new();
    let mut location = Location::new(0, 0);

    while let Some(token_with_location) = tokens.peek() {
        location = token_with_location.location;
        debug!(
            "Parsing field name, current token: {:?}",
            token_with_location.token
        );
        match &token_with_location.token {
            Token::Identifier(part) => {
                debug!("Adding identifier part: {}", part);
                // Split the identifier by underscores and add each part separately
                for (i, subpart) in part.split('_').enumerate() {
                    if i > 0 || !name_parts.is_empty() {
                        name_parts.push("_".to_string());
                    }
                    name_parts.push(subpart.to_string());
                }
                tokens.next(); // Consume the token
            }
            Token::Message if name_parts.is_empty() => {
                debug!("Found Message token");
                name_parts.push("message".to_string());
                tokens.next(); // Consume the token
            }
            Token::Map if name_parts.is_empty() => {
                debug!("Found Map token");
                name_parts.push("map".to_string());
                tokens.next(); // Consume the token
            }
            Token::Repeated if name_parts.is_empty() => {
                debug!("Found Repeated token");
                name_parts.push("repeated".to_string());
                tokens.next(); // Consume the token
            }
            Token::Equals => {
                debug!("Found Equals token, ending field name parsing");
                break;
            }
            _ => {
                debug!(
                    "Found unexpected token: {:?}, ending field name parsing",
                    token_with_location.token
                );
                break;
            }
        }
    }

    if name_parts.is_empty() {
        return Err(ParseError::MissingIdentifier(
            "Expected field name".to_string(),
            location,
        ));
    }

    let name = name_parts.join("");
    debug!("Final field name: {}", name);
    Ok(name)
}

/// Parses an enum definition from the token stream.
///
/// It parses the enum name, opening brace, enum body (including enum values
/// and options), and closing brace.
///
/// # Arguments
///
/// * `tokens` - A mutable reference to a peekable iterator of TokenWithLocation.
///
/// # Returns
///
/// * `Result<Enum, ParseError>` - A Result containing the parsed Enum on success,
///   or a ParseError on failure.
fn parse_enum<'a, I>(tokens: &mut Peekable<I>) -> Result<Enum, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    // Expect 'enum' keyword
    let enum_token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?;
    if enum_token.token != Token::Enum {
        return Err(ParseError::UnexpectedToken(
            format!("Expected 'enum', found {:?}", enum_token.token),
            enum_token.location,
        ));
    }

    // Parse enum name
    let name_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(enum_token.location))?;
    let name = match &name_token.token {
        Token::Identifier(name) => name.to_string(),
        _ => {
            return Err(ParseError::UnexpectedToken(
                format!("Expected enum name, found {:?}", name_token.token),
                name_token.location,
            ))
        }
    };

    // Expect opening brace
    let open_brace_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(name_token.location))?;
    if open_brace_token.token != Token::OpenBrace {
        return Err(ParseError::UnexpectedToken(
            format!("Expected '{{', found {:?}", open_brace_token.token),
            open_brace_token.location,
        ));
    }

    let mut enum_def = Enum::new(name);

    while let Some(token_with_location) = tokens.peek() {
        match &token_with_location.token {
            Token::CloseBrace => {
                tokens.next(); // Consume closing brace
                return Ok(enum_def);
            }
            Token::Identifier(_) => {
                // Parse enum value
                let value = parse_enum_value(tokens)?;
                enum_def.values.push(value);
            }
            Token::Option => {
                let option = parse_enum_option(tokens)?;
                enum_def.options.push(option);
            }
            _ => {
                return Err(ParseError::UnexpectedToken(
                    format!(
                        "Unexpected token in enum body: {:?}",
                        token_with_location.token
                    ),
                    token_with_location.location,
                ));
            }
        }
    }

    Err(ParseError::UnexpectedEndOfInput(open_brace_token.location))
}

fn parse_enum_option<'a, I>(tokens: &mut Peekable<I>) -> Result<EnumValueOption, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    // Consume 'option' token
    let option_token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::Option)?;

    // Parse option name
    let name_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(option_token.location))?;
    let name = match &name_token.token {
        Token::Identifier(s) => s.to_string(),
        _ => {
            return Err(ParseError::UnexpectedToken(
                format!("Expected option name, found {:?}", name_token.token),
                name_token.location,
            ));
        }
    };

    // Expect equals sign
    tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(name_token.location))?
        .expect(Token::Equals)?;

    // Parse option value
    let value_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(name_token.location))?;
    let value = match &value_token.token {
        Token::StringLiteral(s) => EnumValueOptionValue::String(s.to_string()),
        Token::Identifier(s) => EnumValueOptionValue::Identifier(s.to_string()),
        Token::DecimalIntLiteral(i) => EnumValueOptionValue::DecimalInt(*i),
        Token::FloatLiteral(f) => EnumValueOptionValue::Float(*f),
        _ => {
            return Err(ParseError::UnexpectedToken(
                format!("Expected option value, found {:?}", value_token.token),
                value_token.location,
            ));
        }
    };

    // Expect semicolon
    tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(value_token.location))?
        .expect(Token::Semicolon)?;

    Ok(EnumValueOption { name, value })
}

/// Parses an enum value from the token stream.
///
/// This function expects to parse an enum value name, '=' token, and an integer value.
/// It also handles optional enum value options enclosed in square brackets.
///
/// # Arguments
///
/// * `tokens` - A mutable reference to a peekable iterator of TokenWithLocation.
///
/// # Returns
///
/// * `Result<EnumValue, ParseError>` - A Result containing the parsed EnumValue on success,
///   or a ParseError on failure.
fn parse_enum_value<'a, I>(tokens: &mut Peekable<I>) -> Result<EnumValue, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    // Parse enum value name
    let name_token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?;
    let name = match &name_token.token {
        Token::Identifier(name) => name.to_string(),
        _ => {
            return Err(ParseError::UnexpectedToken(
                format!("Expected enum value name, found {:?}", name_token.token),
                name_token.location,
            ))
        }
    };

    // Expect '='
    let equals_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(name_token.location))?;
    if equals_token.token != Token::Equals {
        return Err(ParseError::UnexpectedToken(
            format!("Expected '=', found {:?}", equals_token.token),
            equals_token.location,
        ));
    }

    // Parse enum value number
    let number_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(equals_token.location))?;
    let number = match &number_token.token {
        Token::DecimalIntLiteral(num) => NumberValue::DecimalInt(*num),
        Token::FloatLiteral(num) => NumberValue::Float(*num),
        Token::OctalIntLiteral(num) => NumberValue::Octal(*num),
        Token::HexIntLiteral(num) => NumberValue::Hex(*num),
        _ => {
            return Err(ParseError::UnexpectedToken(
                format!("Expected integer, found {:?}", number_token.token),
                number_token.location,
            ))
        }
    };

    // Parse options if present
    let mut options = Vec::new();
    if let Some(TokenWithLocation {
        token: Token::OpenBracket,
        ..
    }) = tokens.peek()
    {
        tokens.next(); // Consume '['
        loop {
            let option = parse_enum_value_option(tokens)?;
            options.push(option);

            match tokens.peek() {
                Some(TokenWithLocation {
                    token: Token::Comma,
                    ..
                }) => {
                    tokens.next(); // Consume comma
                }
                Some(TokenWithLocation {
                    token: Token::CloseBracket,
                    ..
                }) => {
                    tokens.next(); // Consume closing bracket
                    break;
                }
                Some(t) => {
                    return Err(ParseError::UnexpectedToken(
                        format!("Expected ',' or ']', found {:?}", t.token),
                        t.location,
                    ))
                }
                None => return Err(ParseError::UnexpectedEndOfInput(number_token.location)),
            }
        }
    }

    // Expect semicolon
    let semicolon_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(number_token.location))?;
    if semicolon_token.token != Token::Semicolon {
        return Err(ParseError::UnexpectedToken(
            format!("Expected ';', found {:?}", semicolon_token.token),
            semicolon_token.location,
        ));
    }

    Ok(EnumValue {
        name,
        number,
        options,
    })
}

fn parse_enum_value_option<'a, I>(tokens: &mut Peekable<I>) -> Result<EnumValueOption, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    // Parse option name
    let name_token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?;
    let name = match &name_token.token {
        Token::Identifier(name) => name.to_string(),
        _ => {
            return Err(ParseError::UnexpectedToken(
                format!("Expected option name, found {:?}", name_token.token),
                name_token.location,
            ))
        }
    };

    // Expect '='
    let equals_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(name_token.location))?;
    if equals_token.token != Token::Equals {
        return Err(ParseError::UnexpectedToken(
            format!("Expected '=', found {:?}", equals_token.token),
            equals_token.location,
        ));
    }

    // Parse option value
    let value_token = tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(equals_token.location))?;
    let value = match &value_token.token {
        Token::StringLiteral(s) => EnumValueOptionValue::String(s.to_string()),
        Token::Identifier(s) => EnumValueOptionValue::Identifier(s.to_string()),
        Token::DecimalIntLiteral(i) => EnumValueOptionValue::DecimalInt(*i),
        Token::FloatLiteral(f) => EnumValueOptionValue::Float(*f),
        _ => {
            return Err(ParseError::UnexpectedToken(
                format!("Expected option value, found {:?}", value_token.token),
                value_token.location,
            ))
        }
    };

    Ok(EnumValueOption { name, value })
}

fn parse_service<'a, I>(tokens: &mut Peekable<I>) -> Result<Service, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    skip_comments_and_whitespace(tokens);

    // Expect 'service' keyword
    tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::Service)?;

    skip_comments_and_whitespace(tokens);

    // Parse service name
    let name = parse_identifier(tokens)?;

    skip_comments_and_whitespace(tokens);

    // Expect opening brace
    tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::OpenBrace)?;

    let mut methods = Vec::new();
    let mut options = Vec::new();

    loop {
        skip_comments_and_whitespace(tokens);

        match tokens.peek() {
            Some(TokenWithLocation {
                token: Token::CloseBrace,
                ..
            }) => {
                tokens.next(); // Consume '}'
                break;
            }
            Some(TokenWithLocation {
                token: Token::Rpc, ..
            }) => {
                let method = parse_method(tokens)?;
                methods.push(method);
            }
            Some(TokenWithLocation {
                token: Token::Option,
                ..
            }) => {
                parse_option(tokens, &mut options)?;
            }
            Some(TokenWithLocation {
                token: Token::Comment(_),
                ..
            }) => {
                tokens.next(); // Skip comments
            }
            Some(t) => {
                return Err(ParseError::UnexpectedToken(
                    format!("Unexpected token in service body: {:?}", t.token),
                    t.location,
                ))
            }
            None => return Err(ParseError::UnexpectedEndOfInput(Location::new(0, 0))),
        }
    }

    Ok(Service {
        name,
        methods,
        options,
    })
}

fn parse_method<'a, I>(tokens: &mut Peekable<I>) -> Result<Method, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    skip_comments_and_whitespace(tokens);

    // Expect 'rpc' keyword
    tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::Rpc)?;

    skip_comments_and_whitespace(tokens);

    // Parse method name
    let name = parse_identifier(tokens)?;

    skip_comments_and_whitespace(tokens);

    // Expect opening parenthesis
    tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::OpenParen)?;

    skip_comments_and_whitespace(tokens);

    // Check for 'stream' keyword
    let mut client_streaming = false;
    if let Some(TokenWithLocation {
        token: Token::Stream,
        ..
    }) = tokens.peek()
    {
        client_streaming = true;
        tokens.next(); // Consume 'stream' token
        skip_comments_and_whitespace(tokens);
    }

    // Parse input type
    let input_type = parse_type(tokens)?;

    skip_comments_and_whitespace(tokens);

    // Expect closing parenthesis
    tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::CloseParen)?;

    skip_comments_and_whitespace(tokens);

    // Expect 'returns' keyword
    tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::Returns)?;

    skip_comments_and_whitespace(tokens);

    // Expect opening parenthesis
    tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::OpenParen)?;

    skip_comments_and_whitespace(tokens);

    // Check for 'stream' keyword in return type
    let mut server_streaming = false;
    if let Some(TokenWithLocation {
        token: Token::Stream,
        ..
    }) = tokens.peek()
    {
        server_streaming = true;
        tokens.next(); // Consume 'stream' token
        skip_comments_and_whitespace(tokens);
    }

    // Parse output type
    let output_type = parse_type(tokens)?;

    skip_comments_and_whitespace(tokens);

    // Expect closing parenthesis
    tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::CloseParen)?;

    skip_comments_and_whitespace(tokens);

    let mut options = Vec::new();

    // Check for options or semicolon
    match tokens.peek() {
        Some(TokenWithLocation {
            token: Token::OpenBrace,
            ..
        }) => {
            tokens.next(); // Consume '{'
            while let Some(token) = tokens.peek() {
                match &token.token {
                    Token::CloseBrace => {
                        tokens.next(); // Consume '}'
                        break;
                    }
                    Token::Option => {
                        parse_option(tokens, &mut options)?;
                    }
                    Token::Comment(_) => {
                        tokens.next(); // Skip comments and whitespace
                    }
                    _ => {
                        return Err(ParseError::UnexpectedToken(
                            format!("Unexpected token in method options: {:?}", token.token),
                            token.location,
                        ))
                    }
                }
                skip_comments_and_whitespace(tokens);
            }
        }
        Some(TokenWithLocation {
            token: Token::Semicolon,
            ..
        }) => {
            tokens.next(); // Consume ';'
        }
        Some(t) => {
            return Err(ParseError::UnexpectedToken(
                format!(
                    "Expected '{{' or ';' after method definition, found {:?}",
                    t.token
                ),
                t.location,
            ))
        }
        None => return Err(ParseError::UnexpectedEndOfInput(Location::new(0, 0))),
    }

    Ok(Method {
        name,
        input_type,
        output_type,
        client_streaming,
        server_streaming,
        options,
    })
}

fn parse_option<'a, I>(
    tokens: &mut Peekable<I>,
    options: &mut Vec<ProtoOption>,
) -> Result<(), ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    // Consume 'option' token
    let option_token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::Option)?;

    // Parse option name (which may include dots)
    let name = parse_dotted_identifier(tokens)?;

    // Expect equals sign
    tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(option_token.location))?
        .expect(Token::Equals)?;

    // Parse option value
    let value = parse_option_value(tokens)?;

    // Expect semicolon
    tokens
        .next()
        .ok_or(ParseError::UnexpectedEndOfInput(option_token.location))?
        .expect(Token::Semicolon)?;

    options.push(ProtoOption::new(name, value));

    Ok(())
}

fn parse_dotted_identifier<'a, I>(tokens: &mut Peekable<I>) -> Result<String, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    let mut parts = Vec::new();

    while let Some(token) = tokens.peek() {
        match &token.token {
            Token::Identifier(s) => {
                parts.push(s.to_string());
                tokens.next(); // Consume the identifier
            }
            Token::Dot => {
                if parts.is_empty() {
                    return Err(ParseError::UnexpectedToken(
                        "Unexpected dot at the beginning of identifier".to_string(),
                        token.location,
                    ));
                }
                tokens.next(); // Consume the dot
            }
            _ => break,
        }
    }

    if parts.is_empty() {
        return Err(ParseError::UnexpectedEndOfInput(
            tokens.peek().map_or(Location::new(0, 0), |t| t.location),
        ));
    }

    Ok(parts.join("."))
}

fn parse_option_value<'a, I>(tokens: &mut Peekable<I>) -> Result<OptionValue, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    let value_token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?;

    match &value_token.token {
        Token::StringLiteral(s) => Ok(OptionValue::String(s.to_string())),
        Token::Identifier(s) => Ok(OptionValue::Identifier(s.to_string())),
        Token::DecimalIntLiteral(num) => Ok(OptionValue::DecimalInt(*num)),
        Token::OctalIntLiteral(num) => Ok(OptionValue::Octal(*num)),
        Token::HexIntLiteral(num) => Ok(OptionValue::Hex(*num)),
        Token::FloatLiteral(f) => Ok(OptionValue::Float(*f)),
        _ => Err(ParseError::UnexpectedToken(
            format!("Expected option value, found {:?}", value_token.token),
            value_token.location,
        )),
    }
}

fn parse_field_type(token: &TokenWithLocation) -> Result<FieldType, ParseError> {
    match &token.token {
        Token::Identifier(typ) => match *typ {
            "double" => Ok(FieldType::Double),
            "float" => Ok(FieldType::Float),
            "int32" => Ok(FieldType::Int32),
            "int64" => Ok(FieldType::Int64),
            "uint32" => Ok(FieldType::UInt32),
            "uint64" => Ok(FieldType::UInt64),
            "sint32" => Ok(FieldType::SInt32),
            "sint64" => Ok(FieldType::SInt64),
            "fixed32" => Ok(FieldType::Fixed32),
            "fixed64" => Ok(FieldType::Fixed64),
            "sfixed32" => Ok(FieldType::SFixed32),
            "sfixed64" => Ok(FieldType::SFixed64),
            "bool" => Ok(FieldType::Bool),
            "bytes" => Ok(FieldType::Bytes),
            _ => Ok(FieldType::MessageOrEnum(typ.to_string())),
        },
        Token::StringType => Ok(FieldType::String),
        _ => Err(ParseError::UnexpectedToken(
            format!("Expected field type, found {:?}", token.token),
            token.location,
        )),
    }
}

fn parse_identifier<'a, I>(tokens: &mut Peekable<I>) -> Result<String, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    let token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?;

    match token.token {
        Token::Identifier(s) => Ok(s.to_string()),
        _ => Err(ParseError::UnexpectedToken(
            format!("Expected identifier, found {:?}", token.token),
            token.location,
        )),
    }
}

fn parse_type<'a, I>(tokens: &mut Peekable<I>) -> Result<String, ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    let mut type_name = String::new();
    let mut first = true;

    loop {
        skip_comments_and_whitespace(tokens);

        match tokens.peek() {
            Some(TokenWithLocation {
                token: Token::Identifier(s),
                ..
            }) => {
                if !first {
                    type_name.push('.');
                }
                type_name.push_str(s);
                first = false;
                tokens.next(); // Consume the identifier
            }
            Some(TokenWithLocation {
                token: Token::Dot, ..
            }) => {
                if first {
                    return Err(ParseError::UnexpectedToken(
                        "Unexpected dot at the beginning of type name".to_string(),
                        tokens.peek().unwrap().location,
                    ));
                }
                type_name.push('.');
                tokens.next(); // Consume the dot
            }
            Some(TokenWithLocation {
                token: Token::Rpc, ..
            }) => {
                // Special case: 'rpc' is part of the type name (e.g., google.rpc.Status)
                if !first {
                    type_name.push('.');
                }
                type_name.push_str("rpc");
                tokens.next(); // Consume the 'rpc' token
            }
            Some(TokenWithLocation {
                token: Token::CloseParen,
                ..
            })
            | None => {
                // End of type name
                break;
            }
            Some(t) => {
                return Err(ParseError::UnexpectedToken(
                    format!("Unexpected token in type name: {:?}", t.token),
                    t.location,
                ));
            }
        }
    }

    if type_name.is_empty() {
        Err(ParseError::UnexpectedEndOfInput(Location::new(0, 0)))
    } else {
        Ok(type_name)
    }
}

/// Parses a map field from the token stream.
///
/// This function is called when a 'map' token is encountered while parsing a field.
/// It parses the key and value types of the map, the field name, and the field number.
///
/// # Arguments
///
/// * `tokens` - A mutable reference to a peekable iterator of TokenWithLocation.
///
/// # Returns
///
/// * `Result<Field, ParseError>` - A Result containing the parsed Field on success,
///   or a ParseError on failure.
///
/// # Errors
///
/// Returns a ParseError if:
/// - Unexpected end of input is encountered
/// - An unexpected token is found
/// - The field number is not an integer literal
fn parse_map_field<'a, I>(tokens: &mut Peekable<I>) -> Result<(FieldType, String), ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    // Expect '<'
    tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::LessThan)?;

    // Parse key type
    let key_type_token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?;
    let key_type = parse_field_type(&key_type_token)?;

    // Expect ','
    tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::Comma)?;

    // Parse value type
    let value_type_token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?;
    let value_type = parse_field_type(&value_type_token)?;

    // Expect '>'
    tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::GreaterThan)?;

    // Parse field name
    let name = parse_field_name(tokens)?;

    Ok((
        FieldType::Map(Box::new(key_type), Box::new(value_type)),
        name,
    ))
}

fn parse_reserved<'a, I>(
    tokens: &mut Peekable<I>,
    reserved: &mut Vec<crate::parser::ast::Reserved>,
) -> Result<(), ParseError>
where
    I: Iterator<Item = TokenWithLocation<'a>>,
{
    // Consume 'reserved' token
    let reserved_token = tokens
        .next()
        .ok_or_else(|| ParseError::UnexpectedEndOfInput(Location::new(0, 0)))?
        .expect(Token::Reserved)?;

    let mut last_location = reserved_token.location;

    loop {
        match tokens.next() {
            Some(token_with_location) => {
                match token_with_location.token {
                    Token::DecimalIntLiteral(start) => {
                        let start = start as i32;
                        if let Some(TokenWithLocation {
                            token: Token::To, ..
                        }) = tokens.peek()
                        {
                            tokens.next(); // Consume 'to' token
                            if let Some(TokenWithLocation {
                                token: Token::DecimalIntLiteral(end),
                                ..
                            }) = tokens.next()
                            {
                                let end = end as i32;
                                if start <= end {
                                    reserved.push(crate::parser::ast::Reserved::Range(start, end));
                                } else {
                                    return Err(ParseError::InvalidRange(
                                        start,
                                        end,
                                        token_with_location.location,
                                    ));
                                }
                            } else {
                                return Err(ParseError::UnexpectedEndOfInput(last_location));
                            }
                        } else {
                            reserved.push(crate::parser::ast::Reserved::Number(start));
                        }
                    }
                    Token::StringLiteral(name) => {
                        reserved.push(crate::parser::ast::Reserved::FieldName(name.to_string()));
                    }
                    Token::Semicolon => break,
                    Token::Comma => continue,
                    _ => {
                        return Err(ParseError::UnexpectedToken(
                            format!(
                                "Unexpected token in reserved: {:?}",
                                token_with_location.token
                            ),
                            token_with_location.location,
                        ))
                    }
                }
                last_location = token_with_location.location;
            }
            None => return Err(ParseError::UnexpectedEndOfInput(last_location)),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proto_file() {
        let input = r#"
            syntax = "proto3";
            package example;

            message Person {
                string name = 1;
                int32 age = 2;
                repeated string hobbies = 3;
            }

            enum Gender {
                UNKNOWN = 0;
                MALE = 1;
                FEMALE = 2;
            }
        "#;

        let result = parse_proto_file(input);
        assert!(
            result.is_ok(),
            "Failed to parse proto file: {:?}",
            result.err()
        );

        let proto_file = result.unwrap();
        assert_eq!(proto_file.syntax, crate::parser::ast::Syntax::Proto3);
        assert_eq!(proto_file.package, Some("example".to_string()));
        assert_eq!(proto_file.messages.len(), 1);
        assert_eq!(proto_file.enums.len(), 1);
    }

    #[test]
    fn test_parse_reserved() {
        let input = r#"
            syntax = "proto3";
            message TestReserved {
                reserved 2, 15, 9 to 11;
                reserved "foo", "bar";
            }
        "#;

        let result = parse_proto_file(input);
        assert!(
            result.is_ok(),
            "Failed to parse proto file: {:?}",
            result.err()
        );

        let proto_file = result.unwrap();
        assert_eq!(proto_file.messages.len(), 1);

        let message = &proto_file.messages[0];
        assert_eq!(message.name, "TestReserved");
        assert_eq!(message.reserved.len(), 5);

        assert!(message
            .reserved
            .contains(&crate::parser::ast::Reserved::Number(2)));
        assert!(message
            .reserved
            .contains(&crate::parser::ast::Reserved::Number(15)));
        assert!(message
            .reserved
            .contains(&crate::parser::ast::Reserved::Range(9, 11)));
        assert!(message
            .reserved
            .contains(&crate::parser::ast::Reserved::FieldName("foo".to_string())));
        assert!(message
            .reserved
            .contains(&crate::parser::ast::Reserved::FieldName("bar".to_string())));
    }

    #[test]
    fn test_parse_field_types() {
        let _ = env_logger::builder().is_test(true).try_init();

        let input = r#"
            syntax = "proto3";
            package test;

            message TestFieldTypes {
                double double_field = 1;
                float float_field = 2;
                int32 int32_field = 3;
                int64 int64_field = 4;
                uint32 uint32_field = 5;
                uint64 uint64_field = 6;
                sint32 sint32_field = 7;
                sint64 sint64_field = 8;
                fixed32 fixed32_field = 9;
                fixed64 fixed64_field = 10;
                sfixed32 sfixed32_field = 11;
                sfixed64 sfixed64_field = 12;
                bool bool_field = 13;
                string string_field = 14;
                bytes bytes_field = 15;
                CustomMessage message_field = 16;
                map<string, int32> map_field = 17;
                repeated int32 repeated_field = 18;
            }

            message CustomMessage {
                string custom_field = 1;
            }
        "#;

        let result = parse_proto_file(input);
        assert!(
            result.is_ok(),
            "Failed to parse proto file: {:?}",
            result.err()
        );

        let proto_file = result.unwrap();
        assert_eq!(proto_file.syntax, Syntax::Proto3);
        assert_eq!(proto_file.package, Some("test".to_string()));
        assert_eq!(proto_file.messages.len(), 2);

        let test_field_types = &proto_file.messages[0];
        assert_eq!(test_field_types.name, "TestFieldTypes");
        assert_eq!(test_field_types.fields.len(), 18);

        let expected_types = vec![
            (
                FieldType::Double,
                "double_field",
                NumberValue::DecimalInt(1),
            ),
            (FieldType::Float, "float_field", NumberValue::DecimalInt(2)),
            (FieldType::Int32, "int32_field", NumberValue::DecimalInt(3)),
            (FieldType::Int64, "int64_field", NumberValue::DecimalInt(4)),
            (
                FieldType::UInt32,
                "uint32_field",
                NumberValue::DecimalInt(5),
            ),
            (
                FieldType::UInt64,
                "uint64_field",
                NumberValue::DecimalInt(6),
            ),
            (
                FieldType::SInt32,
                "sint32_field",
                NumberValue::DecimalInt(7),
            ),
            (
                FieldType::SInt64,
                "sint64_field",
                NumberValue::DecimalInt(8),
            ),
            (
                FieldType::Fixed32,
                "fixed32_field",
                NumberValue::DecimalInt(9),
            ),
            (
                FieldType::Fixed64,
                "fixed64_field",
                NumberValue::DecimalInt(10),
            ),
            (
                FieldType::SFixed32,
                "sfixed32_field",
                NumberValue::DecimalInt(11),
            ),
            (
                FieldType::SFixed64,
                "sfixed64_field",
                NumberValue::DecimalInt(12),
            ),
            (FieldType::Bool, "bool_field", NumberValue::DecimalInt(13)),
            (
                FieldType::String,
                "string_field",
                NumberValue::DecimalInt(14),
            ),
            (FieldType::Bytes, "bytes_field", NumberValue::DecimalInt(15)),
            (
                FieldType::MessageOrEnum("CustomMessage".to_string()),
                "message_field",
                NumberValue::DecimalInt(16),
            ),
            (
                FieldType::Map(Box::new(FieldType::String), Box::new(FieldType::Int32)),
                "map_field",
                NumberValue::DecimalInt(17),
            ),
            (
                FieldType::Int32,
                "repeated_field",
                NumberValue::DecimalInt(18),
            ),
        ];

        for (index, (expected_type, expected_name, expected_number)) in
            expected_types.iter().enumerate()
        {
            let field = &test_field_types.fields[index];
            assert_eq!(
                field.typ, *expected_type,
                "Field {} has unexpected type",
                expected_name
            );
            assert_eq!(
                field.name, *expected_name,
                "Field at index {} has unexpected name",
                index
            );
            assert_eq!(
                field.number, *expected_number,
                "Field {} has unexpected number",
                expected_name
            );

            if index == 17 {
                assert_eq!(
                    field.label,
                    FieldLabel::Repeated,
                    "Field {} should be repeated",
                    expected_name
                );
            } else {
                assert_eq!(
                    field.label,
                    FieldLabel::Optional,
                    "Field {} should be optional",
                    expected_name
                );
            }
        }

        let custom_message = &proto_file.messages[1];
        assert_eq!(custom_message.name, "CustomMessage");
        assert_eq!(custom_message.fields.len(), 1);

        let custom_field = &custom_message.fields[0];
        assert_eq!(custom_field.name, "custom_field");
        assert_eq!(custom_field.typ, FieldType::String);
        assert_eq!(custom_field.number, NumberValue::DecimalInt(1));
        assert_eq!(custom_field.label, FieldLabel::Optional);
    }
}

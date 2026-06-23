use proc_macro::{Literal, TokenStream, TokenTree};

#[proc_macro]
pub fn log(input: TokenStream) -> TokenStream {
    expand(input, None)
}

#[proc_macro]
pub fn debug(input: TokenStream) -> TokenStream {
    expand(input, Some("::oslog::Level::Debug"))
}

#[proc_macro]
pub fn info(input: TokenStream) -> TokenStream {
    expand(input, Some("::oslog::Level::Info"))
}

#[proc_macro]
pub fn default(input: TokenStream) -> TokenStream {
    expand(input, Some("::oslog::Level::Default"))
}

#[proc_macro]
pub fn error(input: TokenStream) -> TokenStream {
    expand(input, Some("::oslog::Level::Error"))
}

#[proc_macro]
pub fn fault(input: TokenStream) -> TokenStream {
    expand(input, Some("::oslog::Level::Fault"))
}

fn expand(input: TokenStream, fixed_level: Option<&str>) -> TokenStream {
    match expand_inner(input, fixed_level) {
        Ok(output) => output.parse().expect("generated invalid Rust"),
        Err(message) => compile_error(&message),
    }
}

fn expand_inner(input: TokenStream, fixed_level: Option<&str>) -> Result<String, String> {
    let parts = split_top_level_commas(input);
    let minimum_parts = if fixed_level.is_some() { 2 } else { 3 };

    if parts.len() < minimum_parts {
        return Err(match fixed_level {
            Some(_) => "expected log, format and optional arguments".into(),
            None => "expected log, level, format and optional arguments".into(),
        });
    }

    let log = tokens_to_string(&parts[0]);
    let (level, format_index) = match fixed_level {
        Some(level) => (level.to_string(), 1),
        None => (tokens_to_string(&parts[1]), 2),
    };
    let format_literal = parse_format_literal(&parts[format_index])?;
    let specs = parse_format_specs(format_literal.value.as_bytes())?;
    let spec_count = specs.len();
    let args = &parts[(format_index + 1)..];

    if specs.len() != args.len() {
        return Err(format!(
            "OSLog format string expects {} arguments, but {} were supplied",
            specs.len(),
            args.len()
        ));
    }

    if specs.len() > u8::MAX as usize {
        return Err(format!("OSLog supports at most {} arguments", u8::MAX));
    }

    let specs_tokens = specs
        .iter()
        .map(|spec| {
            format!(
                "::oslog::FormatSpec::new(::oslog::ArgumentKind::{}, {}, {})",
                spec.kind.as_rust(),
                spec.privacy,
                spec.size
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let args_tokens = args
        .iter()
        .zip(specs.iter())
        .map(|(arg, spec)| spec.checker(tokens_to_string(arg)))
        .collect::<Vec<_>>()
        .join(", ");

    Ok(format!(
        r#"{{
            let __oslog_log = ({log});
            let __oslog_level = {level};
            if __oslog_log.level_is_enabled(__oslog_level) {{
                #[link_section = "__TEXT,__oslogstring,cstring_literals"]
                static FORMAT: [u8; concat!({format_source}, "\0").len()] =
                    ::oslog::__private_str_to_array::<{{ concat!({format_source}, "\0").len() }}>(concat!({format_source}, "\0"));
                static SPECS: [::oslog::FormatSpec; {spec_count}] = [{specs_tokens}];
                ::oslog::emit_with_specs(__oslog_log, __oslog_level, &FORMAT, &SPECS, &[{args_tokens}]);
            }}
        }}"#,
        format_source = format_literal.source,
        spec_count = spec_count,
        specs_tokens = specs_tokens,
        log = log,
        level = level,
        args_tokens = args_tokens,
    ))
}

fn split_top_level_commas(input: TokenStream) -> Vec<Vec<TokenTree>> {
    split_comma_tokens(input, token_split_kind)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SplitKind {
    Comma,
    Colon,
    LessThan,
    GreaterThan,
    Pipe,
    IdentAsync,
    IdentMove,
    Other,
}

fn token_split_kind(token: &TokenTree) -> SplitKind {
    match token {
        TokenTree::Punct(punct) if punct.as_char() == ',' => SplitKind::Comma,
        TokenTree::Punct(punct) if punct.as_char() == ':' => SplitKind::Colon,
        TokenTree::Punct(punct) if punct.as_char() == '<' => SplitKind::LessThan,
        TokenTree::Punct(punct) if punct.as_char() == '>' => SplitKind::GreaterThan,
        TokenTree::Punct(punct) if punct.as_char() == '|' => SplitKind::Pipe,
        TokenTree::Ident(ident) if ident.to_string() == "async" => SplitKind::IdentAsync,
        TokenTree::Ident(ident) if ident.to_string() == "move" => SplitKind::IdentMove,
        _ => SplitKind::Other,
    }
}

fn split_comma_tokens<T, F>(tokens: impl IntoIterator<Item = T>, split_kind: F) -> Vec<Vec<T>>
where
    F: Fn(&T) -> SplitKind,
{
    let mut parts = Vec::new();
    let mut current = Vec::new();
    let mut in_closure_params = false;
    let mut generic_depth = 0usize;
    let mut previous = None;
    let mut saw_double_colon = false;

    for token in tokens {
        let kind = split_kind(&token);

        match kind {
            SplitKind::Comma if !in_closure_params && generic_depth == 0 => {
                parts.push(current);
                current = Vec::new();
            }
            SplitKind::LessThan if saw_double_colon => {
                generic_depth += 1;
                current.push(token);
            }
            SplitKind::LessThan if generic_depth > 0 => {
                generic_depth += 1;
                current.push(token);
            }
            SplitKind::GreaterThan if generic_depth > 0 => {
                generic_depth -= 1;
                current.push(token);
            }
            SplitKind::Pipe => {
                if in_closure_params {
                    in_closure_params = false;
                } else if is_closure_prefix(&current, &split_kind) {
                    in_closure_params = true;
                }
                current.push(token);
            }
            _ => current.push(token),
        }

        saw_double_colon = previous == Some(SplitKind::Colon) && kind == SplitKind::Colon;
        previous = Some(kind);
    }

    if !current.is_empty() || parts.is_empty() {
        parts.push(current);
    }

    parts
}

fn is_closure_prefix<T, F>(tokens: &[T], split_kind: &F) -> bool
where
    F: Fn(&T) -> SplitKind,
{
    match tokens {
        [] => true,
        [token] => matches!(
            split_kind(token),
            SplitKind::IdentAsync | SplitKind::IdentMove
        ),
        [first, second] => {
            split_kind(first) == SplitKind::IdentAsync && split_kind(second) == SplitKind::IdentMove
        }
        _ => false,
    }
}

fn tokens_to_string(tokens: &[TokenTree]) -> String {
    tokens.iter().cloned().collect::<TokenStream>().to_string()
}

struct ParsedLiteral {
    source: String,
    value: String,
}

fn parse_format_literal(tokens: &[TokenTree]) -> Result<ParsedLiteral, String> {
    if tokens.len() != 1 {
        return Err("OSLog format must be a single string literal".into());
    }

    let literal = match &tokens[0] {
        TokenTree::Literal(literal) => literal,
        _ => return Err("OSLog format must be a string literal".into()),
    };
    let source = literal.to_string();
    let value = parse_string_literal(literal)?;

    Ok(ParsedLiteral { source, value })
}

fn parse_string_literal(literal: &Literal) -> Result<String, String> {
    let source = literal.to_string();

    if source.starts_with('"') {
        parse_escaped_string(&source)
    } else if source.starts_with('r') {
        parse_raw_string(&source)
    } else {
        Err("OSLog format must be a string literal".into())
    }
}

fn parse_raw_string(source: &str) -> Result<String, String> {
    let hashes = source
        .bytes()
        .skip(1)
        .take_while(|byte| *byte == b'#')
        .count();
    let prefix_len = 1 + hashes;
    let suffix = format!("\"{}", "#".repeat(hashes));

    if !source[prefix_len..].starts_with('"') || !source.ends_with(&suffix) {
        return Err("invalid raw string literal".into());
    }

    Ok(source[(prefix_len + 1)..(source.len() - suffix.len())].to_string())
}

fn parse_escaped_string(source: &str) -> Result<String, String> {
    let mut chars = source[1..source.len() - 1].chars();
    let mut output = String::new();

    while let Some(ch) = chars.next() {
        if ch != '\\' {
            output.push(ch);
            continue;
        }

        let escaped = chars
            .next()
            .ok_or_else(|| "unterminated string escape".to_string())?;
        match escaped {
            '"' => output.push('"'),
            '\\' => output.push('\\'),
            'n' => output.push('\n'),
            'r' => output.push('\r'),
            't' => output.push('\t'),
            '0' => output.push('\0'),
            'x' => {
                let high = chars
                    .next()
                    .ok_or_else(|| "incomplete hex escape".to_string())?;
                let low = chars
                    .next()
                    .ok_or_else(|| "incomplete hex escape".to_string())?;
                let value = hex_value(high)? << 4 | hex_value(low)?;
                output.push(value as char);
            }
            'u' => {
                if chars.next() != Some('{') {
                    return Err("invalid unicode escape".into());
                }
                let mut hex = String::new();
                let mut terminated = false;
                for ch in chars.by_ref() {
                    if ch == '}' {
                        terminated = true;
                        break;
                    }
                    hex.push(ch);
                }
                if !terminated {
                    return Err("invalid unicode escape".into());
                }
                let value = u32::from_str_radix(&hex, 16)
                    .map_err(|_| "invalid unicode escape".to_string())?;
                output.push(
                    char::from_u32(value).ok_or_else(|| "invalid unicode scalar".to_string())?,
                );
            }
            '\n' => {
                while matches!(chars.clone().next(), Some(' ' | '\n' | '\r' | '\t')) {
                    chars.next();
                }
            }
            _ => return Err(format!("unsupported string escape \\{}", escaped)),
        }
    }

    Ok(output)
}

fn hex_value(ch: char) -> Result<u8, String> {
    ch.to_digit(16)
        .map(|value| value as u8)
        .ok_or_else(|| "invalid hex escape".to_string())
}

#[derive(Clone, Copy, Debug)]
struct FormatSpec {
    kind: ArgumentKind,
    privacy: u8,
    size: u8,
    expected: ExpectedArgument,
}

#[derive(Clone, Copy, Debug)]
enum ArgumentKind {
    Scalar,
    String,
}

impl ArgumentKind {
    fn as_rust(self) -> &'static str {
        match self {
            Self::Scalar => "Scalar",
            Self::String => "String",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum ExpectedArgument {
    Signed1,
    Signed2,
    Signed4,
    Signed8,
    SignedPtr,
    Unsigned1,
    Unsigned2,
    Unsigned4,
    Unsigned8,
    UnsignedPtr,
    Float,
    Char,
    String,
    Pointer,
}

impl FormatSpec {
    fn checker(self, arg: String) -> String {
        let function = match self.expected {
            ExpectedArgument::Signed1 => "__private_arg_signed_1",
            ExpectedArgument::Signed2 => "__private_arg_signed_2",
            ExpectedArgument::Signed4 => "__private_arg_signed_4",
            ExpectedArgument::Signed8 => "__private_arg_signed_8",
            ExpectedArgument::SignedPtr => "__private_arg_signed_ptr",
            ExpectedArgument::Unsigned1 => "__private_arg_unsigned_1",
            ExpectedArgument::Unsigned2 => "__private_arg_unsigned_2",
            ExpectedArgument::Unsigned4 => "__private_arg_unsigned_4",
            ExpectedArgument::Unsigned8 => "__private_arg_unsigned_8",
            ExpectedArgument::UnsignedPtr => "__private_arg_unsigned_ptr",
            ExpectedArgument::Float => "__private_arg_float",
            ExpectedArgument::Char => "__private_arg_char",
            ExpectedArgument::String => "__private_arg_string",
            ExpectedArgument::Pointer => "__private_arg_pointer",
        };

        format!("::oslog::{}(&({}))", function, arg)
    }
}

fn parse_format_specs(format: &[u8]) -> Result<Vec<FormatSpec>, String> {
    let mut specs = Vec::new();
    let mut index = 0;

    while index < format.len() {
        if format[index] != b'%' {
            index += 1;
            continue;
        }

        index += 1;

        if format.get(index) == Some(&b'%') {
            index += 1;
            continue;
        }

        let privacy = if format.get(index) == Some(&b'{') {
            let tag_start = index + 1;
            let tag_end = format[tag_start..]
                .iter()
                .position(|byte| *byte == b'}')
                .map(|offset| tag_start + offset)
                .ok_or_else(|| "unterminated OSLog format tag".to_string())?;
            let tag = std::str::from_utf8(&format[tag_start..tag_end])
                .map_err(|_| "OSLog format tags must be UTF-8".to_string())?;
            index = tag_end + 1;

            let mut privacy = 0;
            for part in tag.split(',').map(str::trim) {
                match part {
                    "private" => privacy = 1,
                    "public" => privacy = 2,
                    _ => {}
                }
            }
            privacy
        } else {
            0
        };

        while matches!(format.get(index), Some(b'-' | b'+' | b' ' | b'#' | b'0')) {
            index += 1;
        }

        while matches!(format.get(index), Some(b'0'..=b'9')) {
            index += 1;
        }

        if format.get(index) == Some(&b'*') {
            return Err("dynamic OSLog field widths are not supported".into());
        }

        if format.get(index) == Some(&b'.') {
            index += 1;
            if format.get(index) == Some(&b'*') {
                return Err("dynamic OSLog precision is not supported".into());
            }
            while matches!(format.get(index), Some(b'0'..=b'9')) {
                index += 1;
            }
        }

        let length_start = index;
        while matches!(
            format.get(index),
            Some(b'h' | b'l' | b'L' | b'z' | b't' | b'j')
        ) {
            index += 1;
        }
        let length = &format[length_start..index];

        let conversion = *format
            .get(index)
            .ok_or_else(|| "missing OSLog format conversion".to_string())?;
        index += 1;

        let (kind, size, expected) = match conversion {
            b'd' | b'i' => signed_arg(length)?,
            b'o' | b'u' | b'x' | b'X' => unsigned_arg(length)?,
            b'f' | b'F' | b'e' | b'E' | b'g' | b'G' | b'a' | b'A' => {
                (ArgumentKind::Scalar, 8, ExpectedArgument::Float)
            }
            b'c' => (ArgumentKind::Scalar, 4, ExpectedArgument::Char),
            b's' => (
                ArgumentKind::String,
                std::mem::size_of::<usize>() as u8,
                ExpectedArgument::String,
            ),
            b'p' => (
                ArgumentKind::Scalar,
                std::mem::size_of::<usize>() as u8,
                ExpectedArgument::Pointer,
            ),
            _ => {
                return Err(format!(
                    "unsupported OSLog format conversion '{}'",
                    conversion as char
                ))
            }
        };

        specs.push(FormatSpec {
            kind,
            privacy,
            size,
            expected,
        });
    }

    Ok(specs)
}

fn signed_arg(length: &[u8]) -> Result<(ArgumentKind, u8, ExpectedArgument), String> {
    match length {
        b"hh" => Ok((ArgumentKind::Scalar, 1, ExpectedArgument::Signed1)),
        b"h" => Ok((ArgumentKind::Scalar, 2, ExpectedArgument::Signed2)),
        b"" => Ok((ArgumentKind::Scalar, 4, ExpectedArgument::Signed4)),
        b"l" | b"ll" | b"j" => Ok((ArgumentKind::Scalar, 8, ExpectedArgument::Signed8)),
        b"z" | b"t" => Ok((
            ArgumentKind::Scalar,
            std::mem::size_of::<usize>() as u8,
            ExpectedArgument::SignedPtr,
        )),
        _ => Err("unsupported OSLog signed integer length modifier".into()),
    }
}

fn unsigned_arg(length: &[u8]) -> Result<(ArgumentKind, u8, ExpectedArgument), String> {
    match length {
        b"hh" => Ok((ArgumentKind::Scalar, 1, ExpectedArgument::Unsigned1)),
        b"h" => Ok((ArgumentKind::Scalar, 2, ExpectedArgument::Unsigned2)),
        b"" => Ok((ArgumentKind::Scalar, 4, ExpectedArgument::Unsigned4)),
        b"l" | b"ll" | b"j" => Ok((ArgumentKind::Scalar, 8, ExpectedArgument::Unsigned8)),
        b"z" | b"t" => Ok((
            ArgumentKind::Scalar,
            std::mem::size_of::<usize>() as u8,
            ExpectedArgument::UnsignedPtr,
        )),
        _ => Err("unsupported OSLog unsigned integer length modifier".into()),
    }
}

fn compile_error(message: &str) -> TokenStream {
    let escaped = message.replace('\\', "\\\\").replace('"', "\\\"");
    format!("compile_error!(\"{}\");", escaped)
        .parse()
        .expect("generated invalid compile_error")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_split_kind(token: &&str) -> SplitKind {
        match *token {
            "," => SplitKind::Comma,
            ":" => SplitKind::Colon,
            "<" => SplitKind::LessThan,
            ">" => SplitKind::GreaterThan,
            "|" => SplitKind::Pipe,
            "async" => SplitKind::IdentAsync,
            "move" => SplitKind::IdentMove,
            _ => SplitKind::Other,
        }
    }

    #[test]
    fn splits_top_level_commas() {
        let parts = split_comma_tokens(
            ["log", ",", "level", ",", "\"%d\"", ",", "value"],
            test_split_kind,
        );

        assert_eq!(
            parts,
            vec![vec!["log"], vec!["level"], vec!["\"%d\""], vec!["value"]]
        );
    }

    #[test]
    fn does_not_split_closure_parameter_commas() {
        let parts = split_comma_tokens(
            [
                "log", ",", "\"%p\"", ",", "|", "a", ",", "b", "|", "a", ",", "next",
            ],
            test_split_kind,
        );

        assert_eq!(
            parts,
            vec![
                vec!["log"],
                vec!["\"%p\""],
                vec!["|", "a", ",", "b", "|", "a"],
                vec!["next"]
            ]
        );
    }

    #[test]
    fn does_not_split_move_closure_parameter_commas() {
        let parts = split_comma_tokens(
            [
                "log", ",", "\"%p\"", ",", "async", "move", "|", "a", ",", "b", "|", "a",
            ],
            test_split_kind,
        );

        assert_eq!(
            parts,
            vec![
                vec!["log"],
                vec!["\"%p\""],
                vec!["async", "move", "|", "a", ",", "b", "|", "a"]
            ]
        );
    }

    #[test]
    fn does_not_split_turbofish_generic_argument_commas() {
        let parts = split_comma_tokens(
            [
                "log", ",", "\"%d\"", ",", "value", ":", ":", "<", "Result", "<", "i32", ",",
                "i32", ">", ">",
            ],
            test_split_kind,
        );

        assert_eq!(
            parts,
            vec![
                vec!["log"],
                vec!["\"%d\""],
                vec!["value", ":", ":", "<", "Result", "<", "i32", ",", "i32", ">", ">"]
            ]
        );
    }

    #[test]
    fn still_splits_after_comparison_expressions() {
        let parts = split_comma_tokens(
            ["log", ",", "\"%d %d\"", ",", "a", "<", "b", ",", "c"],
            test_split_kind,
        );

        assert_eq!(
            parts,
            vec![
                vec!["log"],
                vec!["\"%d %d\""],
                vec!["a", "<", "b"],
                vec!["c"]
            ]
        );
    }

    #[test]
    fn parses_escaped_format_literals() {
        assert_eq!(
            parse_escaped_string(r#""line\n%{public}s \u{2713}""#).unwrap(),
            "line\n%{public}s \u{2713}"
        );
        assert_eq!(parse_escaped_string(r#""\x25d""#).unwrap(), "%d");
    }

    #[test]
    fn parses_raw_format_literals() {
        assert_eq!(
            parse_raw_string(r##"r#"raw %{public}s \n"#"##).unwrap(),
            r"raw %{public}s \n"
        );
    }

    #[test]
    fn parses_multiline_and_unicode_escapes() {
        assert_eq!(
            parse_escaped_string("\"one\\\n    two \\u{1f642}\"").unwrap(),
            "onetwo \u{1f642}"
        );
    }

    #[test]
    fn rejects_invalid_literal_escapes() {
        assert_eq!(
            parse_escaped_string(r#""\xzz""#).unwrap_err(),
            "invalid hex escape"
        );
        assert_eq!(
            parse_escaped_string(r#""\u{123""#).unwrap_err(),
            "invalid unicode escape"
        );
    }

    #[test]
    fn parses_supported_specifiers() {
        let specs = parse_format_specs(
            b"%% %{public}hhd %{private}hd %d %lld %zd %hhu %hu %u %llu %zu %f %s %p",
        )
        .unwrap();

        assert_eq!(specs.len(), 13);
        assert!(matches!(specs[0].expected, ExpectedArgument::Signed1));
        assert_eq!(specs[0].privacy, 2);
        assert!(matches!(specs[1].expected, ExpectedArgument::Signed2));
        assert_eq!(specs[1].privacy, 1);
        assert!(matches!(specs[2].expected, ExpectedArgument::Signed4));
        assert!(matches!(specs[3].expected, ExpectedArgument::Signed8));
        assert!(matches!(specs[4].expected, ExpectedArgument::SignedPtr));
        assert!(matches!(specs[5].expected, ExpectedArgument::Unsigned1));
        assert!(matches!(specs[6].expected, ExpectedArgument::Unsigned2));
        assert!(matches!(specs[7].expected, ExpectedArgument::Unsigned4));
        assert!(matches!(specs[8].expected, ExpectedArgument::Unsigned8));
        assert!(matches!(specs[9].expected, ExpectedArgument::UnsignedPtr));
        assert!(matches!(specs[10].expected, ExpectedArgument::Float));
        assert!(matches!(specs[11].expected, ExpectedArgument::String));
        assert!(matches!(specs[12].expected, ExpectedArgument::Pointer));
    }

    #[test]
    fn parses_flags_widths_and_precision() {
        let specs = parse_format_specs(b"%-+#08.2f %10s %.4d").unwrap();

        assert_eq!(specs.len(), 3);
        assert!(matches!(specs[0].expected, ExpectedArgument::Float));
        assert!(matches!(specs[1].expected, ExpectedArgument::String));
        assert!(matches!(specs[2].expected, ExpectedArgument::Signed4));
    }

    #[test]
    fn rejects_unsupported_conversion() {
        let error = parse_format_specs(b"%@").unwrap_err();
        assert!(error.contains("unsupported OSLog format conversion"));
    }

    #[test]
    fn rejects_unsupported_length_modifiers() {
        assert_eq!(
            parse_format_specs(b"%Ld").unwrap_err(),
            "unsupported OSLog signed integer length modifier"
        );
        assert_eq!(
            parse_format_specs(b"%Lu").unwrap_err(),
            "unsupported OSLog unsigned integer length modifier"
        );
    }

    #[test]
    fn rejects_dynamic_width_and_precision() {
        assert_eq!(
            parse_format_specs(b"%*d").unwrap_err(),
            "dynamic OSLog field widths are not supported"
        );
        assert_eq!(
            parse_format_specs(b"%.*f").unwrap_err(),
            "dynamic OSLog precision is not supported"
        );
    }

    #[test]
    fn rejects_unterminated_privacy_tag() {
        let error = parse_format_specs(b"%{private").unwrap_err();
        assert_eq!(error, "unterminated OSLog format tag");
    }

    #[test]
    fn rejects_missing_conversion() {
        let error = parse_format_specs(b"%{public}").unwrap_err();
        assert_eq!(error, "missing OSLog format conversion");
        let error = parse_format_specs(b"%").unwrap_err();
        assert_eq!(error, "missing OSLog format conversion");
    }
}

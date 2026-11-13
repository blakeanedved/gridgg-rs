//! Fills selected fields that are missing from a GraphQL response with
//! `null`.
//!
//! The parser understands the operation documents cynic generates: one
//! operation with fields, aliases, arguments, directives and inline
//! fragments (no named fragments).

use serde_json::{Map, Value};

/// One entry of a selection set.
#[derive(Debug, PartialEq)]
pub(crate) enum Selection {
    /// A field, by response key (its alias if it has one), with its
    /// sub-selection.
    Field(String, Vec<Selection>),
    /// An inline fragment with its type condition, if any.
    Fragment(Option<String>, Vec<Selection>),
}

/// Parses the selection set of the first operation in `query`, or `None` if
/// the document is not in the expected shape.
pub(crate) fn parse(query: &str) -> Option<Vec<Selection>> {
    let tokens = tokenize(query)?;
    let mut parser = Parser { tokens, pos: 0 };
    // Skip the operation header up to its selection set.
    let mut depth = 0_u32;
    loop {
        match parser.peek()? {
            "(" => depth += 1,
            ")" => depth = depth.checked_sub(1)?,
            "{" if depth == 0 => break,
            _ => {}
        }
        parser.pos += 1;
    }
    parser.selection_set()
}

/// Inserts `null` into every object of `data` for each selected field it
/// lacks. Fragment fields are filled only where `__typename` matches the
/// fragment's type condition.
pub(crate) fn fill_missing(data: &mut Value, selections: &[Selection]) {
    match data {
        Value::Array(items) => {
            for item in items {
                fill_missing(item, selections);
            }
        }
        Value::Object(map) => fill_object(map, selections),
        _ => {}
    }
}

fn fill_object(map: &mut Map<String, Value>, selections: &[Selection]) {
    for selection in selections {
        match selection {
            Selection::Field(key, sub) => {
                let value = map.entry(key.clone()).or_insert(Value::Null);
                if !sub.is_empty() {
                    fill_missing(value, sub);
                }
            }
            Selection::Fragment(on, sub) => {
                let applies = on.as_ref().is_none_or(|on| {
                    map.get("__typename").and_then(Value::as_str) == Some(on.as_str())
                });
                if applies {
                    fill_object(map, sub);
                }
            }
        }
    }
}

struct Parser<'a> {
    tokens: Vec<&'a str>,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&'a str> {
        self.tokens.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<&'a str> {
        let token = self.peek()?;
        self.pos += 1;
        Some(token)
    }

    fn eat(&mut self, token: &str) -> bool {
        let found = self.peek() == Some(token);
        if found {
            self.pos += 1;
        }
        found
    }

    /// Parses `{ selection* }`.
    fn selection_set(&mut self) -> Option<Vec<Selection>> {
        if !self.eat("{") {
            return None;
        }
        let mut selections = Vec::new();
        while !self.eat("}") {
            if self.eat("...") {
                let on = if self.eat("on") {
                    Some(self.name()?.to_owned())
                } else {
                    None
                };
                self.directives()?;
                selections.push(Selection::Fragment(on, self.selection_set()?));
                continue;
            }
            let key = self.name()?;
            if self.eat(":") {
                self.name()?;
            }
            self.arguments()?;
            self.directives()?;
            let sub = if self.peek() == Some("{") {
                self.selection_set()?
            } else {
                Vec::new()
            };
            selections.push(Selection::Field(key.to_owned(), sub));
        }
        Some(selections)
    }

    fn name(&mut self) -> Option<&'a str> {
        let token = self.next()?;
        let mut chars = token.chars();
        let first = chars.next()?;
        (first == '_' || first.is_ascii_alphabetic()).then_some(token)
    }

    /// Skips a balanced `( ... )` group, if present.
    fn arguments(&mut self) -> Option<()> {
        if self.peek() != Some("(") {
            return Some(());
        }
        let mut depth = 0_u32;
        loop {
            match self.next()? {
                "(" => depth += 1,
                ")" => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(());
                    }
                }
                _ => {}
            }
        }
    }

    /// Skips `@name(args)*`.
    fn directives(&mut self) -> Option<()> {
        while self.eat("@") {
            self.name()?;
            self.arguments()?;
        }
        Some(())
    }
}

/// Splits a document into names/numbers, punctuators, `...` and string
/// literals; drops whitespace, commas and comments. `None` on an
/// unterminated string.
fn tokenize(query: &str) -> Option<Vec<&str>> {
    let bytes = query.as_bytes();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\r' | b',' => i += 1,
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'"' => {
                i += 1;
                loop {
                    match *bytes.get(i)? {
                        b'\\' => i += 2,
                        b'"' => break,
                        _ => i += 1,
                    }
                }
                i += 1;
                tokens.push(&query[start..i]);
            }
            b'.' if query[i..].starts_with("...") => {
                i += 3;
                tokens.push("...");
            }
            c if c == b'_' || c == b'-' || c.is_ascii_alphanumeric() => {
                // Names, and numbers such as `-1.5e3` (only skipped).
                i += 1;
                while i < bytes.len()
                    && (bytes[i] == b'_' || bytes[i] == b'.' || bytes[i].is_ascii_alphanumeric())
                {
                    i += 1;
                }
                tokens.push(&query[start..i]);
            }
            _ => {
                // Punctuators, and any other byte as its own (UTF-8 safe)
                // character.
                let len = query[i..].chars().next()?.len_utf8();
                i += len;
                tokens.push(&query[start..i]);
            }
        }
    }
    Some(tokens)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const QUERY: &str = r#"query Q($id: ID!, $on: Boolean!) {
  a: series(id: $id, filter: {name: {contains: "x)}"}, n: [-1.5e3]}) {
    id
    title @include(if: $on) {
      name
    }
    teams {
      __typename
      ... on Lol {
        gold
      }
      ... {
        common
      }
    }
  }
}
"#;

    fn field(key: &str, sub: Vec<Selection>) -> Selection {
        Selection::Field(key.into(), sub)
    }

    #[test]
    fn parses_cynic_documents() {
        let parsed = parse(QUERY).unwrap();
        let expected = vec![field(
            "a",
            vec![
                field("id", vec![]),
                field("title", vec![field("name", vec![])]),
                field(
                    "teams",
                    vec![
                        field("__typename", vec![]),
                        Selection::Fragment(Some("Lol".into()), vec![field("gold", vec![])]),
                        Selection::Fragment(None, vec![field("common", vec![])]),
                    ],
                ),
            ],
        )];
        assert_eq!(parsed, expected);
        assert!(parse("query Q { a").is_none());
        assert!(parse(r#"query Q { a(x: "open) }"#).is_none());
    }

    #[test]
    fn fills_missing_fields_with_null() {
        let mut data = json!({
            "a": {
                "id": "1",
                "teams": [
                    { "__typename": "Lol", "gold": 3 },
                    { "__typename": "Valorant" }
                ]
            }
        });
        fill_missing(&mut data, &parse(QUERY).unwrap());
        assert_eq!(
            data,
            json!({
                "a": {
                    "id": "1",
                    "title": null,
                    "teams": [
                        { "__typename": "Lol", "gold": 3, "common": null },
                        { "__typename": "Valorant", "common": null }
                    ]
                }
            })
        );
    }

    #[test]
    fn null_parents_stay_null() {
        let mut data = json!({ "a": null });
        fill_missing(&mut data, &parse(QUERY).unwrap());
        assert_eq!(data, json!({ "a": null }));
    }
}

use crate::keyvalue::KeyValue;
use crate::request_definition::{Content, RequestDefinition};
use lazy_static::lazy_static;
use regex::Regex;
use std::borrow::Cow;

lazy_static! {
    static ref RE: Regex = Regex::new(r"\{(.+?)\}").unwrap();
}

// Naive substitution, just replace each variable one-by-one.
// Could optimize at some point, but possibly not worth it.
pub fn substitute<'a>(base: &'a str, variables: &'a [KeyValue]) -> (Cow<'a, str>, bool) {
    let mut output: String = base.to_owned();
    for var in variables {
        let target = format!("{{{}}}", var.name);
        output = output.replace(&target, &var.value);
    }

    // If nothing was actually replaced, can just return the original reference. This extra boolean
    // flag is just Cow's `is_owned`, when that feature makes it to stable Rust we can remove this
    // flag.
    if output == base {
        (Cow::Borrowed(base), false)
    } else {
        (Cow::Owned(output), true)
    }
}

fn unbound_in_string(s: &str) -> Vec<&str> {
    RE.captures_iter(s)
        .map(|cap| cap.get(1).unwrap().as_str())
        .collect()
}

/// List the variables (things like {var1}) that exist in a RequestDefinition.
pub fn list_unbound_variables(request_definition: &RequestDefinition) -> Vec<&str> {
    let mut result: Vec<&str> = vec![];

    // URL
    result.append(&mut unbound_in_string(&request_definition.request.url));

    // Headers
    for header in request_definition
        .headers
        .iter()
        .flat_map(|h| h.headers.iter())
    {
        result.append(&mut unbound_in_string(&header.name));
        result.append(&mut unbound_in_string(&header.value));
    }

    // Query params
    for param in request_definition
        .query
        .iter()
        .flat_map(|q| q.params.iter())
    {
        result.append(&mut unbound_in_string(&param.name));
        result.append(&mut unbound_in_string(&param.value));
    }

    // Body
    match &request_definition.body {
        Some(Content::Text(text)) => {
            result.append(&mut unbound_in_string(&text));
        }
        Some(Content::Json(json_string)) => {
            result.append(&mut unbound_in_string(&json_string));
        }
        Some(Content::UrlEncoded(params)) => {
            for param in params {
                result.append(&mut unbound_in_string(&param.name));
                result.append(&mut unbound_in_string(&param.value));
            }
        }
        None => {}
    }

    result.sort();
    result.dedup();
    result
}

/// Mutate the provided RequestDefinition so that the provided variables are substituted into the
/// URL, headers, query parameters, and body.
pub fn substitute_all(def: &mut RequestDefinition, vars: &[KeyValue]) {
    let (new_url, is_owned) = substitute(&def.request.url, vars);
    if is_owned {
        def.request.url = new_url.into_owned();
    }

    match &mut def.query {
        Some(query) => {
            for param in &mut query.params {
                let (new_name, is_owned) = substitute(&param.name, vars);
                if is_owned {
                    param.name = new_name.into_owned();
                }
                let (new_value, is_owned) = substitute(&param.value, vars);
                if is_owned {
                    param.value = new_value.into_owned();
                }
            }
        }
        None => {}
    }

    match &mut def.headers {
        Some(headers) => {
            for header in &mut headers.headers {
                let (new_name, is_owned) = substitute(&header.name, vars);
                if is_owned {
                    header.name = new_name.into_owned();
                }
                let (new_value, is_owned) = substitute(&header.value, vars);
                if is_owned {
                    header.value = new_value.into_owned();
                }
            }
        }
        None => {}
    }

    match def.body.as_mut() {
        Some(Content::Text(text_content)) => {
            let (new_content, is_owned) = substitute(&text_content, vars);
            if is_owned {
                *text_content = new_content.into_owned();
            }
        }
        Some(Content::Json(json_string)) => {
            let (new_content, is_owned) = substitute(&json_string, vars);
            if is_owned {
                *json_string = new_content.into_owned();
            }
        }
        Some(Content::UrlEncoded(params)) => {
            for param in params {
                let (new_name, is_owned) = substitute(&param.name, vars);
                if is_owned {
                    param.name = new_name.into_owned();
                }
                let (new_value, is_owned) = substitute(&param.value, vars);
                if is_owned {
                    param.value = new_value.into_owned();
                }
            }
        }
        None => {}
    }
}

#[test]
fn test_substitute() {
    let vars = vec![
        KeyValue {
            name: "var1".to_string(),
            value: "value1".to_string(),
        },
        KeyValue {
            name: "var2".to_string(),
            value: "value2".to_string(),
        },
        KeyValue {
            name: "not_present".to_string(),
            value: "unused".to_string(),
        },
    ];
    let base = "a {var2} b {var1} c {var3} d {var2}";

    let (new_string, is_owned) = substitute(base, &vars);

    assert_eq!(new_string, "a value2 b value1 c {var3} d value2");

    assert_eq!(is_owned, true)
}

#[test]
fn test_unbound_in_string() {
    assert_eq!(
        unbound_in_string("one two {three} four {five} six"),
        vec!["three", "five"]
    );

    let blank: Vec<&str> = Vec::new();
    assert_eq!(unbound_in_string("no variables"), blank);
}

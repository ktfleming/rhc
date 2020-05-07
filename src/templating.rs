use crate::keyvalue::KeyValue;
use std::borrow::Cow;

// Naive substitution, just replace each variable one-by-one.
// Could optimize at some point, but possibly not worth it.
pub fn substitute<'a>(base: &'a str, variables: &'a [KeyValue]) -> Cow<'a, str> {
    let mut output: String = base.to_owned();
    for var in variables {
        let target = format!("{{{}}}", var.name);
        output = output.replace(&target, &var.value);
    }

    // TODO: is this Cow stuff all correct?
    if output == base {
        Cow::Borrowed(base)
    } else {
        Cow::Owned(output)
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

    assert_eq!(
        substitute(base, &vars),
        "a value2 b value1 c {var3} d value2"
    )
}

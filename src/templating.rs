use crate::keyvalue::KeyValue;

// Naive substitution, just replace each variable one-by-one.
// Could optimize at some point, but possibly not worth it.
pub fn substitute(mut output: String, variables: &[KeyValue]) -> String {
    for var in variables {
        let target = format!("{{{}}}", var.name);
        output = output.replace(&target, &var.value);
    }

    output
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
        substitute(base.to_string(), &vars),
        "a value2 b value1 c {var3} d value2"
    )
}

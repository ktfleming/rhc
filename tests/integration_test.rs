use anyhow;
use assert_cmd::assert::Assert;
use assert_cmd::Command;
use httptest::{matchers::*, responders::*, Expectation, Server};
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_help() {
    let mut cmd = Command::cargo_bin("main").unwrap();
    let assert = cmd.arg("--help").assert();
    assert.success();
}

struct TestFixture {
    server: Server,
    def_file: NamedTempFile,
    env_file: Option<NamedTempFile>,
}

fn setup(content: &str, env_content: Option<&str>) -> anyhow::Result<TestFixture> {
    let _ = pretty_env_logger::try_init();
    let server = Server::run();
    let content = content.replace("__base_url__/", &server.url_str(""));
    let mut def_file = NamedTempFile::new()?;
    write!(def_file, "{}", &content)?;

    let env_file = env_content.map(|env_content| {
        let mut env_file = NamedTempFile::new().unwrap();
        write!(env_file, "{}", &env_content).unwrap();

        env_file
    });

    Ok(TestFixture {
        server,
        def_file,
        env_file,
    })
}

fn run(fixture: TestFixture) -> Assert {
    let mut cmd = Command::cargo_bin("main").unwrap();

    // If we don't borrow the env_file here, it will get dropped (deleted) after this block, and
    // the program will fail with a "file not found" error.
    if let Some(env_file) = &fixture.env_file {
        cmd.arg("--environment");
        cmd.arg(env_file.path());
    }

    cmd.arg("--file");
    cmd.arg(fixture.def_file.path());

    let assert = cmd.assert();
    // let output = assert.get_output();
    // println!("{:?}", output);

    assert
}

#[test]
fn test_basic_get() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "GET"
    url = "__base_url__/foo"
    "#,
        None,
    )?;

    fixture.server.expect(
        Expectation::matching(request::method_path("GET", "/foo")).respond_with(status_code(200)),
    );

    run(fixture).success();
    Ok(())
}

#[test]
fn test_query_params() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "GET"
    url = "__base_url__/foo"

    [query]
    params = [
      { name = "first", value = "value1" },
      { name = "あああ", value = "猫" }
    ]
    "#,
        None,
    )?;

    fixture.server.expect(
        Expectation::matching(all_of![
            request::method_path("GET", "/foo"),
            request::query(url_decoded(contains(("first", "value1")))),
            request::query(url_decoded(contains(("あああ", "猫")))),
        ])
        .respond_with(status_code(200)),
    );

    run(fixture).success();
    Ok(())
}

#[test]
fn test_basic_post() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "POST"
    url = "__base_url__/foo"
    "#,
        None,
    )?;

    fixture.server.expect(
        Expectation::matching(request::method_path("POST", "/foo")).respond_with(status_code(200)),
    );

    run(fixture).success();
    Ok(())
}

#[test]
fn test_post_json() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "POST"
    url = "__base_url__/foo"

    [body]
    type = "json"
    content = '''
    {
      "a_number": 123,
      "a_string": "some value",
      "is_null": null,
      "an_array": [1, 2, 3],
      "an_object": {
        "inside": "the object"
      }
    }'''
    "#,
        None,
    )?;

    fixture.server.expect(
        Expectation::matching(all_of![
            request::method_path("POST", "/foo"),
            request::body(json_decoded(eq(serde_json::json!({
                "a_number": 123,
                "a_string": "some value",
                "is_null": null,
                "an_array": [1, 2, 3],
                "an_object": {
                  "inside": "the object"
                }
            }))))
        ])
        .respond_with(status_code(200)),
    );

    run(fixture).success();
    Ok(())
}

#[test]
fn test_post_text() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "POST"
    url = "__base_url__/foo"

    [body]
    type = "text"
    content = "plain text body"
    "#,
        None,
    )?;

    fixture.server.expect(
        Expectation::matching(all_of![
            request::method_path("POST", "/foo"),
            request::body("plain text body")
        ])
        .respond_with(status_code(200)),
    );

    run(fixture).success();
    Ok(())
}

#[test]
fn test_post_urlencoded() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "POST"
    url = "__base_url__/foo"

    [body]
    type = "urlencoded"
    content = [
      { name = "key1", value = "value1" },
      { name = "あいうえお", value = "猪" }
    ]
    "#,
        None,
    )?;

    fixture.server.expect(
        Expectation::matching(all_of![
            request::method_path("POST", "/foo"),
            request::body(url_decoded(contains(("key1", "value1")))),
            request::body(url_decoded(contains(("あいうえお", "猪"))))
        ])
        .respond_with(status_code(200)),
    );

    run(fixture).success();
    Ok(())
}

#[test]
fn test_headers() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "GET"
    url = "__base_url__/foo"

    [headers]
    headers = [
      { name = "first", value = "value1" },
      { name = "second", value = "value2" }
    ]
    "#,
        None,
    )?;

    fixture.server.expect(
        Expectation::matching(all_of![
            request::method_path("GET", "/foo"),
            request::headers(contains(("first", "value1"))),
            request::headers(contains(("second", "value2"))),
        ])
        .respond_with(status_code(200)),
    );

    run(fixture).success();
    Ok(())
}

#[test]
fn test_templating_json() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "POST"
    url = "__base_url__/{var1}"

    [headers]
    headers = [
      { name = "first", value = "{header_1}" },
    ]

    [body]
    type = "json"
    content = '''
    {
        "{var1}": "{var1}",
        "an_object": {
            "inside": "the object",
            "aa{var1}": "___{var1}"
        }
    }
    '''
    "#,
        Some(
            r#"
        name = "test_env"
        variables = [
          { name = "var1", value = "bar" },
          { name = "header_1", value = "translated" },
        ]
    "#,
        ),
    )?;

    fixture.server.expect(
        Expectation::matching(all_of![
            request::method_path("POST", "/bar"),
            request::headers(contains(("first", "translated"))),
            request::body(json_decoded(eq(serde_json::json!({
                "bar": "bar",
                "an_object": {
                  "inside": "the object",
                  "aabar": "___bar"
                }
            }))))
        ])
        .respond_with(status_code(200)),
    );

    run(fixture).success();
    Ok(())
}

#[test]
fn test_templating_string() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "POST"
    url = "__base_url__/{var1}"

    [body]
    type = "text"
    content = "foo{var1}"
    "#,
        Some(
            r#"
        name = "test_env"
        variables = [
          { name = "var1", value = "bar" }
        ]
    "#,
        ),
    )?;

    fixture.server.expect(
        Expectation::matching(all_of![
            request::method_path("POST", "/bar"),
            request::body("foobar")
        ])
        .respond_with(status_code(200)),
    );

    run(fixture).success();
    Ok(())
}

#[test]
fn test_templating_urlencoded() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "POST"
    url = "__base_url__/{var1}"

    [body]
    type = "urlencoded"
    content = [
      { name = "{var1}", value = "{var1}" }
    ]
    "#,
        Some(
            r#"
        name = "test_env"
        variables = [
          { name = "var1", value = "bar" }
        ]
    "#,
        ),
    )?;

    fixture.server.expect(
        Expectation::matching(all_of![
            request::method_path("POST", "/bar"),
            request::body(url_decoded(contains(("bar", "bar")))),
        ])
        .respond_with(status_code(200)),
    );

    run(fixture).success();
    Ok(())
}

#[test]
fn test_not_a_tty_1() -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin("main").unwrap();
    let assert = cmd.assert();

    // No request definition file was provided, so the program will try to enter interactive mode,
    // but the test is not running in a TTY, so it should fail with the appropriate message.
    assert.failure().stderr(predicate::eq(
        "Running in interactive mode requires a TTY\n",
    ));

    Ok(())
}

#[test]
fn test_not_a_tty_2() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "GET"
    url = "__base_url__/{unbound}"
    "#,
        None,
    )?;

    // let mut cmd = Command::cargo_bin("main").unwrap();
    // cmd.arg("--file");
    // cmd.arg(fixture.def_file.path());
    // let assert = cmd.assert();

    // Unbound variables exist, so the program will try to enter interactive mode, but the test is
    // not running in a TTY, so it should fail with the appropriate message.
    run(fixture).failure().stderr(predicate::eq(
        "Running in interactive mode requires a TTY\n",
    ));

    Ok(())
}

#[test]
fn test_no_spinner_when_no_tty() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "GET"
    url = "__base_url__/foo"
    "#,
        None,
    )?;

    fixture.server.expect(
        Expectation::matching(request::method_path("GET", "/foo")).respond_with(status_code(200)),
    );

    run(fixture)
        .success()
        .stdout(predicate::str::contains("Sending request").not());
    Ok(())
}

#[test]
fn test_bindings_simple() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "GET"
    url = "__base_url__/{var1}"
    "#,
        None,
    )?;

    fixture.server.expect(
        Expectation::matching(request::method_path("GET", "/bar")).respond_with(status_code(200)),
    );

    let mut cmd = Command::cargo_bin("main").unwrap();

    cmd.arg("--file");
    cmd.arg(fixture.def_file.path());

    cmd.arg("--binding");
    cmd.arg("var1=bar");

    cmd.assert().success();
    Ok(())
}

#[test]
fn test_bindings_overwrite() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "GET"
    url = "__base_url__/{var1}/{something}/{var3}"
    "#,
        Some(
            r#"
        name = "test_env"
        variables = [
          { name = "var1", value = "original" },
          { name = "something", value = "a_value" },
        ]
    "#,
        ),
    )?;

    fixture.server.expect(
        Expectation::matching(request::method_path("GET", "/new/a_value/aaa"))
            .respond_with(status_code(200)),
    );

    let mut cmd = Command::cargo_bin("main").unwrap();

    cmd.arg("--file");
    cmd.arg(fixture.def_file.path());

    if let Some(env_file) = &fixture.env_file {
        cmd.arg("--environment");
        cmd.arg(env_file.path());
    }

    cmd.arg("--binding");
    cmd.arg("var1=new");

    cmd.arg("--binding");
    cmd.arg("var3=aaa");

    cmd.assert().success();
    Ok(())
}

#[test]
fn test_duplicate_vars_in_env() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "GET"
    url = "__base_url__/foo"
    "#,
        Some(
            r#"
        name = "test_env"
        variables = [
          { name = "var1", value = "original" },
          { name = "var1", value = "duplicate" },
        ]
    "#,
        ),
    )?;

    let assert = run(fixture);
    assert.failure().stderr(predicate::str::contains(
        "contains duplicate bindings for: var1",
    ));
    Ok(())
}

use anyhow;
use assert_cmd::Command;
use httptest::{matchers::*, responders::*, Expectation, Server};
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

fn run(fixture: TestFixture) -> anyhow::Result<()> {
    let mut cmd = Command::cargo_bin("main").unwrap();

    // If we don't borrow the env_file here, it will get
    // dropped (deleted) after this block, and the program
    // will fail with a "file not found" error.
    if let Some(env_file) = &fixture.env_file {
        cmd.arg("--environment");
        cmd.arg(env_file.path());
    }
    cmd.arg(fixture.def_file.path());
    let assert = cmd.assert();
    // let output = assert.get_output();
    // println!("{:?}", output);
    assert.success();

    Ok(())
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

    run(fixture)
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

    run(fixture)
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

    run(fixture)
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

    run(fixture)
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

    run(fixture)
}

#[test]
fn test_templating() -> anyhow::Result<()> {
    let fixture = setup(
        r#"
    [request]
    method = "GET"
    url = "__base_url__/{var1}"

    [headers]
    headers = [
      { name = "first", value = "{header_1}" },
    ]
    "#,
        Some(
            r#"
        variables = [
          { name = "var1", value = "bar" },
          { name = "header_1", value = "translated" },
        ]
    "#,
        ),
    )?;

    fixture.server.expect(
        Expectation::matching(all_of![
            request::method_path("GET", "/bar"),
            request::headers(contains(("first", "translated"))),
        ])
        .respond_with(status_code(200)),
    );

    run(fixture)
}

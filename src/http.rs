use crate::request_definition::{Content, RequestDefinition};
use crate::response::Response;
use attohttpc::body;

// Wrapper around attohttpc's PreparedRequest, in order to
// make the types simpler
enum OurPreparedRequest {
    Bytes(attohttpc::PreparedRequest<body::Bytes<Vec<u8>>>),
    Text(attohttpc::PreparedRequest<body::Text<String>>),
    Empty(attohttpc::PreparedRequest<body::Empty>),
}

fn prepare_request(def: RequestDefinition) -> anyhow::Result<OurPreparedRequest> {
    let mut request_builder =
        attohttpc::RequestBuilder::try_new(def.request.method.to_http_method(), &def.request.url)?;

    if let Some(headers) = def.headers {
        for header in headers.headers {
            let name = attohttpc::header::HeaderName::from_bytes(&header.name.as_bytes())?;
            let value = attohttpc::header::HeaderValue::from_str(&header.value)?;

            request_builder = request_builder.try_header_append(name, value)?;
        }
    }

    if let Some(query) = def.query {
        for param in query.params {
            request_builder = request_builder.param(param.name, param.value);
        }
    }

    match def.body {
        None => {
            let prepared = request_builder.try_prepare()?;
            Ok(OurPreparedRequest::Empty(prepared))
        }
        Some(Content::Json(json_string)) => {
            // At this point, all variable substitutions have been made, so if the string content
            // can't be successfully parsed to JSON, this will return an Error.
            let json_value: serde_json::Value = serde_json::from_str(&json_string)?;
            let prepared = request_builder.json(&json_value)?.try_prepare()?;
            Ok(OurPreparedRequest::Bytes(prepared))
        }
        Some(Content::Text(text)) => {
            let prepared = request_builder.text(text).try_prepare()?;
            Ok(OurPreparedRequest::Text(prepared))
        }
        Some(Content::UrlEncoded(form)) => {
            // put into a Vec of tuples for serialization with serde_urlencoded
            let tuples: Vec<(String, String)> = form
                .into_iter()
                .map(|keyvalue| (keyvalue.name, keyvalue.value))
                .collect();

            let prepared = request_builder.form(&tuples)?.try_prepare()?;
            Ok(OurPreparedRequest::Bytes(prepared))
        }
    }
}

#[test]
fn test_bad_files() {
    for entry in std::fs::read_dir("test_definitions/prepare_bad").unwrap() {
        let path = entry.unwrap().path();

        let def = RequestDefinition::new(&path);
        assert!(
            def.is_ok(),
            "expected file {:?} to contain a valid RequestDefinition, but it errored",
            path.to_string_lossy()
        );

        let prepared = prepare_request(def.unwrap());
        assert!(
            prepared.is_err(),
            "expected file {:?} to error on calling prepare_request, but it was OK",
            path.to_string_lossy()
        );
    }
}

pub fn send_request(def: RequestDefinition) -> anyhow::Result<Response> {
    let prepared = prepare_request(def)?;

    let res = match prepared {
        OurPreparedRequest::Empty(mut req) => req.send(),
        OurPreparedRequest::Text(mut req) => req.send(),
        OurPreparedRequest::Bytes(mut req) => req.send(),
    }?;

    let res = transform_response(res)?;

    Ok(res)
}

fn transform_response(res: attohttpc::Response) -> anyhow::Result<Response> {
    let status_code = res.status();
    let body = res.text()?;

    let our_response = Response { body, status_code };

    Ok(our_response)
}

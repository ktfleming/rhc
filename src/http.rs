use crate::keyvalue::KeyValue;
use crate::request_definition::{Content, RequestDefinition};
use crate::response::Response;
use crate::templating::substitute;
use attohttpc::{self, body};

// Wrapper around attohttpc's PreparedRequest, in order to
// make the types simpler
enum OurPreparedRequest {
    Bytes(attohttpc::PreparedRequest<body::Bytes<Vec<u8>>>),
    Text(attohttpc::PreparedRequest<body::Text<String>>),
    Empty(attohttpc::PreparedRequest<body::Empty>),
}

fn prepare_request(
    def: RequestDefinition,
    variables: &[KeyValue],
) -> anyhow::Result<OurPreparedRequest> {
    let final_url = substitute(def.request.url, variables);

    let mut request_builder =
        attohttpc::RequestBuilder::try_new(def.request.method.to_http_method(), final_url)?;

    if let Some(headers) = def.headers {
        for header in headers.headers {
            let name = substitute(header.name, variables);
            let name = attohttpc::header::HeaderName::from_bytes(name.as_bytes())?;

            let value = substitute(header.value, variables);
            let value = attohttpc::header::HeaderValue::from_str(&value)?;

            request_builder = request_builder.try_header_append(name, value)?;
        }
    }

    if let Some(query) = def.query {
        for param in query.params {
            let name = substitute(param.name, variables);
            let value = substitute(param.value, variables);

            request_builder = request_builder.param(name, value);
        }
    }

    match def.body {
        None => {
            let prepared = request_builder.try_prepare()?;
            Ok(OurPreparedRequest::Empty(prepared))
        }
        Some(Content::Json(json)) => {
            let prepared = request_builder.json(&json)?.try_prepare()?;
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

        let prepared = prepare_request(def.unwrap(), &vec![]);
        assert!(
            prepared.is_err(),
            "expected file {:?} to error on calling prepare_request, but it was OK",
            path.to_string_lossy()
        );
    }
}

pub fn send_request(def: RequestDefinition, variables: &[KeyValue]) -> anyhow::Result<Response> {
    let prepared = prepare_request(def, variables)?;

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

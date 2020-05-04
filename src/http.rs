use crate::keyvalue::KeyValue;
use crate::request_definition::{Content, RequestDefinition};
use crate::response::Response;
use crate::templating::substitute;
use anyhow;
use attohttpc::{self, body};

// Wrapper around attohttpc's PreparedRequest, in order to
// make the types simpler
enum OurPreparedRequest {
    JsonRequest(attohttpc::PreparedRequest<body::Bytes<Vec<u8>>>),
    TextRequest(attohttpc::PreparedRequest<body::Text<String>>),
    EmptyRequest(attohttpc::PreparedRequest<body::Empty>),
}

fn prepare_request(
    def: RequestDefinition,
    variables: &Vec<KeyValue>,
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
            Ok(OurPreparedRequest::EmptyRequest(prepared))
        }
        Some(Content::Json(json)) => {
            let prepared = request_builder.json(&json)?.try_prepare()?;
            Ok(OurPreparedRequest::JsonRequest(prepared))
        }
        Some(Content::Text(text)) => {
            let prepared = request_builder.text(text).try_prepare()?;
            Ok(OurPreparedRequest::TextRequest(prepared))
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

pub fn send_request(def: RequestDefinition, variables: &Vec<KeyValue>) -> anyhow::Result<Response> {
    let prepared = prepare_request(def, variables)?;

    let res = match prepared {
        OurPreparedRequest::EmptyRequest(mut req) => req.send(),
        OurPreparedRequest::TextRequest(mut req) => req.send(),
        OurPreparedRequest::JsonRequest(mut req) => req.send(),
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

use crate::request_definition::RequestDefinition;
use crate::response::Response;
use anyhow;
use attohttpc::{self, body};

// Wrapper around attohttpc's PreparedRequest, in order to
// make the types simpler
enum OurPreparedRequest {
    StringRequest(attohttpc::PreparedRequest<body::Text<String>>),
    EmptyRequest(attohttpc::PreparedRequest<body::Empty>),
}

fn prepare_request(def: RequestDefinition) -> anyhow::Result<OurPreparedRequest> {
    let request_builder =
        attohttpc::RequestBuilder::try_new(def.request.method.to_http_method(), def.request.url)?;

    if let Some(body) = def.body {
        let prepared = request_builder.text(body.content).try_prepare()?;
        Ok(OurPreparedRequest::StringRequest(prepared))
    } else {
        let prepared = request_builder.try_prepare()?;
        Ok(OurPreparedRequest::EmptyRequest(prepared))
    }
}

pub fn send_request(def: RequestDefinition) -> anyhow::Result<Response> {
    let prepared = prepare_request(def)?;

    let res = match prepared {
        OurPreparedRequest::EmptyRequest(mut req) => req.send(),
        OurPreparedRequest::StringRequest(mut req) => req.send(),
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

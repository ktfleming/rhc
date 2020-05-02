use crate::request_definition::RequestDefinition;
use crate::response::Response;
use anyhow;
use reqwest::blocking;

/// Given a RequestDefinition, construct and send a request and
/// return our Response model
pub fn send_request(mut def: RequestDefinition) -> anyhow::Result<Response> {
    let client = blocking::Client::new();
    let url = reqwest::Url::parse(&def.request.url)?;
    let mut req = client.request(def.request.method.to_reqwest_method(), url);
    if let Some(body) = def.body.take() {
        req = req
            .body(body.content)
            .header("Content-Type", body.content_type);
    }
    let res = req.send()?;
    let res = transform_response(res)?;

    Ok(res)
}

/// Transform a reqwest Response into our own Response
fn transform_response(res: blocking::Response) -> anyhow::Result<Response> {
    let status_code = res.status();
    let body = res.text()?;

    let our_response = Response { body, status_code };

    Ok(our_response)
}

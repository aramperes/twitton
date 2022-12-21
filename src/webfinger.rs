use actix_web::{error, get, web, Responder};
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct WebfingerRequest {
    resource: String,
}

#[derive(Serialize, Debug)]
pub struct WebfingerResponse {
    subject: String,
    aliases: Vec<String>,
    links: Vec<WebfingerLink>,
}

#[derive(Serialize, Debug)]
struct WebfingerLink {
    rel: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename(serialize = "type"))]
    rel_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    href: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    template: Option<String>,
}

#[derive(Debug, Display, Error)]
#[display(fmt = "error: {}", description)]
pub struct WebfingerError {
    pub description: &'static str,
}

impl error::ResponseError for WebfingerError {}

#[get("/.well-known/webfinger")]
pub async fn finger(
    data: web::Data<crate::Environment>,
    query: web::Query<WebfingerRequest>,
) -> actix_web::Result<impl Responder, WebfingerError> {
    let admin_resource = format!("acct:{}", data.admin_username_domain);
    if query.resource == admin_resource {
        Ok(web::Json(WebfingerResponse {
            subject: admin_resource,
            aliases: vec![data.admin_profile_url.clone()],
            links: vec![
                WebfingerLink {
                    rel: "http://webfinger.net/rel/profile-page".into(),
                    rel_type: Some("text/html".into()),
                    href: Some(data.admin_profile_url.clone()),
                    template: None,
                },
                WebfingerLink {
                    rel: "self".into(),
                    rel_type: Some("application/activity+json".into()),
                    href: Some(data.admin_profile_url.clone()),
                    template: None,
                },
                WebfingerLink {
                    rel: "http://ostatus.org/schema/1.0/subscribe".into(),
                    rel_type: None,
                    href: None,
                    template: Some(data.subscribe_url.clone()),
                },
            ],
        }))
    } else {
        Err(WebfingerError { description: "404" })
    }
}

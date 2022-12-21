use anyhow::Context;

use actix_web::{error, get, guard, web, Responder};
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
struct Environment {
    web_domain: String,
    local_domain: String,
    admin_username: String,
    admin_username_domain: String,
    admin_profile_url: String,
    subscribe_url: String,
    inbox_url: String,
    admin_public_key_pem: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct WebfingerRequest {
    resource: String,
}

#[derive(Serialize, Debug)]
struct WebfingerResponse {
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
struct WebfingerError {
    description: &'static str,
}

impl error::ResponseError for WebfingerError {}

#[get("/.well-known/webfinger")]
async fn finger(
    data: web::Data<Environment>,
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

#[derive(Serialize, Debug)]
struct PubActorPublicKey {
    id: String,
    owner: String,
    #[serde(rename(serialize = "publicKeyPem"))]
    public_key_pem: String,
}

#[derive(Serialize, Debug)]
struct PubActorResponse {
    #[serde(rename(serialize = "@context"))]
    context: Vec<String>,
    id: String,
    #[serde(rename(serialize = "type"))]
    actor_type: String,
    #[serde(rename(serialize = "preferredUsername"))]
    preferred_username: String,
    inbox: String,
    #[serde(rename(serialize = "publicKey"))]
    public_key: PubActorPublicKey,
}

#[get("/user/{username}")]
async fn pub_user(
    data: web::Data<Environment>,
    username: web::Path<String>,
) -> actix_web::Result<impl Responder, WebfingerError> {
    let username = username.into_inner();
    if username == data.admin_username {
        Ok(web::Json(PubActorResponse {
            context: vec![
                "https://www.w3.org/ns/activitystreams".into(),
                "https://w3id.org/security/v1".into(),
            ],
            id: data.admin_profile_url.clone(),
            actor_type: "Person".into(),
            preferred_username: data.admin_username.clone(),
            inbox: data.inbox_url.clone(),
            public_key: PubActorPublicKey {
                id: format!("{}#main-key", data.admin_profile_url),
                owner: data.admin_profile_url.clone(),
                public_key_pem: data.admin_public_key_pem.clone(),
            },
        }))
    } else {
        Err(WebfingerError { description: "404" })
    }
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    use actix_web::{App, HttpServer};

    HttpServer::new(|| {
        let env = {
            let web_domain = get_env("WEB_DOMAIN").unwrap();
            let local_domain = get_env("LOCAL_DOMAIN").unwrap();
            let admin_username = get_env("ADMIN_USERNAME").unwrap();
            let admin_username_domain = format!("{}@{}", admin_username, local_domain);
            let admin_profile_url = format!("https://{}/user/{}", web_domain, admin_username);
            let subscribe_url = format!("https://{}/authorize_interaction?uri={{uri}}", web_domain);
            let inbox_url = format!("https://{}/inbox", web_domain);
            let admin_public_key_pem = get_env("ADMIN_PUBLIC_KEY_PEM").unwrap();

            Environment {
                web_domain,
                local_domain,
                admin_username,
                admin_username_domain,
                admin_profile_url,
                subscribe_url,
                inbox_url,
                admin_public_key_pem,
            }
        };
        App::new()
            .app_data(web::Data::new(env))
            .service(finger)
            .service(pub_user)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
    .with_context(|| "Failed to bind actix server")
}

fn get_env(name: &str) -> anyhow::Result<String> {
    std::env::var(name)
        .ok()
        .filter(env_not_empty)
        .with_context(|| format!("missing env: {}", name))
}

fn env_not_empty(s: &String) -> bool {
    !s.trim().is_empty()
}
use anyhow::Context;

#[macro_use]
extern crate log;

mod webfinger;

use actix_web::{
    get,
    http::{header::HeaderValue, StatusCode},
    post, web, HttpRequest, HttpResponse, Responder,
};
use serde::Serialize;
use webfinger::WebfingerError;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Environment {
    web_domain: String,
    local_domain: String,
    admin_username: String,
    admin_username_domain: String,
    admin_profile_url: String,
    subscribe_url: String,
    shared_inbox_url: String,
    admin_public_key_pem: String,
    admin_icon_url: Option<String>,
}

impl Environment {
    pub fn direct_inbox_url(&self, username: &str) -> String {
        format!("https://{}/user/{}/inbox", self.web_domain, username)
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
struct PubActorImage {
    #[serde(rename(serialize = "type"))]
    res_type: String,
    #[serde(rename(serialize = "mediaType"))]
    media_type: String,
    url: String,
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
    icon: Option<PubActorImage>,
    endpoints: PubActorEndpoints,
}

#[derive(Serialize, Debug)]
struct PubActorEndpoints {
    #[serde(rename(serialize = "sharedInbox"))]
    shared_inbox: String,
}

#[get("/user/{username}")]
async fn pub_user(
    req: HttpRequest,
    data: web::Data<Environment>,
    username: web::Path<String>,
) -> actix_web::Result<impl Responder, WebfingerError> {
    let username = username.into_inner();
    if username == data.admin_username {
        match req
            .headers()
            .get("accept")
            .map(HeaderValue::to_str)
            .and_then(Result::ok)
        {
            Some("application/activity+json" | "application/json") => {
                Ok(HttpResponse::build(StatusCode::OK)
                    .content_type("application/activity+json")
                    .json(PubActorResponse {
                        context: vec![
                            "https://www.w3.org/ns/activitystreams".into(),
                            "https://w3id.org/security/v1".into(),
                        ],
                        id: data.admin_profile_url.clone(),
                        actor_type: "Person".into(),
                        preferred_username: data.admin_username.clone(),
                        inbox: data.direct_inbox_url(&data.admin_username),
                        public_key: PubActorPublicKey {
                            id: format!("{}#main-key", data.admin_profile_url),
                            owner: data.admin_profile_url.clone(),
                            public_key_pem: data.admin_public_key_pem.clone(),
                        },
                        icon: data.admin_icon_url.clone().map(|url| PubActorImage {
                            res_type: "Image".into(),
                            media_type: "image/png".into(),
                            url,
                        }),
                        endpoints: PubActorEndpoints {
                            shared_inbox: data.shared_inbox_url.clone(),
                        },
                    }))
            }
            _ => Ok(HttpResponse::build(StatusCode::OK)
                .content_type("text/html")
                .body(format!("twitton // {}", username))),
        }
    } else {
        Err(WebfingerError { description: "404" })
    }
}

#[get("/")]
async fn index() -> actix_web::Result<impl Responder, WebfingerError> {
    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("text/html")
        .body("twitton :)"))
}

#[post("/inbox")]
async fn shared_inbox(
    req: HttpRequest,
    body: web::Bytes,
) -> actix_web::Result<impl Responder, WebfingerError> {
    process_inbox(req, body).await
}

#[post("/user/{username}/inbox")]
async fn direct_inbox(
    req: HttpRequest,
    body: web::Bytes,
) -> actix_web::Result<impl Responder, WebfingerError> {
    process_inbox(req, body).await
}

async fn process_inbox(
    req: HttpRequest,
    body: web::Bytes,
) -> actix_web::Result<impl Responder, WebfingerError> {
    info!(
        "headers={:?}\nbody = {}",
        req.headers(),
        std::str::from_utf8(&body).expect("failed to parse body as utf")
    );

    Ok(HttpResponse::build(StatusCode::OK)
        .content_type("application/json")
        .body("{}"))
}

#[actix_web::main]
async fn main() -> anyhow::Result<()> {
    use actix_web::{App, HttpServer};

    pretty_env_logger::init();

    HttpServer::new(|| {
        let env = {
            let web_domain = get_env("WEB_DOMAIN").unwrap();
            let local_domain = get_env("LOCAL_DOMAIN").unwrap();
            let admin_username = get_env("ADMIN_USERNAME").unwrap();
            let admin_username_domain = format!("{}@{}", admin_username, local_domain);
            let admin_profile_url = format!("https://{}/user/{}", web_domain, admin_username);
            let subscribe_url = format!("https://{}/authorize_interaction?uri={{uri}}", web_domain);
            let shared_inbox_url = format!("https://{}/inbox", web_domain);
            let admin_public_key_pem = get_env("ADMIN_PUBLIC_KEY_PEM").unwrap();
            let admin_icon_url = std::env::var("ADMIN_ICON_URL").ok();

            Environment {
                web_domain,
                local_domain,
                admin_username,
                admin_username_domain,
                admin_profile_url,
                subscribe_url,
                shared_inbox_url,
                admin_public_key_pem,
                admin_icon_url,
            }
        };
        App::new()
            .app_data(web::Data::new(env))
            .service(index)
            .service(webfinger::finger)
            .service(pub_user)
            .service(shared_inbox)
            .service(direct_inbox)
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
    .with_context(|| "Failed to bind actix server")
}

fn get_env(name: &str) -> anyhow::Result<String> {
    std::env::var(name)
        .ok()
        .filter(|s| !s.trim().is_empty())
        .with_context(|| format!("missing env: {}", name))
}

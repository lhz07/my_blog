use std::fs;

use actix_web::{HttpResponse, get, web};
use serde::{Deserialize, Serialize};
use tera::Tera;

use crate::errors::{CatError, RespError};

#[derive(Deserialize, Serialize)]
struct Friend {
    name: String,
    url: String,
    img: String,
    description: String,
}

#[derive(Deserialize, Serialize)]
struct Friends {
    friend: Vec<Friend>,
}

fn extract_friend_links() -> Result<Vec<Friend>, CatError> {
    let content = fs::read_to_string("./other_data/friends.toml")?;
    let friends: Friends = toml::from_str(&content)?;
    Ok(friends.friend)
}

#[get("/friend_links")]
pub async fn friend_links(templates: web::Data<Tera>) -> Result<HttpResponse, RespError> {
    let mut context = tera::Context::new();
    let friends = extract_friend_links().inspect_err(|e| eprintln!("{e}"))?;
    context.insert("page", "friend_links");
    context.insert("friends", &friends);
    let html = templates.render("friend_links.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

use actix_web::{HttpResponse, get, post, web};
use base64::{Engine, prelude::BASE64_URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha1::Digest;
use std::fs;
use tera::Tera;

use crate::errors::{CatError, RespError};

#[derive(Deserialize, Serialize)]
struct Friend {
    name: String,
    url: String,
    avatar: String,
    description: String,
}

#[derive(Deserialize, Serialize)]
struct Friends {
    friend: Vec<Friend>,
}

#[derive(Deserialize, Serialize, Default)]
pub struct FriendRequest {
    name: String,
    url: String,
    avatar: String,
    description: String,
    email: String,
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
fn count_files<P>(dir: P) -> std::io::Result<usize>
where
    P: AsRef<std::path::Path>,
{
    let mut count = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            count += 1;
        }
    }
    Ok(count)
}

fn write_friend_request(value: &FriendRequest) -> Result<(), CatError> {
    let mut hasher = sha1::Sha1::new();
    hasher.update(value.url.as_bytes());
    let hash = hasher.finalize();
    let file_name = BASE64_URL_SAFE_NO_PAD.encode(hash);
    let content = toml::to_string_pretty(&value).inspect_err(|e| {
        eprintln!("{e}");
    })?;
    let dir = std::path::Path::new("../friend_requests");
    if !fs::exists(dir)? {
        fs::create_dir(dir)?;
    } else if count_files(dir)? > 1000 {
        return Err(CatError::Custom("Too many friend requests".to_string()));
    }
    fs::write(format!("../friend_requests/{file_name}.toml"), content).inspect_err(|e| {
        eprintln!("{e}");
    })?;
    Ok(())
}

#[post("/api/friend-link")]
pub async fn post_link(value: web::Json<FriendRequest>) -> Result<HttpResponse, RespError> {
    write_friend_request(&value)?;
    Ok(HttpResponse::Ok().finish())
}

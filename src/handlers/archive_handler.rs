use crate::{
    CONTEXT,
    errors::{CatError, RespError},
    handlers::{
        home_handler::FrontMatter,
        post_handler::{SORT_BY_POSTED_FRONTMATTERS, render_a_post},
    },
    lock::Lock,
};
use actix_web::{HttpResponse, get, web};
use chrono::Datelike;
use indexmap::IndexMap;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use serde::Serialize;
use std::sync::{Arc, LazyLock};
use strum_macros::AsRefStr;
use tera::Tera;

#[derive(Debug, AsRefStr, FromPrimitive, Hash, PartialEq, Eq, Serialize)]
pub enum Month {
    January = 1,
    February,
    March,
    April,
    May,
    June,
    July,
    August,
    September,
    October,
    November,
    December,
}

#[derive(Debug, Serialize, Default)]
pub struct ArchiveYear {
    count: usize,
    months: ArchiveMonth,
}

impl ArchiveYear {
    fn new() -> Self {
        Self {
            count: 1,
            months: IndexMap::new(),
        }
    }
}

pub type ArchiveMonth = IndexMap<Month, Vec<Arc<FrontMatter>>>;

pub type Archives = IndexMap<i32, ArchiveYear>;

pub static ARCHIVES: LazyLock<Lock<Archives>> = LazyLock::new(|| match init_archives() {
    Ok(map) => Lock::new(map),
    Err(e) => {
        eprintln!("{e}");
        std::process::exit(1);
    }
});

pub fn init_archives() -> Result<Archives, CatError> {
    let mut archives = IndexMap::new();
    let fms = SORT_BY_POSTED_FRONTMATTERS.get();
    for fm in fms.iter() {
        let date = fm.posted.date_naive();
        let year = date.year();
        let month = date.month();
        let month =
            Month::from_u32(month).ok_or(CatError::internal(format!("Invalid month: {month}")))?;
        archives
            .entry(year)
            .and_modify(|y: &mut ArchiveYear| y.count += 1)
            .or_insert_with(ArchiveYear::new)
            .months
            .entry(month)
            .or_insert_with(Vec::new)
            .push(fm.clone());
    }
    Ok(archives)
}

#[get("/archives")]
pub async fn archive(templates: web::Data<Arc<Lock<Tera>>>) -> Result<HttpResponse, RespError> {
    let mut context = CONTEXT.clone();
    context.insert("page", "archives");
    let archives = ARCHIVES.get();
    context.insert("archives", &*archives);
    let html = templates.get().render("archives.html", &context)?;
    Ok(HttpResponse::Ok().content_type("text/html").body(html))
}

#[get("/archives/{post_name}")]
pub async fn archive_post(
    templates: web::Data<Arc<Lock<Tera>>>,
    post_name: web::Path<String>,
) -> Result<HttpResponse, RespError> {
    render_a_post(
        templates,
        post_name,
        &SORT_BY_POSTED_FRONTMATTERS,
        "/archives",
        "Archives",
        "/archives",
    )
}

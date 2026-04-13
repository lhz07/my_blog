use search_utils::{build_index::build_index, errors::SearchError, formatter};

fn main() -> Result<(), SearchError> {
    let mut arg = std::env::args();
    match arg.nth(1) {
        Some(s) if s == "fmt" => formatter::format_all(),
        _ => build_index(),
    }
}

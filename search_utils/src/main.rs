use search_utils::{build_index::build_index, errors::SearchError};

fn main() -> Result<(), SearchError> {
    build_index()
}

use my_blog::{errors::CatError, search_utils::build_index::build_index};

fn main() -> Result<(), CatError> {
    build_index()
}

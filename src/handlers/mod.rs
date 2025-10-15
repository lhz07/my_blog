mod about_handler;
mod friend_links_handler;
pub mod home_handler;
pub mod post_handler;
mod search_handler;

pub use about_handler::about;
pub use friend_links_handler::{FriendRequest, friend_links, post_link};
pub use home_handler::index;
pub use home_handler::page;
pub use post_handler::post;
pub use search_handler::{search, search_lucky};

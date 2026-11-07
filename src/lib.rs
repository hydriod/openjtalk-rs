pub mod error;
pub mod openjtalk;

pub use openjtalk::{mecab_dict_index, run_frontend, set_user_dict, unset_user_dict};

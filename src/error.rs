//! このクレート共通のエラー型。
//!
//! 元は espnet-burn の `crate::error::Error` を参照していたが、分離に伴い
//! Open JTalk のフロントエンドで使う種別だけをここへ移した。

#[derive(thiserror::Error, Debug, Clone)]
pub enum Error {
    #[error("argument error: {0}")]
    Argument(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("tokenizer error: {0}")]
    Tokenizer(String),

    #[error("request error: {0}")]
    Request(String),
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e.to_string())
    }
}

impl From<ureq::Error> for Error {
    fn from(e: ureq::Error) -> Self {
        Error::Request(e.to_string())
    }
}

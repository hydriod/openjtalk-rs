mod dict;

use std::path::Path;
use std::sync::OnceLock;

use crate::{error::Error, openjtalk::dict::resolve_dict_dir};

#[cxx::bridge]
mod ffi {
    unsafe extern "C++" {
        include!("openjtalk_bridge.h");

        fn run_frontend(text: &str, use_unvoiced_vowel: bool, output: &mut Vec<String>);
        // fn make_label(text: &str, output: &mut Vec<String>);
        fn init_dictionary(dict_dir_str: &str) -> bool;
        fn create_user_dict(dict_dir_str: &str, csv_path: &str, out_path: &str) -> bool;
        fn set_user_dict(dict_dir_str: &str, user_dict_path: &str) -> bool;
        fn unset_user_dict(dict_dir_str: &str) -> bool;
    }
}

static INIT: OnceLock<Result<(), Error>> = OnceLock::new();

fn ensure_dict_loaded() -> Result<(), Error> {
    INIT.get_or_init(|| {
        let dict_dir_str = dict_dir_str()?;
        if ffi::init_dictionary(&dict_dir_str) {
            Ok(())
        } else {
            Err(Error::Io(format!(
                "Failed to load dictionary from \"{}\"",
                dict_dir_str
            )))
        }
    })
    .clone()
}

/// 解決済みの辞書ディレクトリを UTF-8 文字列で返す。
///
/// C++ 側の再ロード（ユーザー辞書の適用／解除）は辞書ディレクトリを
/// 引数に取るため、`ensure_dict_loaded` の初回ロード以外でも必要になる。
fn dict_dir_str() -> Result<String, Error> {
    let dict_dir = resolve_dict_dir()?;
    dict_dir
        .to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Tokenizer("Dictionary path is not valid UTF-8".to_string()))
}

/// パスを UTF-8 文字列へ変換する（FFI へ渡すための共通処理）。
fn path_to_str(path: &Path, what: &str) -> Result<String, Error> {
    path.to_str()
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Argument(format!("{} path is not valid UTF-8: {:?}", what, path)))
}

/// Converts Japanese text to HTS-format labels using OpenJTalk.
///
/// This is equivalent to pyopenjtalk.make_label() but calls OpenJTalk C++ directly.
///
/// # Arguments
/// * `text` - Input Japanese text (UTF-8)
///
/// # Returns
/// Vec of label strings in HTS Full Context Label format
///
/// # Example
/// ```ignore
/// let labels = make_label("こんにちは").unwrap();
/// for label in labels {
///     println!("{}", label);
/// }
/// ```
///
/// # Errors
/// Returns error if text processing or MeCab analysis fails
/*
pub fn make_label(text: &str) -> Result<Vec<String>, String> {
    let mut output = Vec::new();
    ffi::make_label(text, &mut output);
    if output.is_empty() {
        Err("Failed to process text or no features generated".to_string())
    } else {
        Ok(output)
    }
}
*/

/// Runs the full OpenJTalk frontend with control over unvoiced vowel handling.
///
/// This is equivalent to pyopenjtalk.run_frontend() but calls OpenJTalk C++ directly.
/// This function gives more control than make_label() by allowing unvoiced vowel preservation.
///
/// # Arguments
/// * `text` - Input Japanese text (UTF-8)
/// * `use_unvoiced_vowel` - If true, preserves unvoiced vowels (A/I/U/E/O) as-is.
///                          If false, converts them to lowercase (a/i/u/e/o).
///
/// # Returns
/// Vec of label strings in HTS Full Context Label format
///
/// # Example
/// ```ignore
/// let labels = run_frontend("おはようございます", false).unwrap();
/// ```
///
/// # Errors
/// Returns error if text processing or MeCab analysis fails
pub fn run_frontend(text: &str, use_unvoiced_vowel: bool) -> Result<Vec<String>, Error> {
    ensure_dict_loaded()?;
    let mut output = Vec::new();

    ffi::run_frontend(text, use_unvoiced_vowel, &mut output);
    if output.is_empty() {
        Err(Error::Tokenizer(
            "Failed to process text or no features generated".to_string(),
        ))
    } else {
        Ok(output)
    }
}

/// ユーザー辞書の CSV を MeCab のユーザー辞書ファイルへコンパイルする。
///
/// pyopenjtalk の `create_user_dict()`（本家の改名後は `mecab_dict_index()`）
/// 相当。実体は Open JTalk 同梱の `mecab_dict_index` で、システム辞書の
/// あるディレクトリを基準に CSV をバイナリ辞書へ変換する。
/// コンパイル自体はグローバルな MeCab インスタンスに触れないため、
/// 解析処理とは競合しない（C++ 側のコメント参照）。
///
/// # Errors
/// CSV が存在しない、パスが UTF-8 でない、またはコンパイルに失敗した場合。
pub fn mecab_dict_index(csv_path: &Path, out_path: &Path) -> Result<(), Error> {
    if !csv_path.is_file() {
        return Err(Error::Argument(format!(
            "no such file or directory: {:?}",
            csv_path
        )));
    }
    ensure_dict_loaded()?;
    let dict_dir = dict_dir_str()?;
    let csv = path_to_str(csv_path, "csv")?;
    let out = path_to_str(out_path, "output dictionary")?;

    if !ffi::create_user_dict(&dict_dir, &csv, &out) {
        return Err(Error::Io(format!(
            "Failed to create user dictionary from {:?}",
            csv_path
        )));
    }
    if !out_path.is_file() {
        return Err(Error::Io(format!(
            "User dictionary was not created: {:?}",
            out_path
        )));
    }
    Ok(())
}

/// コンパイル済みユーザー辞書を適用する。
///
/// pyopenjtalk の `set_user_dict()` 相当。グローバルな MeCab インスタンスを
/// ユーザー辞書付きで作り直すため、以降の `run_frontend` の結果が変わる。
/// 差し替えは C++ 側で解析処理と同じ Mutex の保護下で行われ、解析中に
/// インスタンスが差し替わることはない。
///
/// # Errors
/// 辞書ファイルが存在しない、パスが UTF-8 でない、または再ロードに失敗した場合。
pub fn set_user_dict(path: &Path) -> Result<(), Error> {
    if !path.is_file() {
        return Err(Error::Argument(format!(
            "no such file or directory: {:?}",
            path
        )));
    }
    ensure_dict_loaded()?;
    let dict_dir = dict_dir_str()?;
    let user_dict = path_to_str(path, "user dictionary")?;

    if ffi::set_user_dict(&dict_dir, &user_dict) {
        Ok(())
    } else {
        Err(Error::Io(format!(
            "Failed to load user dictionary from {:?}",
            path
        )))
    }
}

/// ユーザー辞書の適用を解除し、システム辞書のみの状態へ戻す。
///
/// pyopenjtalk の `unset_user_dict()` 相当。スレッド安全性の考え方は
/// [`set_user_dict`] と同じ。
///
/// # Errors
/// 再ロードに失敗した場合。
pub fn unset_user_dict() -> Result<(), Error> {
    ensure_dict_loaded()?;
    let dict_dir = dict_dir_str()?;

    if ffi::unset_user_dict(&dict_dir) {
        Ok(())
    } else {
        Err(Error::Io(
            "Failed to reload dictionary without user dictionary".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_run_frontend_with_unvoiced_vowel() {
        let result = run_frontend("おはよう", true);
        assert!(result.is_ok());
    }

    #[test]
    fn test_run_frontend_without_unvoiced_vowel() {
        let result = run_frontend("あ", false);
        assert!(result.is_ok());
    }

    /*
    #[test]
    fn test_make_label_simple() {
        let result = make_label("あ");
        assert!(result.is_ok());
        let labels = result.unwrap();
        assert!(!labels.is_empty());
        println!("{}", labels.join(" "));
    }

    #[test]
    fn test_make_label_complex() {
        let result = make_label("こんにちは");
        assert!(result.is_ok());
        let labels = result.unwrap();
        assert!(labels.len() > 0);
        for label in &labels {
            assert!(!label.is_empty());
        }
    }
    */
}

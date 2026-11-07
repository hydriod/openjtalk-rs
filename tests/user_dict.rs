//! ユーザー辞書まわり（`mecab_dict_index` / `set_user_dict` /
//! `unset_user_dict`）の結合テスト。
//!
//! この3関数はプロセス内グローバルの MeCab インスタンスを差し替えるため、
//! 並行実行される他のテストと競合しないよう、検証は単一のテスト関数に
//! まとめ、順番に実施する（辞書適用→確認→解除→復元の1シナリオ）。
//! 辞書が必要なため、実行時は環境変数 `OPEN_JTALK_DICT_DIR` に
//! `open_jtalk_dic_utf_8-1.11` のパスを指定すること。

use std::fs;
use std::path::PathBuf;

use openjtalk_rs::openjtalk::{mecab_dict_index, run_frontend, set_user_dict, unset_user_dict};

/// ラベル列から音素列だけを取り出す（HTS ラベルの `-<音素>+` 部分）。
fn phonemes(labels: &[String]) -> Vec<String> {
    labels
        .iter()
        .filter_map(|label| {
            let start = label.find('-')? + 1;
            let end = label[start..].find('+')? + start;
            Some(label[start..end].to_string())
        })
        .collect()
}

/// 音素列に子音＋母音の並び（例: "g","u" ＝ グ）があるかどうか。
fn has_phoneme_pair(labels: &[String], first: &str, second: &str) -> bool {
    phonemes(labels)
        .windows(2)
        .any(|w| w[0] == first && w[1] == second)
}

#[test]
fn user_dict_roundtrip() {
    let text = "テスト単語を読む";

    // 一時ディレクトリはクレート内の target 配下に作る
    // （/tmp は小さな tmpfs のため使わない）。終わったら掃除する。
    let tmp_dir: PathBuf = std::env::current_dir()
        .expect("current_dir should be available")
        .join("target")
        .join(format!("user-dict-test-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp_dir);
    fs::create_dir_all(&tmp_dir).expect("failed to create temp dir");
    let csv_path = tmp_dir.join("user.csv");
    let dic_path = tmp_dir.join("user.dic");

    // ① ユーザー辞書なしの結果を基準にする
    // なおラベル単位の音素は子音と母音が分かれるため、「グ」は "g","u" の
    // 2音素として現れる（g2p の "gu" とは粒度が違う）。
    let before = run_frontend(text, false).expect("run_frontend should succeed before user dict");
    assert!(
        !has_phoneme_pair(&before, "g", "u"),
        "適用前の音素に g+u（グ）は含まれないはずです: {:?}",
        phonemes(&before)
    );

    // ② CSV からユーザー辞書をコンパイルできること
    // コストは 1000 にする: この辞書（naist-jdic）では 5000 だと
    // 「テスト＋単語」のシステム辞書側の経路にコストで負けて読みが
    // 変わらないことを、旧 pyopenjtalk でも同様に確認済み。
    fs::write(
        &csv_path,
        "テスト単語,1285,1285,1000,名詞,一般,*,*,*,*,テストタング,テストタング,テストタング,2/5,*\n",
    )
    .expect("failed to write user dictionary csv");
    mecab_dict_index(&csv_path, &dic_path).expect("mecab_dict_index should compile the csv");
    assert!(
        dic_path.is_file(),
        "ユーザー辞書ファイルが生成されていません"
    );
    assert!(
        fs::metadata(&dic_path).expect("metadata").len() > 0,
        "ユーザー辞書ファイルが空です"
    );

    // ③ 適用すると読み（＝ラベル）が変わること
    set_user_dict(&dic_path).expect("set_user_dict should succeed");
    let after = run_frontend(text, false).expect("run_frontend should succeed with user dict");
    assert_ne!(before, after, "ユーザー辞書適用でラベルが変わるはずです");
    assert!(
        has_phoneme_pair(&after, "g", "u"),
        "適用後は「テストタング」の g+u（グ）が現れるはずです: {:?}",
        phonemes(&after)
    );

    // ④ 解除すると元の結果に戻ること
    unset_user_dict().expect("unset_user_dict should succeed");
    let restored =
        run_frontend(text, false).expect("run_frontend should succeed after unset_user_dict");
    assert_eq!(before, restored, "解除後は元のラベルに戻るはずです");

    let _ = fs::remove_dir_all(&tmp_dir);
}

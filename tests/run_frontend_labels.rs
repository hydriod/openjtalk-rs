//! `run_frontend` の出力ラベルが旧 pyopenjtalk（VOICEVOX fork）と
//! バイト一致することを確認する結合テスト。
//!
//! 期待値は espnet-burn 内蔵版で実測し、旧 pyopenjtalk と一致済みのもの。
//! 辞書が必要なため、実行時は環境変数 `OPEN_JTALK_DICT_DIR` に
//! `open_jtalk_dic_utf_8-1.11` のパスを指定すること（未指定の場合は
//! 実行ファイル隣接の辞書か自動ダウンロードにフォールバックする）。

use openjtalk_rs::openjtalk::run_frontend;

#[test]
fn run_frontend_returns_expected_labels() {
    let labels = run_frontend("今日はいい天気ですね", false)
        .expect("run_frontend should succeed with a valid dictionary");

    assert_eq!(labels.len(), 20, "ラベル件数が期待と異なります");
    assert_eq!(
        labels[0],
        "xx^xx-sil+ky=o/A:xx+xx+xx/B:xx-xx_xx/C:xx_xx+xx/D:02+xx_xx/E:xx_xx!xx_xx-xx/F:xx_xx#xx_xx@xx_xx|xx_xx/G:3_1%0_xx_xx/H:xx_xx/I:xx-xx@xx+xx&xx-xx|xx+xx/J:3_11/K:1+3-11",
        "先頭ラベルが旧 pyopenjtalk の出力と一致しません"
    );
}

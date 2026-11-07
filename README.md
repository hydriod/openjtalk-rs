# openjtalk-rs

Open JTalk の Rust ラッパークレート。日本語テキストの形態素解析と HTS フルコンテキストラベル生成を、CXX FFI 経由で提供します。[espnet-burn](https://github.com/hydriod/espnet-burn) から OpenJTalk 部分だけを切り出して独立させたものです。

## 機能

- `run_frontend(text, use_unvoiced_vowel)` — pyopenjtalk の `extract_fullcontext` 相当のフルコンテキストラベル生成
- `mecab_dict_index(csv, out)` — ユーザー辞書のコンパイル（pyopenjtalk の `create_user_dict` 相当）
- `set_user_dict(path)` / `unset_user_dict()` — ユーザー辞書の適用と解除

## 辞書

実行時に Open JTalk 辞書（open_jtalk_dic_utf_8-1.11）が必要です。解決順は次の通りです。

1. 環境変数 `OPEN_JTALK_DICT_DIR`
2. 実行ファイルと同階層の `open_jtalk_dic_utf_8-1.11/`
3. 自動ダウンロード（ureq を使用するため、プロキシ環境変数は読みません）

## テスト

```bash
OPEN_JTALK_DICT_DIR=<辞書へのパス> cargo test
```

## 同梱物について

`lib/open_jtalk/` は r9y9/open_jtalk のソースを通常ファイルとして同梱したものです（現時点では git サブモジュールではありません）。ただし次のファイルは、ビルドにも実行にも使わないため、このリポジトリには含めていません。完全なファイルは上流リポジトリを参照してください。

- MeCab 辞書の元データ：`lib/open_jtalk/src/mecab-naist-jdic/` の `matrix.def`、`naist-jdic.csv`、`unidic-csj.csv`（大容量のため）
- EUC-JP / Shift_JIS 用のルールヘッダ18件（`*_rule_euc_jp.h`、`*_rule_shift_jis.h`）：このクレートは UTF-8 前提でビルドするため未使用で、文字コードが UTF-8 でないため転送方式の都合で除外

## ライセンス

Apache-2.0。同梱する Open JTalk 本体のライセンスは `lib/open_jtalk/COPYING` を参照してください。

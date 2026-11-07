# AGENTS.md

Open JTalk の Rust ラッパークレート。espnet-burn から OpenJTalk 部分だけを切り出して独立させたもの（フェーズ1：切り出し。フェーズ3：ユーザー辞書まわりの FFI を追加。espnet-burn 本体は未変更）。

## 構成

- `src/lib.rs` — クレートルート。`pub mod openjtalk;` と `pub use openjtalk::run_frontend;`、独自の `pub mod error;` を持つ。
- `src/openjtalk/mod.rs` — CXX ブリッジ定義と `run_frontend(text, use_unvoiced_vowel)`。pyopenjtalk の `extract_fullcontext` 相当は `run_frontend(text, false)`。辞書の初回ロードは `OnceLock` でキャッシュする。**cxx ブリッジはこのファイル内にあるので、`build.rs` の bridge パスは `src/openjtalk/mod.rs` のまま変えないこと。**
  - ユーザー辞書まわりの公開 API もここにある：`mecab_dict_index(csv, out)`（pyopenjtalk の `create_user_dict` 相当）、`set_user_dict(path)`、`unset_user_dict()`。いずれも `src/lib.rs` から再公開する。
- `src/openjtalk/dict.rs` — 辞書ディレクトリの解決と自動ダウンロード（ureq / tar / flate2）。
- `src/error.rs` — このクレート独自の `Error`（thiserror、Argument / Io / Tokenizer / Request）。元は espnet-burn の `crate::error::Error` を参照していた。
- `lib/openjtalk_bridge.cpp` / `.h` — C++ 側ブリッジ。グローバルな MeCab インスタンス（`g_mecab`）へのアクセスはすべて `std::mutex`（`g_mecab_call_mutex`）で直列化する（`MECAB_WITHOUT_MUTEX_LOCK` 前提のため）。初回初期化はかつて `std::call_once` だったが、ユーザー辞書の再ロード（`set_user_dict` / `unset_user_dict`）と両立しないため廃止し、同じ Mutex の保護下で「初回ロード／再ロード」を行う方式に変えた。解析（`run_frontend`）も同じ Mutex を保持して実行されるため、解析中にインスタンスが差し替わることはなく、この1本のロックで十分（根拠は bridge.cpp 冒頭の設計コメントにも記載）。
  - 辞書コンパイル（`create_user_dict` → `mecab_dict_index`）は `g_mecab` に触れない独立処理のため、ロックの対象外。
- `lib/mecab_load_ex.cpp` — ユーザー辞書付きロード（pyopenjtalk の Cython 実装 `Mecab_load_ex` 相当）。`MeCab::createModel` に `-d` 辞書dir と `-u` ユーザー辞書を渡して model / tagger / lattice を作り直す。正確なクラス定義が要るため、`mecab.h` を include する翻訳単位として bridge から分離した。呼び出しは bridge 側が Mutex を保持した状態で行う。
- `lib/open_jtalk/` — Open JTalk 本体（r9y9/open_jtalk、上流コミット 462fc38）のソースを通常ファイルとして同梱したもの（git サブモジュールではない）。GitHub リポジトリ（hydriod/openjtalk-rs）では、push 方式の制限のため次のファイルを含めていない：`src/mecab-naist-jdic/` の大容量ファイル3件（`matrix.def`、`naist-jdic.csv`、`unidic-csj.csv`、計約109MB。辞書の元データでビルド・実行とも未使用、実行時辞書は別途ダウンロードされる）と、EUC-JP／Shift_JIS 用ルールヘッダ18件（`*_rule_euc_jp.h`／`*_rule_shift_jis.h`。UTF-8 前提のこのクレートでは未使用で、文字コードが UTF-8 でないため）。ローカル作業コピーには完全なファイルが残っている。
- `build.rs` — `cxx_build::bridge("src/openjtalk/mod.rs")` でブリッジを生成し、ブリッジと Open JTalk の C/C++ ソース（MeCab 本体・NJD・JPCommon 等）をコンパイルする。学習等のツール群は除外するが、辞書コンパイラ（`dictionary_compiler.cpp`）だけはユーザー辞書のコンパイルに必要なため含める。CMake は不要。

## 依存

- 実行時: `cxx` / `tar` / `flate2` / `ureq`（辞書の自動ダウンロード用）/ `thiserror`
- ビルド時: `cxx-build` のみ。**Burn 系の依存は入れない。**

## 辞書と環境変数

辞書ディレクトリは `src/openjtalk/dict.rs` が次の順で解決する:

1. 環境変数 `OPEN_JTALK_DICT_DIR`（有効なディレクトリならそれを使う）
2. 実行ファイルと同階層の `open_jtalk_dic_utf_8-1.11/`
3. 無ければ GitHub リリースから自動ダウンロードして展開

注意: 自動ダウンロードは ureq を使うためプロキシ環境変数を読まない。プロキシ必須の環境では `OPEN_JTALK_DICT_DIR` で手動配置した辞書を指定すること（既知の宿題、急ぎではない）。

## ビルドと検証

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo fmt --check
OPEN_JTALK_DICT_DIR=<open_jtalk_dic_utf_8-1.11 へのパス> cargo test
```

- 単体テスト: `src/openjtalk/mod.rs` 内の `run_frontend` テスト2件。
- 結合テスト: `tests/run_frontend_labels.rs`。「今日はいい天気ですね」が20件のラベルを返し、先頭ラベルが旧 pyopenjtalk の出力とバイト一致することを確認する。
- 結合テスト: `tests/user_dict.rs`。ユーザー辞書のコンパイル→適用で読みが変わる→解除で元に戻る、を単一テスト関数で検証する（グローバルな MeCab を触るため並行実行と競合させない構成）。CSV のコストは 1000：この辞書では 5000 だとシステム辞書側の経路にコストで負けて読みが変わらない（旧 pyopenjtalk でも同現象を確認済み）。
- `target/` はクレート内に作る（`/tmp` は小さな tmpfs で溢れるため使わない）。

## 規約

- コメントやドキュメントは日本語（espnet-burn の規約に合わせる）。ベンダード／派生コード内も同様。
- 失敗し得る公開 API は `Result<T, Error>` を返す。

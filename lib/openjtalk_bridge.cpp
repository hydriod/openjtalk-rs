#include "openjtalk_bridge.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <atomic>
#include <mutex>
#include <string>

// Declare opaque C structures
typedef struct _Mecab Mecab;
typedef struct _NJD NJD;
typedef struct _JPCommon JPCommon;

extern "C"
{
  // Forward declare Mecab functions (avoid including mecab.h due to C++ template issues)
  void Mecab_initialize(Mecab *mecab);
  bool Mecab_load(Mecab *m, const char *dicdir);
  // ユーザー辞書付きロード（lib/mecab_load_ex.cpp で実装。pyopenjtalk の Mecab_load_ex 相当）
  bool Mecab_load_ex(Mecab *m, const char *dicdir, const char *userdic);
  // 辞書コンパイラ本体（lib/open_jtalk/src/mecab/src/dictionary_compiler.cpp）
  int mecab_dict_index(int argc, char **argv);
  void Mecab_analysis(Mecab *mecab, const char *str);
  int Mecab_get_size(Mecab *mecab);
  char **Mecab_get_feature(Mecab *mecab);
  void Mecab_clear(Mecab *mecab);

  // Declare all other OpenJTalk functions we need
  void NJD_initialize(NJD *njd);
  void NJD_clear(NJD *njd);
  void text2mecab(char *output, const char *input);
  void mecab2njd(NJD *njd, char **feature, int size);
  void njd_set_pronunciation(NJD *njd);
  void njd_set_digit(NJD *njd);
  void njd_set_accent_phrase(NJD *njd);
  void njd_set_accent_type(NJD *njd);
  void njd_set_unvoiced_vowel(NJD *njd);
  void njd_set_long_vowel(NJD *njd);
  void JPCommon_initialize(JPCommon *jpcommon);
  void JPCommon_clear(JPCommon *jpcommon);
  void njd2jpcommon(JPCommon *jpcommon, NJD *njd);
  void JPCommon_make_label(JPCommon *jpcommon);
  int JPCommon_get_label_size(JPCommon *jpcommon);
  char **JPCommon_get_label_feature(JPCommon *jpcommon);
}

// We declare Mecab/NJD/JPCommon as `char` arrays to hold their data
// This avoids needing to include their header files
#define MECAB_SIZE 512    // Approximate size - adjust if needed
#define NJD_SIZE 1024     // Approximate size - adjust if needed
#define JPCOMMON_SIZE 512 // Approximate size - adjust if needed

// Max buffer size for text2mecab output
#define MECAB_OUTPUT_MAX 8192

namespace
{
  Mecab *g_mecab = nullptr;
  std::atomic<bool> g_load_success{false};
  // 初期化で確定したシステム辞書ディレクトリ。再ロード時にも同じものを使う。
  // g_mecab_call_mutex を保持している間だけ読み書きする。
  std::string g_dict_dir;
  std::mutex g_mecab_call_mutex; // MECAB_WITHOUT_MUTEX_LOCK 対策

  // スレッド安全性の設計（ユーザー辞書の再ロードを含む）:
  // MeCab 本体は MECAB_WITHOUT_MUTEX_LOCK でビルドされており、内部ロックを
  // 持たない。したがって g_mecab へのアクセスはすべて、このファイルの
  // g_mecab_call_mutex で直列化する。
  // - run_frontend は解析全体（Mecab_analysis から特徴量の取り出しまで）を
  //   ロック保持下で実行する。
  // - set_user_dict / unset_user_dict は同じロックを取ってから
  //   Mecab_load_ex / Mecab_load で model / tagger / lattice を作り直す。
  //   解析側がロックを離すまで差し替えは始まらず、差し替え中にも解析は
  //   入らないため、解放済みインスタンスへのアクセスは起こらない。
  // - init_dictionary も同じロックの下で初回ロードだけを行う
  //   （旧実装の std::call_once は再ロードと両立しないため廃止した）。
  // ロックが1本で競合相手も g_mecab 利用者だけなので、この構成で十分。
  // 辞書コンパイル（create_user_dict）は g_mecab に触れない独立処理のため
  // ロックの対象外とする。

  // g_mecab を未確保なら確保して初期化する。呼び出し側がロックを保持すること。
  bool ensure_mecab_allocated()
  {
    if (g_mecab)
    {
      return true;
    }
    g_mecab = (Mecab *)malloc(MECAB_SIZE);
    if (!g_mecab)
    {
      return false;
    }
    Mecab_initialize(g_mecab);
    return true;
  }
}

bool init_dictionary(rust::Str dict_dir)
{
  // 直接初期化でdict_dirのstd::stringを生成
  std::string dict_dir_str(dict_dir);
  std::lock_guard<std::mutex> lock(g_mecab_call_mutex);
  if (g_load_success)
  {
    // 初回ロード済み。再ロードは set_user_dict / unset_user_dict の責務であり、
    // ここでは旧来の call_once と同じく二度目以降を成功として扱う。
    return true;
  }
  if (!ensure_mecab_allocated())
  {
    g_load_success = false;
    return false;
  }
  g_dict_dir = dict_dir_str;
  g_load_success = (Mecab_load(g_mecab, g_dict_dir.c_str()) != 0);
  return g_load_success.load();
}

bool create_user_dict(rust::Str dict_dir, rust::Str csv_path, rust::Str out_path)
{
  // argv の並びは pyopenjtalk（VOICEVOX fork）の CreateUserDict と同一にする:
  //   mecab-dict-index -d <辞書dir> -u <出力先> -f utf-8 -t utf-8 <csv>
  std::string dict_dir_str(dict_dir);
  std::string csv_path_str(csv_path);
  std::string out_path_str(out_path);
  std::string args[10] = {
      "mecab-dict-index",
      "-d",
      dict_dir_str,
      "-u",
      out_path_str,
      "-f",
      "utf-8",
      "-t",
      "utf-8",
      csv_path_str,
  };
  char *argv[10];
  for (int i = 0; i < 10; i++)
  {
    argv[i] = &args[i][0];
  }
  // 注意: mecab_dict_index の戻り値は直感と逆で、0 が成功を表す
  // （pyopenjtalk 側のコメント "mecab load returns 1 if success, but
  //   mecab_dict_index return the opposite" と同じ事情）。
  return mecab_dict_index(10, argv) == 0;
}

bool set_user_dict(rust::Str dict_dir, rust::Str user_dict_path)
{
  std::string dict_dir_str(dict_dir);
  std::string user_dict_path_str(user_dict_path);
  std::lock_guard<std::mutex> lock(g_mecab_call_mutex);
  if (!ensure_mecab_allocated())
  {
    g_load_success = false;
    return false;
  }
  g_dict_dir = dict_dir_str;
  // Mecab_load_ex が内部で Mecab_clear してから作り直す。ロック保持下なので
  // 解析処理と競合しない（冒頭の設計コメント参照）。
  g_load_success = Mecab_load_ex(g_mecab, g_dict_dir.c_str(), user_dict_path_str.c_str());
  return g_load_success.load();
}

bool unset_user_dict(rust::Str dict_dir)
{
  std::string dict_dir_str(dict_dir);
  std::lock_guard<std::mutex> lock(g_mecab_call_mutex);
  if (!ensure_mecab_allocated())
  {
    g_load_success = false;
    return false;
  }
  g_dict_dir = dict_dir_str;
  // ユーザー辞書なしの通常ロードへ戻す（Mecab_load は内部で Mecab_clear する）。
  g_load_success = (Mecab_load(g_mecab, g_dict_dir.c_str()) != 0);
  return g_load_success.load();
}

// OpenJTalk frontend: text -> make_label -> labels (HTS format)
void run_frontend(
    rust::Str text,
    bool use_unvoiced_vowel,
    rust::Vec<rust::String> &output)
{
  if (!g_load_success)
  {
    return;
  }
  std::string text_str(text);
  std::lock_guard<std::mutex> lock(g_mecab_call_mutex);

  // Allocate storage for structures - we use opaque pointers
  // The memory will be managed by the C functions
  NJD *njd = (NJD *)malloc(NJD_SIZE);
  JPCommon *jpcommon = (JPCommon *)malloc(JPCOMMON_SIZE);

  if (!njd || !jpcommon)
  {
    free(njd);
    free(jpcommon);
    return;
  }

  NJD_initialize(njd);
  JPCommon_initialize(jpcommon);

  // Step 1: text -> MeCab features
  char mecab_buffer[MECAB_OUTPUT_MAX];
  text2mecab(mecab_buffer, text_str.c_str());

  if (mecab_buffer[0] == '\0')
  {
    NJD_clear(njd);
    JPCommon_clear(jpcommon);
    free(njd);
    free(jpcommon);
    return;
  }

  // Step 2: MeCab analysis
  Mecab_analysis(g_mecab, mecab_buffer);

  // Step 3: MeCab -> NJD
  int size = Mecab_get_size(g_mecab);
  char **features = Mecab_get_feature(g_mecab);
  if (!features || size <= 0)
  {
    NJD_clear(njd);
    JPCommon_clear(jpcommon);
    free(njd);
    free(jpcommon);
    return;
  }

  mecab2njd(njd, features, size);

  // Step 4: Apply NJD rules
  njd_set_pronunciation(njd);
  njd_set_digit(njd);
  njd_set_accent_phrase(njd);
  njd_set_accent_type(njd);
  if (!use_unvoiced_vowel)
  {
    njd_set_unvoiced_vowel(njd);
  }
  njd_set_long_vowel(njd);

  // Step 5: NJD -> JPCommon
  njd2jpcommon(jpcommon, njd);

  // Step 6: Generate labels
  JPCommon_make_label(jpcommon);

  // Step 7: Extract label features
  int label_size = JPCommon_get_label_size(jpcommon);
  char **label_features = JPCommon_get_label_feature(jpcommon);

  output.clear();
  for (int i = 0; i < label_size; i++)
  {
    output.push_back(rust::String(label_features[i]));
  }

  // Cleanup
  NJD_clear(njd);
  JPCommon_clear(jpcommon);
  free(njd);
  free(jpcommon);
}

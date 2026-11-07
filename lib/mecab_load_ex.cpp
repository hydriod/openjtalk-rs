// ユーザー辞書付きで MeCab をロードするための小さな翻訳単位。
//
// pyopenjtalk（VOICEVOX fork）の openjtalk.pyx にあった Mecab_load_ex の
// Cython 実装を C++ へ移したもの。openjtalk_bridge.cpp は mecab.h の
// インクルードを避けて不透明な構造体として MeCab を扱っているため、
// MeCab::Model / Tagger / Lattice の正確なクラス定義が要るこの処理だけを、
// mecab.h を include する本ファイルへ分離した（mecab.cpp 自身も同じ流儀）。
//
// 呼び出し側（openjtalk_bridge.cpp）は g_mecab_call_mutex を保持した状態で
// 本関数を呼ぶこと。内部で Mecab_clear により旧 model / tagger / lattice を
// 解放してから作り直すため、解析処理と競合させてはいけない。

#include <string.h>

#include "mecab.h"

// Mecab_load / Mecab_clear は mecab.h が宣言する（戻り値は BOOL = int）。
// ここでは再宣言せず、ヘッダの宣言をそのまま使う。

// Mecab_load_ex 相当: userdic が空でなければ createModel に "-d" dicdir と
// "-u" userdic を渡し、そうでなければ通常の Mecab_load に委ねる。
// argv は pyopenjtalk の実装と同じ固定配列で、createModel が読み終えるまで
// 生存していればよい（ポインタは保持されない）。
extern "C" bool Mecab_load_ex(Mecab *m, const char *dicdir, const char *userdic)
{
  if (userdic == NULL || strlen(userdic) == 0)
  {
    return Mecab_load(m, dicdir) != 0;
  }

  if (m == NULL || dicdir == NULL || strlen(dicdir) == 0)
  {
    return false;
  }

  Mecab_clear(m);

  char *argv[5] = {
      (char *)"mecab",
      (char *)"-d",
      (char *)dicdir,
      (char *)"-u",
      (char *)userdic,
  };
  MeCab::Model *model = MeCab::createModel(5, argv);
  if (model == NULL)
  {
    return false;
  }
  m->model = model;

  MeCab::Tagger *tagger = model->createTagger();
  if (tagger == NULL)
  {
    Mecab_clear(m);
    return false;
  }
  m->tagger = tagger;

  MeCab::Lattice *lattice = model->createLattice();
  if (lattice == NULL)
  {
    Mecab_clear(m);
    return false;
  }
  m->lattice = lattice;
  return true;
}

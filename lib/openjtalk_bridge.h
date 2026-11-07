#pragma once

#include "rust/cxx.h"
#include <vector>
#include <string>

// Loads MeCab dictionary.
bool init_dictionary(rust::Str dict_dir);

// Converts Japanese text to HTS-format labels using OpenJTalk.
// use_unvoiced_vowel: if true, preserves unvoiced vowels; if false, converts to lowercase.
void run_frontend(
    rust::Str text,
    bool use_unvoiced_vowel,
    rust::Vec<rust::String> &output);

// Compiles a user dictionary CSV into a MeCab user dictionary file.
// Wraps mecab_dict_index() from dictionary_compiler.cpp; returns true on success.
// Equivalent to pyopenjtalk's create_user_dict().
bool create_user_dict(rust::Str dict_dir, rust::Str csv_path, rust::Str out_path);

// Reloads the global MeCab instance with the given user dictionary applied.
// Equivalent to pyopenjtalk's set_user_dict().
bool set_user_dict(rust::Str dict_dir, rust::Str user_dict_path);

// Reloads the global MeCab instance without a user dictionary.
// Equivalent to pyopenjtalk's unset_user_dict().
bool unset_user_dict(rust::Str dict_dir);

// Shorthand for run_frontend with default options (use_unvoiced_vowel = false)
// void make_label(rust::Str text, rust::Vec<rust::String> &output);

#!/usr/bin/env bash
# Apple Command Line Tools may omit the C++ standard library from clang's search path.
spiel_cxx_include="/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk/usr/include/c++/v1"
if [[ -d "$spiel_cxx_include" && -z "${CPLUS_INCLUDE_PATH:-}" ]]; then
  export CPLUS_INCLUDE_PATH="$spiel_cxx_include"
fi

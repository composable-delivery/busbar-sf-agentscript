{
  "targets": [
    {
      "target_name": "tree_sitter_agentscript_binding",
      "include_dirs": [
        "<!(node -e \"require('nan')\")",
        "src"
      ],
      "sources": [
        "bindings/node/binding.cc",
        "src/parser.c",
        "src/scanner.c"
      ],
      "cflags_c": [
        "-std=c11",
        "-Wno-unused-value"
      ],
      "cflags_cc": [
        "-std=c++14"
      ],
      "xcode_settings": {
        "CLANG_CXX_LANGUAGE_STANDARD": "c++14",
        "OTHER_CFLAGS": [
          "-Wno-unused-value"
        ]
      }
    }
  ]
}

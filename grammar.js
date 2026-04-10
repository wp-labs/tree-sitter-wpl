/// <reference types="tree-sitter-cli/dsl" />

module.exports = grammar({
  name: "wpl",

  extras: ($) => [/\s/],

  word: ($) => $.identifier,

  externals: ($) => [$.quote_format],

  conflicts: ($) => [[$.group_meta, $.function_name]],

  rules: {
    source_file: ($) => repeat($._declaration),

    _declaration: ($) => choice($.package_decl, $.rule_decl),

    package_decl: ($) =>
      seq(
        optional($.annotation),
        "package",
        field("name", $.path_name),
        "{",
        repeat1($.rule_decl),
        "}",
      ),

    rule_decl: ($) =>
      seq(
        optional($.annotation),
        "rule",
        field("name", $.path_name),
        "{",
        $.expression,
        "}",
      ),

    path_name: ($) => $.key,

    expression: ($) => seq(optional($.preproc), $.group, repeat(seq(",", $.group))),

    preproc: ($) => seq("|", repeat1(seq($.preproc_step, "|"))),

    preproc_step: ($) => choice($.plg_pipe_step, $.key, $.identifier),

    plg_pipe_step: ($) =>
      choice(
        seq("plg_pipe", "/", field("name", $.key)),
        seq("plg_pipe", "(", field("name", $.key), ")"),
      ),

    group: ($) =>
      seq(
        optional(field("meta", $.group_meta)),
        "(",
        optional($._field_list),
        ")",
        optional($.group_length),
        optional(field("separator", $.shortcut_sep)),
      ),

    group_meta: ($) => choice("alt", "opt", "some_of", "seq", "not"),

    group_length: ($) => seq("[", $.number, "]"),

    _field_list: ($) => seq($.field, repeat(seq(",", $.field)), optional(",")),

    field: ($) =>
      seq(
        optional($.repeat_prefix),
        field("meta", $.meta_token),
        optional($.symbol_content),
        optional($.subfields),
        optional(seq(":", field("binding", $.var_name))),
        optional($.field_length),
        optional($.format),
        optional(field("separator", $.field_separator)),
        repeat($.pipe),
      ),

    repeat_prefix: ($) => seq(optional($.number), "*"),

    subfields: ($) =>
      seq("(", optional($._subfield_list), ")"),

    _subfield_list: ($) =>
      seq($.subfield, repeat(seq(",", $.subfield)), optional(",")),

    subfield: ($) =>
      choice(
        seq(
          field("meta", $.subfield_meta),
          optional($.symbol_content),
          optional(seq("@", field("ref", $.ref_path_or_quoted))),
          optional(seq(":", field("binding", $.var_name))),
          optional($.format),
          optional(field("separator", $.field_separator)),
          repeat($.pipe),
        ),
        seq(
          "@",
          field("ref", $.ref_path_or_quoted),
          optional(seq(":", field("binding", $.var_name))),
          optional($.format),
          optional(field("separator", $.field_separator)),
          repeat($.pipe),
        ),
        seq(
          ":",
          field("binding", $.var_name),
          optional($.format),
          optional(field("separator", $.field_separator)),
          repeat($.pipe),
        ),
        seq(
          $.format,
          optional(field("separator", $.field_separator)),
          repeat($.pipe),
        ),
        seq($.field_separator, repeat($.pipe)),
      ),

    subfield_meta: ($) => choice($.opt_type, $.meta_token),

    opt_type: ($) => seq("opt", "(", $.key, ")"),

    meta_token: ($) => choice($.array_type, $.meta_name),

    meta_name: ($) => /[A-Za-z0-9_\/]+/,

    array_type: ($) =>
      seq("array", optional(seq("/", field("element", $.identifier)))),

    symbol_content: ($) =>
      seq("(", field("content", $.symbol_text), ")"),

    symbol_text: ($) => token(repeat1(choice(/[^),@\\]/, /\\./))),

    field_length: ($) => seq("[", $.number, "]"),

    format: ($) => choice($.scope_format, $.quote_format),

    scope_format: ($) => seq("<", token(/[^>]*/), ">"),

    field_separator: ($) => choice($.shortcut_sep, $.pattern_sep),

    shortcut_sep: ($) => prec(1, repeat1($.escape_char)),

    escape_char: ($) => /\\./,

    pattern_sep: ($) => seq("{", token(/[^}]*/), "}"),

    pipe: ($) => seq("|", choice(prec(2, $.fun_call), $.group)),

    fun_call: ($) =>
      prec(
        1,
        seq(
          field("function", $.function_name),
          "(",
          optional($.fun_args),
          ")",
        ),
      ),

    function_name: ($) => choice("not", $.identifier),

    fun_args: ($) => seq($._fun_arg, repeat(seq(",", $._fun_arg))),

    _fun_arg: ($) =>
      choice(
        $.fun_call,
        $.quoted_string,
        $.raw_string,
        $.array_literal,
        $.number,
        $.identifier,
        $.key,
      ),

    array_literal: ($) =>
      seq("[", $._fun_arg, repeat(seq(",", $._fun_arg)), "]"),

    annotation: ($) =>
      seq(
        $.annotation_start,
        $.ann_item,
        repeat(seq(",", $.ann_item)),
        "]",
      ),

    annotation_start: ($) => "#[",

    ann_item: ($) => choice($.tag_anno, $.copy_raw_anno),

    tag_anno: ($) =>
      seq("tag", "(", $.tag_kv, repeat(seq(",", $.tag_kv)), ")"),

    tag_kv: ($) =>
      seq(field("key", $.key), ":", field("value", $._string_literal)),

    copy_raw_anno: ($) => seq("copy_raw", "(", $.tag_kv, ")"),

    _string_literal: ($) =>
      choice($.quoted_string, $.raw_string),

    quoted_string: ($) => choice($.double_quoted_string, $.single_quoted_string),

    double_quoted_string: ($) =>
      token(seq('"', repeat(choice(/[^"\\]/, /\\./)), '"')),

    single_quoted_string: ($) =>
      token(seq("'", repeat(choice(/[^'\\]/, /\\./)), "'")),

    raw_string: ($) =>
      token(
        choice(
          seq('r#"', repeat(/[^"]/), '"#'),
          seq('r"', repeat(/[^"]/), '"'),
        ),
      ),

    single_quoted_raw: ($) =>
      token(seq("'", repeat(choice(/[^'\\]/, /\\./)), "'")),

    number: ($) => /[0-9]+/,

    identifier: ($) => /[A-Za-z_][A-Za-z0-9_.]*/,

    key: ($) => /[A-Za-z0-9_.\/-]+/,

    var_name: ($) => /[A-Za-z0-9_.-]+/,

    ref_path_or_quoted: ($) => choice($.ref_path, $.single_quoted_raw),

    ref_path: ($) => /[A-Za-z0-9_.\/*\[\]\-]+/,
  },
});

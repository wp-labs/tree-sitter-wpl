/// <reference types="tree-sitter-cli/dsl" />

module.exports = grammar({
  name: "wpl",

  extras: ($) => [/\s/],

  word: ($) => $.identifier,

  externals: ($) => [$.quote_format],

  conflicts: ($) => [[$.field, $.subfield]],

  rules: {
    source_file: ($) => repeat($._declaration),

    _declaration: ($) => choice($.package_decl, $.rule_decl),

    // ── Package declaration ──────────────────────────────────────
    package_decl: ($) =>
      seq(
        optional($.annotation),
        "package",
        field("name", $.path_name),
        "{",
        repeat($.rule_decl),
        "}",
      ),

    // ── Rule declaration ─────────────────────────────────────────
    rule_decl: ($) =>
      seq(
        optional($.annotation),
        "rule",
        field("name", $.path_name),
        "{",
        $._statement,
        "}",
      ),

    path_name: ($) => /[A-Za-z0-9_.\/-]+/,

    // ── Statement ────────────────────────────────────────────────
    _statement: ($) => choice($.plg_pipe_block, $.expression),

    plg_pipe_block: ($) =>
      seq(
        optional("@"),
        "plg_pipe",
        "(",
        "id",
        ":",
        field("key", $.key),
        ")",
        "{",
        $.expression,
        "}",
      ),

    // ── Expression: [preproc] group {, group} ────────────────────
    expression: ($) =>
      seq(optional($.preproc), $.group, repeat(seq(",", $.group))),

    // ── Preprocessor pipeline: |step|step| ───────────────────────
    preproc: ($) => seq("|", repeat1(seq($.preproc_step, "|"))),

    preproc_step: ($) =>
      choice(seq("plg_pipe", "/", $.key), $.preproc_path),

    preproc_path: ($) =>
      seq(field("ns", $.identifier), "/", field("name", $.identifier)),

    // ── Group: [meta] ( field_list ) [len] [sep] ─────────────────
    group: ($) =>
      seq(
        optional(field("meta", $.group_meta)),
        "(",
        optional($._field_list),
        ")",
        optional($.group_length),
        optional($.separator),
      ),

    group_meta: ($) => choice("alt", "opt", "some_of", "seq", "not"),

    group_length: ($) => seq("[", $.number, "]"),

    _field_list: ($) =>
      seq($._field_item, repeat(seq(",", $._field_item)), optional(",")),

    _field_item: ($) => choice($.subfield, $.field),

    // ── Field ────────────────────────────────────────────────────
    // Order per EBNF & parser (wpl_field.rs):
    //   [repeat] type [(args)] [:name] [[len]] [fmt] [sep] {pipe}
    field: ($) =>
      seq(
        optional($.repeat_prefix),
        $.data_type,
        optional($.type_arguments),
        optional(seq(":", field("binding", $.var_name))),
        optional($.field_length),
        optional($.format),
        optional($.separator),
        repeat($.pipe),
      ),

    // ── Subfield (with @ref) ─────────────────────────────────────
    subfield: ($) =>
      seq(
        optional(choice($.opt_type, $.data_type)),
        optional($.type_arguments),
        "@",
        field("ref", $.ref_path),
        optional(seq(":", field("binding", $.var_name))),
        optional($.format),
        optional($.separator),
        repeat($.pipe),
      ),

    opt_type: ($) => seq("opt", "(", $.data_type, ")"),

    repeat_prefix: ($) => seq(optional($.number), "*"),

    // ── Data type ────────────────────────────────────────────────
    data_type: ($) => choice($.array_type, $.ns_type, $.type_name),

    type_name: ($) => $.identifier,

    ns_type: ($) =>
      prec(
        1,
        seq(
          field("namespace", $.identifier),
          "/",
          field("name", $.identifier),
        ),
      ),

    array_type: ($) =>
      seq("array", optional(seq("/", field("element", $.identifier)))),

    // ── Type arguments (subfields or symbol content) ─────────────
    type_arguments: ($) => seq("(", optional($._field_list), ")"),

    field_length: ($) => seq("[", $.number, "]"),

    // ── Format ───────────────────────────────────────────────────
    format: ($) => choice($.scope_format, $.quote_format),
    scope_format: ($) => seq("<", /[^>]*/, ">"),
    // quote_format handled by external scanner

    // ── Separator ────────────────────────────────────────────────
    separator: ($) => choice($.shortcut_sep, $.pattern_sep),
    shortcut_sep: ($) => prec(1, repeat1($.escape_char)),
    escape_char: ($) => /\\./,
    pattern_sep: ($) => seq("{", /[^}]*/, "}"),

    // ── Pipe ─────────────────────────────────────────────────────
    pipe: ($) => seq("|", choice($.fun_call, $.group)),

    fun_call: ($) =>
      prec(
        1,
        seq(
          field("function", choice($.identifier, "not")),
          "(",
          optional($.fun_args),
          ")",
        ),
      ),

    fun_args: ($) => seq($._fun_arg, repeat(seq(",", $._fun_arg))),

    _fun_arg: ($) =>
      choice(
        $.fun_call,
        $.quoted_string,
        $.array_literal,
        $.number,
        $.key,
      ),

    array_literal: ($) =>
      seq("[", $._fun_arg, repeat(seq(",", $._fun_arg)), "]"),

    // ── Annotation ───────────────────────────────────────────────
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
      seq(
        field("key", $.identifier),
        ":",
        field("value", $._string_literal),
      ),

    copy_raw_anno: ($) =>
      seq(
        "copy_raw",
        "(",
        "name",
        ":",
        field("value", $._string_literal),
        ")",
      ),

    _string_literal: ($) => choice($.quoted_string, $.raw_string),

    // ── Literals ─────────────────────────────────────────────────
    quoted_string: ($) => token(seq('"', repeat(choice(/[^"\\]/, /\\./)), '"')),

    raw_string: ($) => token(seq("r#", '"', /[^"]*/, '"', "#")),

    number: ($) => /[0-9]+/,

    // ── Identifiers ──────────────────────────────────────────────
    identifier: ($) => /[a-zA-Z_][a-zA-Z0-9_]*/,

    key: ($) => /[A-Za-z0-9_.\/-]+/,

    var_name: ($) => /[A-Za-z_][A-Za-z0-9_.\-]*/,

    ref_path: ($) => /[A-Za-z0-9_.\/*\[\]\-]+/,
  },
});

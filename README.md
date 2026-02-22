# tree-sitter-wpl

[Tree-sitter](https://tree-sitter.github.io/tree-sitter/) grammar for **WPL (WarpLabs Parsing Language)** — a declarative language for defining structured data parsing rules with typed fields, grouping patterns, and transformation pipelines.

## Overview

WPL defines how to parse structured data (text, binary, or protocol formats) into typed fields. It supports hierarchical field grouping with modifiers (alternation, optional, repetition), preprocessing pipelines, field-level transformations via pipes, subfield references, format specifiers, and metadata annotations.

## Language Structure

A WPL file contains `package` and `rule` declarations:

```wpl
package my_package {
    rule record {
        (
            chars:name,
            digit:age,
            ip:address
        )
    }
}
```

## Language Features

### Package Declaration

Group related rules under a named package:

```wpl
#[tag(version: "1.0")]
package network/http {
    rule request { ... }
    rule response { ... }
}
```

### Rule Declaration

Define a parsing rule with a name and a body expression:

```wpl
rule syslog/message {
    (
        digit:priority,
        chars:hostname,
        chars:message
    )
}
```

Rule and package names support path-style identifiers: `network/http`, `log/syslog`.

### Fields

Fields are the core parsing unit, with the general form:

```
[repeat] type [(args)] [:binding] [[length]] [format] [separator] {|pipe}
```

```wpl
rule example {
    (
        chars:username,
        digit:port,
        ip:src_addr,
        float:ratio,
        hex:mac_addr,
        bool:is_active
    )
}
```

#### Type System

**Base types:**

| Type | Description |
|------|-------------|
| `chars` | String / text |
| `digit` | Integer number |
| `float` | Floating-point number |
| `bool` | Boolean value |
| `ip` | IP address |
| `hex` | Hexadecimal value |
| `time` | Timestamp |

**Array types:**

```wpl
array/chars:tags
array/digit:ports
array:items
```

**Namespaced types:**

```wpl
network/ipv4:header
http/request:req
```

#### Variable Binding

Bind a parsed value to a named variable with `:name`:

```wpl
chars:username
digit:port
ip:src_addr
```

#### Field Length

Specify fixed-length fields with `[n]`:

```wpl
chars[16]:hostname
hex[6]:mac_addr
```

#### Repeat Prefix

Parse repeated fields with `*` or `n*`:

```wpl
*chars:lines           // zero or more
3*digit:coordinates    // exactly 3
```

#### Format Specifiers

Control how a field is parsed:

```wpl
// Angle-bracket format
chars<yyyy-MM-dd>:date
digit<hex>:value

// Quote format (standalone " marker)
chars":name
```

#### Separators

Specify field delimiters:

```wpl
// Escape character shortcut
chars\,:field1          // comma separator
chars\t:field2          // tab separator
chars\n:field3          // newline separator

// Pattern separator
chars{||}:field4        // "||" as separator
digit{, }:numbers      // ", " as separator
```

### Subfields

Reference fields from another rule with `@`:

```wpl
rule header {
    (chars:version, digit:length)
}

rule packet {
    (
        @header,                        // bare reference
        chars@payload/data:content,     // typed reference with binding
        opt(digit)@header/length:len    // optional typed reference
    )
}
```

### Groups

Fields are organized in parenthesized groups with optional modifiers:

```wpl
// Alternation: match one of the alternatives
alt(
    digit:int_value,
    float:float_value,
    chars:str_value
)

// Optional: may or may not be present
opt(
    chars:optional_field
)

// Sequence: ordered sequence
seq(
    chars:first,
    chars:second
)

// Some-of: match one or more
some_of(
    digit:a,
    digit:b,
    digit:c
)

// Not: negative match
not(
    chars:excluded
)
```

Groups can have a length and separator:

```wpl
(digit:a, digit:b, digit:c)[10]    // group with length
(chars:line)\n                      // group with separator
```

### Preprocessing Pipeline

Apply preprocessing steps before parsing:

```wpl
rule preprocessed {
    |base64/decode|gzip/decompress|
    (
        chars:data
    )
}

rule with_plg_pipe {
    |plg_pipe/my_plugin|
    (chars:result)
}
```

### Pipes (Field Transformations)

Chain transformations on a field with `|function(args)`:

```wpl
rule transformed {
    (
        chars:raw |lowercase() |trim(),
        digit:code |map("200", "ok") |default(0),
        chars:data |not(empty()) |split(",")
    )
}
```

Pipe functions accept arguments: strings, numbers, arrays, nested function calls, or keys.

```wpl
chars:value |replace("old", "new") |validate([1, 2, 3])
```

### plg_pipe Block

Invoke a plugin pipeline with an identifier key:

```wpl
rule with_plugin {
    plg_pipe(id: my_plugin_key) {
        (chars:field1, digit:field2)
    }
}

// With @ prefix
rule with_plugin2 {
    @plg_pipe(id: another_key) {
        (chars:output)
    }
}
```

### Type Arguments

Types can accept inline arguments (subfields or nested field lists):

```wpl
rule nested {
    (
        record(chars:name, digit:age):person,
        array(digit):numbers
    )
}
```

### Annotations

Attach metadata to packages or rules:

```wpl
#[tag(author: "team", version: "2.0")]
package my_package {
    #[copy_raw(name: "raw_output")]
    rule my_rule {
        (chars:data)
    }
}
```

Annotation types:
- **tag** — key-value metadata: `tag(key: "value", ...)`
- **copy_raw** — raw data copy: `copy_raw(name: "field_name")`

### Comments

Line comments with `//`:

```wpl
// This is a comment
rule example {
    (
        chars:field  // inline comment
    )
}
```

## Full Example

```wpl
#[tag(name: "syslog", version: "1.0")]
package log/syslog {

    rule header {
        (
            digit:priority,
            digit:version,
            chars<yyyy-MM-ddTHH:mm:ss>:timestamp
        )
    }

    #[tag(protocol: "rfc5424")]
    rule message {
        |base64/decode|
        (
            @header,
            chars:hostname\s,
            chars:app_name\s,
            chars:proc_id\s,
            chars:msg_id\s,
            opt(
                chars:structured_data
            ),
            chars:message |trim() |lowercase()
        )
    }

    rule batch {
        (
            digit:count,
            *@message:entries
        )\n
    }
}
```

## Usage

### Rust

Add to your `Cargo.toml`:

```toml
[dependencies]
tree-sitter = ">=0.22.6"
tree-sitter-wpl = "0.0.1"
```

```rust
let language = tree_sitter_wpl::language();
let mut parser = tree_sitter::Parser::new();
parser.set_language(&language).unwrap();

let source = r#"rule example {
    (chars:name, digit:age)
}"#;
let tree = parser.parse(source, None).unwrap();
println!("{}", tree.root_node().to_sexp());
```

### Node.js

```javascript
const Parser = require("tree-sitter");
const WPL = require("tree-sitter-wpl");

const parser = new Parser();
parser.setLanguage(WPL);

const tree = parser.parse(`rule example {
    (chars:name, digit:age)
}`);
console.log(tree.rootNode.toString());
```

### Python

```python
import tree_sitter_wpl

language = tree_sitter_wpl.language()
```

### Go

```go
import tree_sitter_wpl "github.com/tree-sitter/tree-sitter-wpl"

language := tree_sitter.NewLanguage(tree_sitter_wpl.Language())
```

### Swift

Add via Swift Package Manager using `Package.swift`.

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) (for `tree-sitter-cli`)
- [Rust toolchain](https://rustup.rs/) (for building the Rust binding)

### Building

```bash
# Install dependencies
npm install

# Generate the parser from grammar.js
npx tree-sitter generate

# Run tests
npx tree-sitter test

# Build the Rust binding
cargo build

# Run Rust tests
cargo test

# Build C library
make
```

### Project Structure

```
tree-sitter-wpl/
├── grammar.js              # Grammar definition
├── queries/
│   └── highlights.scm      # Syntax highlighting queries
├── bindings/
│   ├── rust/                # Rust binding
│   ├── node/                # Node.js binding
│   ├── python/              # Python binding
│   ├── go/                  # Go binding
│   ├── c/                   # C header and pkg-config
│   └── swift/               # Swift binding
├── src/
│   ├── parser.c             # Generated parser
│   ├── scanner.c            # External scanner (quote_format)
│   ├── grammar.json         # Generated grammar schema
│   └── node-types.json      # AST node type definitions
├── Cargo.toml               # Rust package manifest
├── package.json             # Node.js package manifest
├── pyproject.toml           # Python package manifest
├── Package.swift            # Swift package manifest
└── Makefile                 # C library build rules
```

### External Scanner

The grammar includes an external scanner (`src/scanner.c`) that handles the `quote_format` token — a standalone `"` character used as a format marker. It is distinguished from `quoted_string` by checking the character that follows: a `quote_format` is always followed by a delimiter (`,`, `)`, `|`, `\`, `{`, whitespace, or EOF).

## License

Apache License 2.0 — see [LICENSE](LICENSE) for details.

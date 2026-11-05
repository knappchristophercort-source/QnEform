# QnEform Frontend Syntax Subset v0.1

**Status:** Implemented syntax subset; experimental and intentionally incomplete  
**Scope:** Lexer, parser, AST, and syntax-check command  
**Does not define:** Quantum gate semantics, type checking, execution, or physical behavior

## 1. Supported grammar

The current parser accepts the following EBNF subset:

\`\`\`ebnf
program          = { statement } ;
statement        = qubit_decl
                  | measurement
                  | let_binding
                  | quantum_block
                  | classical_block ;

qubit_decl       = "qbit" , identifier , ";" ;
measurement      = "measure" , identifier , ";" ;
let_binding      = "let" , identifier , ":=" , expression , ";" ;

quantum_block    = "quantum" , "{" , { statement } , "}" ;
classical_block  = "classical" , "{" , { statement } , "}" ;

expression       = identifier
                  | number
                  | string
                  | "true"
                  | "false" ;
identifier       = ( letter | "_" ) , { letter | digit | "_" } ;
number           = digit , { digit | "_" } , [ "." , digit , { digit | "_" } ] ;
\`\`\`

The grammar is a parser implementation contract for this milestone, not a final language grammar. Nested blocks are accepted by the current recursive parser. Semantic restrictions between block domains are not yet enforced.

## 2. Lexical rules

- Identifiers are ASCII in v0.1.
- Whitespace is ignored between tokens.
- Line comments begin with \`//\`.
- Block comments use \`/* ... */\` and do not nest.
- Strings use double quotes.
- Supported string escapes: \`\\n\`, \`\\r\`, \`\\t\`, \`\\"\`, and \`\\\\\`.
- Numeric literals are preserved as source text. Numeric range and floating-point semantics are not assigned yet.
- Tokens carry source spans with UTF-8 byte offsets and one-based line/column coordinates.
- Unknown characters and unterminated strings/comments produce lexical errors.

## 3. AST contract

The AST currently represents:

- \`Program\`: ordered top-level statements.
- \`QubitDeclaration\`: declared qubit identifier.
- \`Measurement\`: measurement target identifier.
- \`LetBinding\`: name and literal/identifier expression.
- \`QuantumBlock\` and \`ClassicalBlock\`: ordered nested statements.
- Each statement retains its source span.

The AST is syntax-only. In particular, parsing \`qbit q0;\` does not allocate a quantum resource, and parsing \`measure q0;\` does not perform a measurement.

## 4. Command-line behavior

\`\`\`sh
qneform check path/to/program.qn
\`\`\`

A successful result means the file was lexed and parsed under this grammar. It is not a type-check, proof, simulation, circuit compilation, or hardware execution result.

Unsupported syntax must return a non-zero exit status with a diagnostic. For example, gate syntax such as \`H q0;\` is rejected until its grammar and semantic contract are deliberately introduced.

## 5. Required next-stage work

1. Add semantic validation for duplicate declarations, unresolved identifiers, and invalid statement placement.
2. Define explicit type and resource rules before adding gate syntax.
3. Lower the AST into a versioned, backend-independent Semantic IR.
4. Preserve source spans and source-to-IR provenance.
5. Add conformance tests that distinguish syntax acceptance from semantic validity.
6. Only then add a minimal supported gate vocabulary and reference simulator.

## 6. Non-goals for this version

- No implicit qubit allocation or release.
- No implicit measurement, reset, or state inspection.
- No gate statements, gate parameters, or circuit execution.
- No assumptions about physical hardware, noise, decoherence, or error correction.
- No claim that the current parser implements the full QnEform language.

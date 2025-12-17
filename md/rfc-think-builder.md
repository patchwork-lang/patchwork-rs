# RFC: ThinkBuilder API

## Summary

Patchwork-rs is a Rust library for blending deterministic programming with LLM-powered reasoning. It provides a builder-style API for constructing prompts that can invoke Rust closures as MCP tools, enabling seamless interleaving of structured code and natural language processing.

## Motivation

Modern applications increasingly benefit from LLM capabilities, but integrating them into typed, deterministic codebases is awkward. You either:

1. **String templates** - Lose type safety, no compile-time checking of interpolations
2. **Separate prompt files** - Context switch between code and prompts, hard to pass runtime values
3. **Framework lock-in** - Heavy abstractions that obscure what's actually happening

Patchwork-rs takes a different approach: LLM interactions are first-class Rust expressions. The `think` builder composes prompts programmatically while allowing the LLM to call back into typed Rust closures via MCP tools.

This is inspired by the [Patchwork programming language](https://github.com/patchwork-lang/patchwork), which pioneered the idea of `think` blocks that blend imperative code with LLM reasoning.

## Guide-level design

### Basic usage

```rust
use patchwork::Patchwork;
use sacp::Component;

#[tokio::main]
async fn main() -> Result<(), patchwork::Error> {
    let component: Component = /* ... */;
    let patchwork = Patchwork::new(component);
    
    let name = "Alice";
    let result: String = patchwork.think()
        .text("Say hello to")
        .display(&name)
        .text("in a friendly way.")
        .await?;
    
    println!("{}", result);  // "Hello Alice! Great to meet you!"
    Ok(())
}
```

### Composing prompts

The `ThinkBuilder` provides methods for building up prompts piece by piece:

- `.text("...")` - Add literal text
- `.display(&value)` - Interpolate a value using its `Display` impl
- `.debug(&value)` - Interpolate a value using its `Debug` impl (useful for paths, complex types)

```rust
let file_path = Path::new("data/input.txt");
let contents = std::fs::read_to_string(&file_path)?;

let summary: String = patchwork.think()
    .text("Summarize the following file")
    .debug(&file_path)
    .text(":\n\n")
    .display(&contents)
    .await?;
```

### Smart spacing

By default, the builder automatically inserts spaces between segments to reduce visual noise. A space is inserted before a segment when:

- The previous segment was text and did not end in whitespace

**Unless** the current segment is text and begins with `.`, `,`, `:`, or `;`.

This means you can write:

```rust
.text("Hello,")
.display(&name)
.text(". How are you?")
```

And get `"Hello, Alice. How are you?"` — space auto-inserted before the name, but not before the period.

If you need precise control, disable smart spacing:

```rust
patchwork.think()
    .explicit_spacing()  // disable auto-spacing for this builder
    .text("No")
    .text("Spaces")
    .text("Here")
    // produces "NoSpacesHere"
```

### Tools: calling Rust from the LLM

The real power comes from `.tool()`, which registers a Rust closure as an MCP tool the LLM can invoke:

```rust
let result: String = patchwork.think()
    .text("Process the transcript and invoke")
    .tool("rephrase", async |cx, input: RephraseInput| -> String {
        make_it_nicer(&input.phrase)
    })
    .text("on each mean-spirited phrase.")
    .await?;
```

When you call `.tool(name, closure)`:
1. The closure is registered as an MCP tool with the given name
2. The text `<mcp_tool>name</mcp_tool>` is embedded in the prompt

The closure receives a `&PatchworkCx` as its first argument, followed by the tool input. The context is currently empty but provides room for future extensions (access to the parent session, logging, etc.).

The LLM sees the available tools and can call them. The input and output types must implement `JsonSchema` (via `schemars`) so the LLM knows the expected schema.

### Defining tools without embedding

Sometimes you want to make a tool available without embedding a reference in the prompt at that point:

```rust
let result: String = patchwork.think()
    .text("Analyze the sentiment of each paragraph.")
    .text("Use the classify tool for ambiguous cases.")
    .define_tool("classify", async |cx, text: String| classify_sentiment(&text))
    .tool("summarize", async |cx, paras: Vec<String>| summarize_all(&paras))
    .await?;
```

Here `classify` is available but not explicitly referenced with `<mcp_tool>` tags—the prompt mentions it in natural language. The `summarize` tool is both defined and referenced.

### Structured output

The return type of `think()` can be any type that implements `JsonSchema + DeserializeOwned`:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
struct Analysis {
    sentiment: String,
    confidence: f64,
    key_phrases: Vec<String>,
}

let analysis: Analysis = patchwork.think()
    .text("Analyze the sentiment of: ")
    .display(&text)
    .await?;
```

The LLM is instructed to return its result by calling a `return_result` MCP tool with the appropriate JSON schema.

## Frequently asked questions

### Why a builder instead of a macro?

We plan to add a `think!` macro eventually, but starting with a builder has advantages:

1. **Easier to iterate** - Runtime API is simpler to evolve than proc-macro
2. **Better error messages** - Proc-macro errors are notoriously hard to debug
3. **Transparent** - You can see exactly what the builder does

The macro will likely expand to builder calls (or something equivalent).

### Why MCP tools?

We use MCP tools both for invoking user-defined closures and for returning results. The key advantage is that MCP tools provide an explicit, deterministic output structure—the LLM must call the `return_result` tool with JSON matching the expected schema. This avoids the need to parse free-form text output and ensures type safety end-to-end.

### How does the LLM know to return a result?

The `Patchwork` runtime automatically:

1. Adds a `return_result` MCP tool with a schema matching your expected output type
2. Includes instructions telling the LLM to call this tool when done
3. Waits for the tool call and deserializes the result

### What about nested think blocks?

A tool closure can contain another `think()` call, enabling multi-agent patterns:

```rust
.tool("deep_analysis", async |cx, topic: String| {
    patchwork.think()
        .text("Provide deep analysis of:")
        .display(&topic)
        .await
})
```

Nested `think()` calls just work—they create independent sessions. In the future, `PatchworkCx` may provide access to information about what the LLM has done so far in the parent session, allowing tools to embed a summary of the current output in the subthread.

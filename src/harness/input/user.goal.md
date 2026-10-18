# Build a Node.js CLI Hello World Tool

Create a Node.js CLI tool in the repository root.

## Goal

The tool is executed/called with a single CLI argument and writes that value to a `.md` file.

The CLI entrypoint must be:

```sh
hello-globe.cjs
```

If the tool is called with

```sh
node hello-globe.cjs 'hey there'
```

Then a file is generated named

```sh
hello-globe.md
```

With this content

```md
hey there
```

## Required behavior

From the repository root, the user must be able to run:

```sh
node hello-globe.cjs "hi"
```

and the command must create or overwrite `hello-globe.md` so that the file contains the supplied value.

For example:

```sh
node hello-globe.cjs "hi"
```

must result in:

```sh
hello-globe.md
```

containing:

```text
hi
```

The implementation should use Node.js and should not require a third-party runtime or framework.

## Tests

Add automated tests covering both:

1. **JavaScript logic**
   - Test the logic responsible for taking an input value and producing/writing the expected Markdown output.
   - The tests should be able to exercise this logic without needing to invoke the CLI as a separate process.

2. **CLI behavior**
   - Test the actual command-line entrypoint.
   - The test should invoke the equivalent of:

     ```sh
     node hello-globe.cjs "hi"
     ```

   - Verify that the expected `hello-globe.md` file is created and contains the expected value.

Tests should avoid modifying unrelated repository files and should clean up any temporary files they create.

## Acceptance criteria

The implementation is complete when all of the following are true:

- `hello-globe.cjs` exists at the repository root.
- `hello-globe.cjs` is executable through Node.js with no additional runtime setup.
- Running `node hello-globe.cjs "hi"` succeeds.
- Running the command creates or overwrites `hello-globe.md` at the repository root.
- `hello-globe.md` contains the supplied argument.
- The JavaScript logic has automated test coverage.
- The CLI behavior has automated test coverage.
- All automated tests pass.
- No unrelated existing files or behavior are modified.

Do not add functionality beyond this requirement unless it is necessary for testing or basic robustness.

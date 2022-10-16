# openfga-rs

openfga-rs is a OpenFGA model parser and compiler written in rust

## Features

- Compile OpenFGA authorization model into its JSON representation

## Run Locally

Clone the project

```bash
  git clone https://github.com/iammathew/openfga-rs
```

Go to the project directory

```bash
  cd openfga-rs
```

Build the executable

```bash
  cargo build
```

Run the compiler

```bash
  ./target/debug/openfgac [filepath] > [jsonfile]
```

## Next steps

- Publish cargo packages
- Add wasm build
- Add correctness check of model
- Build a LSP

## Authors

- [@iammathew](https://www.github.com/iammathew)

## Acknowledgments

- [OpenFGA](https://openfga.dev/)

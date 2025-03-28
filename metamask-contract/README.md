🚧  This repo is WIP - None of the README are up-to-date 🚧

# Metamask Identity risc0 example

On Hylé, any smart contract can serve as proof of identity. This flexibility allows you to register your preferred identity source as a smart contract for account identification. Hylé also ships [a native `hydentity` contract](https://github.com/Hyle-org/hyle/tree/main/contracts/hydentity) for simplicity.

This is a Risc0 example called simple_identity.

## Prerequisites

- [Install Rust](https://www.rust-lang.org/tools/install) (you'll need `rustup` and Cargo).
- For our example, [install RISC Zero](https://dev.risczero.com/api/zkvm/install).
- [Start a single-node devnet](https://docs.hyle.eu/developers/quickstart/devnet/). We recommend using [dev-mode](https://dev.risczero.com/api/generating-proofs/dev-mode) with `-e RISC0_DEV_MODE=1` for faster iterations during development.

## Quickstart

### Build and register the identity contract

To build all methods and register the smart contract on the local node [from the source](https://github.com/Hyle-org/examples/blob/simple_erc20/simple-token/host/src/main.rs), run:

```bash
cargo run -- register-contract
```

The expected output is `📝 Registering new contract mmid`.

### Register an account / Sign up

To register an account with a public key (`0x437aa724e898f0ba345852bbbc2e416d9346e1c9`) and signature, execute:

```sh
RISC0_DEV_MODE=1 cargo run -- register-identity 0x437aa724e898f0ba345852bbbc2e416d9346e1c9.mmid 0x3279f925d976ddfc012a95157f87054717610b4fa08028913ab3139f552e76342d609c11acbca164fb9c64fb5553db3fe34c826c0084fd6bc934dcd154993b0a1b
```

Note that signature needs to be sign with `hyle registration`

The node's logs will display:

```bash
INFO hyle::data_availability::node_state::verifiers: ✅ Risc0 proof verified.
INFO hyle::data_availability::node_state::verifiers: 🔎 Program outputs: Successfully registered identity for account: 0x437aa724e898f0ba345852bbbc2e416d9346e1c9.mmid
```

### Verify identity / Login

To verify `0x437aa724e898f0ba345852bbbc2e416d9346e1c9`'s identity:

```bash
cargo run -- verify-identity 0x437aa724e898f0ba345852bbbc2e416d9346e1c9.mmid 0
```

This command will:

1. Send a blob transaction to verify `0x437aa724e898f0ba345852bbbc2e416d9346e1c9`'s identity.
1. Generate a ZK proof of that identity. It will only be valid once, thus the inclusion of a nonce.
1. Send the proof to the devnet.

Upon reception of the proof, the node will:

1. Verify the proof.
1. Settle the blob transaction.
1. Update the contract's state.

The node's logs will display:

```bash
INFO hyle::data_availability::node_state::verifiers: ✅ Risc0 proof verified.
INFO hyle::data_availability::node_state::verifiers: 🔎 Program outputs: Identity verified for account: alice.simple_identity
```

### Verify k256 signature

To verify you're signature locally (must be signed with `hyle registration`) without Hylé node running you can call :

```sh
cargo run -- validate-signature 0x437aa724e898f0ba345852bbbc2e416d9346e1c9 0x3279f925d976ddfc012a95157f87054717610b4fa08028913ab3139f552e76342d609c11acbca164fb9c64fb5553db3fe34c826c0084fd6bc934dcd154993b0a1b
```

### Run server to generate proof uppon metamask request

To host server that will generate proof for identity registration from metamask run :

```sh
RISC0_DEV_MODE=1 cargo run -- run-server
```

Server will start and listen for proof request generation.
Once proof generated it is pushed to Hylé node.

### Executing the Project Locally in Development Mode

During development, faster iteration upon code changes can be achieved by leveraging [dev-mode], we strongly suggest activating it during your early development phase. Furthermore, you might want to get insights into the execution statistics of your project, and this can be achieved by specifying the environment variable `RUST_LOG="[executor]=info"` before running your project.

Put together, the command to run your project in development mode while getting execution statistics is:

```bash
RUST_LOG="[executor]=info" RISC0_DEV_MODE=1 cargo run
```

<!--### Running Proofs Remotely on Bonsai-->
<!---->
<!--_Note: The Bonsai proving service is still in early Alpha; an API key is-->
<!--required for access. [Click here to request access][bonsai access]._-->
<!---->
<!--If you have access to the URL and API key to Bonsai you can run your proofs-->
<!--remotely. To prove in Bonsai mode, invoke `cargo run` with two additional-->
<!--environment variables:-->
<!---->
<!--```bash-->
<!--BONSAI_API_KEY="YOUR_API_KEY" BONSAI_API_URL="BONSAI_URL" cargo run-->
<!--```-->

## How to create a project based on this example

- The [RISC Zero Developer Docs][dev-docs] is a great place to get started.
- Example projects are available in the [examples folder][examples] of
  [`risc0`][risc0-repo] repository.
- Reference documentation is available at [https://docs.rs][docs.rs], including
  [`risc0-zkvm`][risc0-zkvm], [`cargo-risczero`][cargo-risczero],
  [`risc0-build`][risc0-build], and [others][crates].

## Directory Structure

It is possible to organize the files for these components in various ways.
However, in this starter template we use a standard directory structure for zkVM
applications, which we think is a good starting point for your applications.

```text
project_name
├── Cargo.toml
├── contract
│   ├── Cargo.toml
│   └── src
│       └── lib.rs         <-- [Contract code goes here, common to host & guest]
├── host
│   ├── Cargo.toml
│   └── src
│       └── main.rs        <-- [Host code goes here]
└── methods
    ├── Cargo.toml
    ├── build.rs
    ├── guest
    │   ├── Cargo.toml
    │   └── src
    │       └── main.rs    <-- [Guest code goes here]
    └── src
        └── lib.rs
```

<!--[bonsai access]: https://bonsai.xyz/apply-->

[cargo-risczero]: https://docs.rs/cargo-risczero
[crates]: https://github.com/risc0/risc0/blob/main/README.md#rust-binaries
[dev-docs]: https://dev.risczero.com
[dev-mode]: https://dev.risczero.com/api/generating-proofs/dev-mode
[docs.rs]: https://docs.rs/releases/search?query=risc0
[examples]: https://github.com/risc0/risc0/tree/main/examples
[risc0-build]: https://docs.rs/risc0-build
[risc0-repo]: https://www.github.com/risc0/risc0
[risc0-zkvm]: https://docs.rs/risc0-zkvm
[rust-toolchain]: rust-toolchain.toml
[rustup]: https://rustup.rs
[zkvm-overview]: https://dev.risczero.com/zkvm

# Elastic Poll

[![Build status][ci-image]][ci-url]
[![Live website][website-image]][website-url]
[![License: Apache-2.0][license-image]][license-url]

[ci-image]: https://github.com/slowli/elasticpoll.app/actions/workflows/ci.yml/badge.svg
[ci-url]: https://github.com/slowli/elasticpoll.app/actions/workflows/ci.yml
[website-image]: https://img.shields.io/badge/website-live-blue.svg
[website-url]: https://elasticpoll.app/
[license-image]: https://img.shields.io/github/license/slowli/elasticpoll.app.svg
[license-url]: https://github.com/slowli/elasticpoll.app/blob/main/LICENSE

Serverless web app that allows organizing single-choice and multi-choice polls 
that combine privacy and universal verifiability with the help of 
some applied cryptography.

Core technologies:

- Rust programming language, WASM and [Yew] framework for dynamic logic
- [Webpack] packager
- [Bootstrap] and [Bootstrap Icons] for styling
- Additively homomorphic [ElGamal encryption] and related [zero-knowledge proofs][ZKP]
  to prove authenticity of voting and tallying without disclosing any information
  powered by the [`elastic-elgamal`] library with the [Ristretto255] crypto backend

See the *Implementation* page for details on how the app works, 
and the *About* page for more details on technologies used.

## ⚠ Warnings

Cryptography behind the app was not independently audited, in particular
against side-channel (e.g., timing) attacks.

The app entirely lacks a server part; the poll state is stored within the browser.
It is participants’ responsibility to exchange data via a reliable broadcast channel,
sync it among themselves, and to back this data up if needed.

The application is early-stage and the backward compatibility is not yet a thing.
Thus, polls created in one version of the app may become unreadable in the following versions.

## Running locally

You will need to install a Node / npm toolchain (preferably via a manager like [`nvm`])
and a Rust toolchain (preferably via [`rustup`]). Both toolchains should be recent; i.e., Node 16-LTS
and Rust 1.57+. You should also install [`wasm-pack`].

To serve the app locally with the Webpack dev server, run

```shell
npm start
```

## Testing

To run tests, use `npm test`.
Be aware that this command requires specifying browsers used for testing as flags
(e.g., `-- --firefox`).

Consult [`package.json`](package.json) for the full list of linting and testing commands.
Note that Rust-related linting requires additional components (`fmt` and `clippy`) installed as a part
of the relevant toolchain.

## License

Licensed under [Apache-2.0 license](LICENSE).

[Yew]: https://yew.rs/
[Webpack]: https://webpack.js.org/
[Bootstrap]: https://getbootstrap.com/
[Bootstrap Icons]: https://icons.getbootstrap.com/
[ElGamal encryption]: https://en.wikipedia.org/wiki/ElGamal_encryption
[ZKP]: https://en.wikipedia.org/wiki/Zero-knowledge_proof
[Ristretto255]: https://ristretto.group/
[`elastic-elgamal`]: https://github.com/slowli/elastic-elgamal
[`nvm`]: https://github.com/creationix/nvm
[`rustup`]: https://rustup.rs/
[`wasm-pack`]: https://rustwasm.github.io/wasm-pack/installer/

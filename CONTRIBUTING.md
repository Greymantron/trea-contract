# Contributing to Trea Contract

Thanks for considering contributing. This repo contains only the **Soroban smart contract** for Trea, an event registration/ticketing platform on Stellar. The backend and frontend live in separate repos — if you're looking to contribute there, see the links in the README.

## Code of conduct

Be respectful, assume good faith, and keep disagreements about the code, not the person. Harassment or abusive behavior toward maintainers or other contributors will result in removal from the project and its communication channels.

## Ways to contribute

- New contract features (e.g. partial refunds, waitlists, transferable tickets)
- Bug fixes
- Gas/resource optimization
- Additional test coverage, especially for edge cases and failure paths
- Documentation improvements

If you found this repo through **Drips Wave**, issues there are tagged with a complexity/point value (Trivial / Medium / High). Comment on the issue to claim it before starting work — this avoids duplicate effort.

## Project setup

### Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) v1.84.0+
- `rustup target add wasm32v1-none`
- [Stellar CLI](https://developers.stellar.org/docs/tools/cli): `cargo install --locked stellar-cli`

### Clone and build

```bash
git clone https://github.com/Trea-Hub/trea-contract.git
cd trea-contract/contracts/registration
cargo test
stellar contract build
```

If `cargo test` fails on a clean checkout, that's a bug worth opening an issue for before you start other work.

## Branching and commits

- Branch off `main`: `git checkout -b feat/short-description` or `fix/short-description`.
- Keep commits scoped — one logical change per commit is easier to review than one giant commit at the end.
- Write commit messages in the imperative mood: `Add refund deadline check`, not `Added` or `Adding`.

## Coding conventions

- Run `cargo fmt` before committing.
- Every state-changing function that acts on behalf of an address must call `.require_auth()` on that address — this is the contract's core security boundary. A PR that changes authorization logic gets closer review.
- New storage entries should use `persistent()` storage unless there's a specific reason for `temporary()` or `instance()` — leave a comment explaining the choice if you deviate.
- Favor small, composable functions over long ones; Soroban contracts have a 32-character limit on public function names, and a max contract size, so keep logic lean.
- Don't move funds directly to an address that isn't the transaction signer unless the transfer is the contract paying out of its **own** balance (escrow pattern) — see `refund`/`payout` in `lib.rs` for the existing pattern.

## Tests

- Every new public contract function needs at least one passing-path test and one test for each way it can fail (wrong caller, missing state, deadline passed, etc.).
- Use `env.mock_all_auths()` for unit tests — it bypasses signature verification but still enforces your `assert!` logic, so a passing test with a fixed bug should fail again if the bug is reintroduced.
- Name tests descriptively: `test_<function>_<scenario>_<expected_outcome>`, e.g. `test_self_refund_after_deadline_fails`.
- Run the full suite before opening a PR: `cargo test`.

## Pull requests

1. Make sure `cargo test` and `stellar contract build` both succeed locally.
2. Reference the issue you're closing, e.g. `Closes #12`.
3. Describe **what changed and why** — not just what the diff shows. If you made a design decision (e.g. escrow vs. direct payment), explain the reasoning briefly.
4. Keep PRs focused on one issue. Unrelated cleanup can be a separate PR.
5. A maintainer will review, may request changes, and will merge once CI and review pass.

## Reporting bugs

Open an issue with:
- What you expected to happen vs. what actually happened
- Steps to reproduce (exact commands, inputs)
- Relevant error output (paste the full panic/error, not a summary)
- Your environment: Rust version (`rustc --version`), Stellar CLI version (`stellar --version`), OS

## Reporting security issues

**Do not open a public issue for a security vulnerability**, especially anything involving fund safety (unauthorized transfers, escrow draining, auth bypass). Contact the maintainers privately first. *(Add a security contact email or `SECURITY.md` once one exists.)*

## Questions

Open a discussion or issue if something in this guide is unclear — that's useful feedback for improving the docs, not a bother.
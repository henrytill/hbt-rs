# hbt-rs

## REMEMBER

**Use GitHub MCP for all GitHub queries** (instead of fetching webpages)

**Never work directly on `master`** - branch first, land via PR (see [Git Workflow](#git-workflow))

**Run `cargo fmt` and `cargo clippy` before every commit** - both are flake checks, and the Nix job is the only place they run in CI (see [CI](#ci))

**Use `cargo add` for new dependencies** - it picks up the current version rather than a guessed one

**Fixtures live in a submodule shared with three other implementations** - changing one is a cross-language decision (see [Testing](#testing))

## Overview

A Rust implementation of hbt, a bookmark and document collection tool, developed differentially alongside:

- [hbt-go](https://github.com/henrytill/hbt-go) (Go)
- [hbt-ocaml](https://github.com/henrytill/hbt-ocaml) (OCaml)
- [hbt-hs](https://github.com/henrytill/hbt-hs) (Haskell)

The tool reads bookmarks from Pinboard exports (JSON/XML), Netscape bookmark HTML, and Markdown, merges them into a collection keyed by URL, and writes the result as YAML or HTML.

The four implementations share a wire format and a fixture corpus, so a semantic question - what merging two entities that share a timestamp should produce, say - gets settled once and pinned in [hbt-data](https://github.com/henrytill/hbt-data), then implemented in each. Issues are filed as companions across the repos; the discussion usually lives in whichever one hit it first.

## Core Principles

- **Typecheck early & often**: Types not only help us check the correctness of our code, they are also a tool for helping to guide our creative process and ideation.
- **No recursion**: Avoid recursive function calls to prevent stack overflow on deeply nested user-provided data. Both parsers already do this - `html.rs` walks an explicit `Vec<StackItem>`, `markdown.rs` a `ParserState` - so keep it that way.
- **No unsafe**: Nothing we're doing in this codebase warrants it. `#![forbid(unsafe_code)]` belongs at every crate root - see [Workspace Layout](#workspace-layout) for where that currently holds and where it doesn't.
- **Make illegal states unrepresentable**: Preferred over guarding at each construction site. The multi-valued fields on `Entity` are all `BTreeSet` for this reason - see [Merge Semantics](#merge-semantics). But read the exception below before extending the argument to `Collection::edges`.

**`Collection::edges` is deliberately not a set.** It is a `Vec<Vec<usize>>` whose dedupe lives in `add_edge`'s early return, and it looks like an unfinished version of the `extended`/`updated_at` cleanup. It isn't. `extended` is unordered where `edges` is not: the fixture corpus has committed to insertion order, and a `BTreeSet` would silently restate that as ascending. No fixture is non-ascending today, which is what makes the reinterpretation invisible rather than safe. The invariant is asserted in the schema instead, via `schemars(extend)`, without touching the representation. See #55 and `fd33db6`.

## Workspace Layout

Crates, edition 2024, ISC, versioned together via `workspace.package`.

| Crate | Path | Role |
| --- | --- | --- |
| `hbt-core` | `core/` | Collection and entity model, parsers, formatters |
| `hbt` | `cli/` | Command-line binary |
| `hbt-pinboard` | `pinboard/` | Pinboard export decoding (`Post::from_json`, `Post::from_xml`) |
| `hbt-attic` | `attic/` | Scratch space, not depended on by anything |

Each crate root opens with `#![forbid(unsafe_code)]`, `#![warn(clippy::pedantic)]`, `#![deny(clippy::unwrap_in_result)]`. Give a new crate the same header - there is no `[workspace.lints]` table, so nothing detects the omission.

`hbt-attic`'s clippy warnings are long-standing; don't treat them as a regression you introduced.

### `hbt-core`

- `collection.rs` - `Collection`, a `Vec<Entity>` plus a `HashMap<Url, usize>` index and an adjacency list. `upsert` is the merge entry point; `add_edges` is bidirectional (`add_edge` is one-way and idempotent). `CollectionRepr` is the serialized shape and the type the JSON schema is generated from.
  - An `Id` is not a bare index: it carries a `Weak<()>` back to the collection's `Rc<()>` token, and equality compares the owner as well as the index. This is the generativity/branding pattern - it makes an `Id` minted by one collection unusable against another, and `check_id` turns a foreign or stale id into a panic rather than a silent read of the wrong entity. Keep it when adding APIs that hand out or accept ids. Note the consequence: the `Rc` makes `Collection` and `Id` neither `Send` nor `Sync`, so parsing cannot be parallelized across threads as written. Swapping in `Arc` would lift that, but dropping the token to satisfy a trait bound discards the invariant.
- `entity.rs` - `Entity`, where every field is a newtype rather than a bare primitive, and `Flag`, the tri-state `Option<bool>` that `Shared`, `ToRead` and `IsFeed` share. Merge semantics live here.
- `html.rs` - Netscape bookmark parsing via `scraper`, and formatting via a minijinja template at `core/src/html/netscape_bookmarks.jinja`. Escaping is handled explicitly (`escape_attr` / `escape_text`), since the template does not autoescape.
- `markdown.rs` - pulldown-cmark parsing, where heading depth builds the entity graph.
- `lib.rs` - `InputFormat` (`Json`, `Xml`, `Markdown`, `Html`) and `OutputFormat` (`Html`, `Yaml`), each with `detect` (by extension) and `parse`/`unparse`. Adding a format means adding a variant here plus the module that backs it.

`hbt-core` has one optional feature, `clap`, off by default; it only adds `ValueEnum` impls for the two format enums, and `cli` turns it on. There is no `pinboard` feature - Pinboard support moved into its own always-on crate.

## Merge Semantics

The most-revised part of the codebase, and where the cross-implementation issues concentrate. `Entity::merge` absorbs another entity with the same URL.

- **An entity's `updated_at` never contains its `created_at`, and this is maintained at construction and at decoding, not only by the merge.** `Entity::normalize` is the whole of the normal form, settled for all four implementations as henrytill/hbt-data#38. Three places end in it: `merge`, `Entity::from_attrs` (HTML reads `ADD_DATE` and `LAST_MODIFIED` independently, so one anchor may state the same instant in both - the `html/bookmarks_simple` shape) and `Deserialize` (a serialized history is input like any other). `Entity::new` and `TryFrom<Post>` are normal only because they record no updates at all; one that learns to must normalize too. This is guarding at each construction site, which the Core Principle above says to prefer *against* -- the exception is deliberate, and the note there explains when the principle yields. What makes it an exception rather than an oversight: `Serialize` carries a `debug_assert!` that the normal form holds, so a forgotten fourth site fails a debug test run rather than shipping quietly. Closing the representation is the stronger form. `Entity`'s fields are already private, but `pub mod html` is a child of the module that owns it, so `from_attrs` still builds the struct and mutates it field by field; giving it its own attribute record and one normalizing constructor is what would finish the job. That is #85, and henrytill/hbt-hs#54 is the same change for hbt-hs, where it costs far more. **Merging is field-wise, then `normalize`d** - do not give `merged_updates` a removal of its own, since two spellings of one rule is what a later change would have to keep in step.
- **The merged updates are every history and every creation time in the merge, minus the smallest creation time**, which becomes `created_at`. `Entity::merged_updates` states it. This is the rule that keeps merging associative, which is how henrytill/hbt-data#36 settled it for all four implementations: putting both creation times back into the history before removing the winner is what makes any bracketing of a sequence of mentions agree. Removing the winner only when the two creation times differ is *not* associative, and neither is removing every update at or below `created_at`; that issue carries both counterexamples, and `merge_is_associative` pins the rule here.
- **A timestamp equal to `created_at` is not recorded** - an "update" that merely repeats the creation instant carries no information (henrytill/hbt-go#57). The rule above extends it to the instant that *becomes* `created_at`, including one no merge moved (`html/bookmarks_merged_repeat`).
- **A creation time can be absent, and an absent one contributes nothing.** `CreatedAt` wraps an `Option<Time>`, so an anchor with no `ADD_DATE` parses to an entity with no creation time rather than one created at the epoch. `CreatedAt::merge` makes absence the identity: a dated mention merged with an undated one keeps its own instant, and neither operand pushes an absence into the history. This is henrytill/hbt-data#37, and it is why the merge cannot be `min` over the derived `Ord`, where `None` sorts below every `Some` and would win. Two undated mentions stay undated.
- **Absence has a wire representation**, which is what makes the rule survive a round-trip: `createdAt` is omitted when absent, as `shared` and `lastVisitedAt` are, and is no longer in the schema's `required` list. Before that, an undated entity serialized as `createdAt: 0` and decoded back as one genuinely created on 1970-01-01, so the same collection merged differently depending on whether it had passed through YAML. A real creation time of `0` still serializes, and still exports `ADD_DATE="0"`: every optional timestamp in the HTML template tests `is not none` rather than truthiness, which is what #63 was -- `LAST_MODIFIED` and `LAST_VISIT` both dropped a value of `0` until this rule reached them.
- **An update strictly below `created_at` stays.** A single anchor states that shape, since HTML reads `ADD_DATE` and `LAST_MODIFIED` independently, and the same law requires keeping it: henrytill/hbt-data#34, pinned by `html/bookmarks_update_before_creation`.
- **Both histories are unioned.** `merge` used to discard the incoming entity's `updated_at` entirely, which was #64; it is fixed, and `html/bookmarks_incoming_update` pins it.
- **Identical entities short-circuit.** `if *self == other { return self; }`, with the field-wise merge split out as `merge_unguarded`. Before the normal form this guard was load-bearing: an entity parsed from `LAST_MODIFIED == ADD_DATE` would lose that update on meeting a byte-identical duplicate. Normalizing at parse and at decode makes that entity unreachable, so the guard now changes the result only for one written down by hand - `merge_is_idempotent_without_the_guard` pins the stronger claim that the field-wise merge is already idempotent on a normalized entity, and `merge_is_idempotent_for_identical_entities` pins what the guard still buys. It is kept for parity: hbt-hs guards the same way in `absorb`, outside the `Semigroup` instance. It cannot affect associativity either way, since any later merge puts both creation times back regardless.
- **Multi-valued fields are sets, not vectors.** `names`, `labels`, `extended`, and `updated_at` are all `BTreeSet`. This is the load-bearing decision: the equality guard covers only identical entities, so two entities that differ in *any* field bypass it, and anything appended would land once per occurrence - making the output depend on how many times the input mentioned a bookmark. `extended` became a set in #52, `updated_at` in #54.
- **Ordering is a consequence, not a step.** A `BTreeSet` is sorted by construction; there is no explicit sort to maintain.

The corpus cannot reach the decoding half - `hbt` has no YAML *input* format, so nothing round-trips there - and `decoding_normalizes_the_update_history` is what covers it. The other three implementations unit-test their own decode path for the same reason.

When touching this, add a unit test in `core/src/entity.rs` *and* consider whether the case deserves a shared fixture. Use three occurrences rather than two when the bug is a duplication - two cannot distinguish "deduplicated" from "recorded once by accident".

## Testing

Three layers: unit tests beside the code in `core/`, the conformance harness run against the built binary, and CLI integration tests.

**Shared fixtures.** `test-data/` is a git submodule of [hbt-data](https://github.com/henrytill/hbt-data), consumed by all four implementations. Clone with `--recurse-submodules`, or run `git submodule update --init`. Changing a fixture is a cross-language decision: it will go red in the other three until their fixes land, so a fix and its submodule bump belong in the same commit.

**Conformance.** `test-data/` also carries hbt-data's conformance harness (henrytill/hbt-data#14), which runs every fixture through the built `hbt` and compares what the CLI writes - the serialized form every implementation shares, not decoded values, so a change to serialization alone cannot stay green. It is the `conformance` flake check: `test-data/` is also the `hbt-data` flake input (`path:./test-data`), whose `lib.check` runs the harness, so nothing about the harness is wired up here. A relative path input locks relative to this flake rather than by hash, so bumping the submodule needs no relock. In the dev shell, which provides the harness's Python from the same input:

```sh
cargo build -p hbt
(cd test-data && python3 -m hbt.conformance --binary ../target/debug/hbt)                 # every fixture
(cd test-data && python3 -m hbt.conformance --binary ../target/debug/hbt markdown/basic)  # a name, substring or glob
```

The harness's flags, what counts as a match, and its timezone policy are documented in `test-data/README.md`.

A `github:` flake reference carries no submodules, so it lacks the `hbt-data` input: `nix build github:henrytill/hbt-rs` still works, but `nix develop` and `nix flake check` fail on it and need a reference that includes submodules, such as `git+https://github.com/henrytill/hbt-rs?submodules=1`, or a checkout.

**CLI integration tests.** `cli/tests/cli.rs` drives the built binary with `snapbox`, covering the flags and the error paths. snapbox's default filters rewrite backslashes to forward slashes, so `schema_output` compares `.raw()` - the JSON schema contains a regex that would otherwise be corrupted.

**Regenerating the schema.** `cli/tests/cli.rs` pins `test-data/collection.schema.json` against `hbt --schema`. Changing a serialized type changes the schema, so regenerate it into the submodule and commit it there:

```sh
cargo run -p hbt -- --schema > test-data/collection.schema.json
```

That writes *inside the submodule*, so it needs its own commit in hbt-data, and hbt-data must be pushed before the superproject PR can pass. Both CI jobs check out with submodules, so a bump pointing at a commit that exists only in your local `test-data/` builds and tests green on your machine and fails on GitHub with an unresolvable ref.

## Development Commands

Standard cargo, with three things worth pinning:

```sh
cargo clippy --workspace --all-targets
cargo fmt                       # no rustfmt.toml; plain default style
cargo add -p hbt-core <dep>     # never hand-edit Cargo.toml for this
```

### Running the CLI

```sh
cargo run -p hbt -- -t yaml test-data/html/bookmarks_simple.input.html
cargo run -p hbt -- --mappings mappings.yaml -t yaml input.html
```

| Flag | Meaning |
| --- | --- |
| `-f`, `--from` | Input format: `json`, `xml`, `markdown`, `html` - the names all four implementations accept. Inferred from the extension when omitted |
| `-t`, `--to` | Output format: `html`, `yaml`. Inferred from `--output`'s extension when omitted |
| `-o`, `--output` | Output file; stdout otherwise. Applies to `-t` output and `--schema` only - `--info` and `--list-tags` always write to stdout and ignore it silently |
| `--info` | Entity count |
| `--list-tags` | Every label, sorted |
| `--schema` | JSON schema for `CollectionRepr` |
| `--mappings <FILE>` | Label rewrites |

An output format or an analysis flag is required; with neither, the CLI errors. The flags do not compose - they short-circuit in the order `--schema`, `--info`, `--list-tags`, then `-t`/`-o`. `hbt --info -t yaml -o out.yaml f.md` prints the entity count and exits 0, writing nothing. The mappings file is parsed as YAML - a flat old-label-to-new mapping - so JSON works as a subset rather than as the expected form. Entries that aren't string pairs are an error, not silently skipped.

### Nix

```sh
nix flake check -L        # cargo-clippy, cargo-deny, cargo-fmt, conformance, plus both package builds
nix build -L .#hbt
nix build -L .#hbt-static # musl static build, Linux only
nix develop               # adds rust-analyzer, cargo-deny, yaml-language-server, and the harness's Python
```

The flake sets `self.submodules = true`, so flake builds see `test-data/`. `HBT_COMMIT_HASH` and `HBT_COMMIT_SHORT_HASH` are baked in for `--version`.

`deny.toml` holds a license allowlist and explicitly denies `libyml` and `serde_yml` - hence `serde_norway` as the YAML implementation. Adding a dependency that pulls either in will fail `cargo-deny`.

## CI

`.github/workflows/ci.yml` runs on pushes and PRs to `master`, with no path filter, so every PR gets both jobs:

- **Linux (cargo)** - `cargo build`, `cargo test`, with a cargo cache, then the conformance harness without Nix, through hbt-data's action (`test-data/.github/actions/conformance`, with `binary: target/debug/hbt`), which brings its own Python. A red job with a green `cargo test` is a fixture failing conformance - see [Testing](#testing).
- **Linux (Nix flake)** - `nix flake check -L` plus both package builds.

Both are required status checks. **Formatting, clippy and cargo-deny run only in the Nix job**, so a green local `cargo test` says nothing about whether CI will pass; run `cargo fmt`, `cargo clippy` and the conformance harness yourself before pushing. Two other workflows: `zizmor.yml` (Actions security scan, path-filtered to `.github/**` plus a weekly cron) and `update.yml` (monthly flake lock bump).

## Git Workflow

**Never commit to `master`.**

```sh
git checkout -b <topic>   # branch first, before making any changes
# ... work, commit ...
git push -u origin <topic>
gh pr create
gh pr merge --rebase      # after CI is green
```

If changes have already been made on `master` by mistake, move them to a branch before committing. `git branch -f` on the branch you are *not* on is the clean way to rewind one without disturbing the working tree or the submodule.

### Branch protection on `origin/master`

The GitHub remote enforces this; direct pushes to `master` will be rejected.

| Rule | Setting |
| --- | --- |
| Pull request required | yes, 0 approvals (stale reviews dismissed) |
| Required status checks | `Linux (cargo)`, `Linux (Nix flake)`, strict (branch must be up to date) |
| Linear history | required |
| Conversation resolution | required |
| Force pushes / deletions | blocked |
| Applies to administrators | yes (no bypass) |

### Merge settings

Rebase is the only merge method enabled; squash merges and merge commits are turned off. Merged branches are deleted automatically on GitHub, so prune locally afterwards:

```sh
git fetch --prune
git branch -D <topic>   # -d may refuse: rebase merges rewrite SHAs
```

Rebase merges always create new commit SHAs, so a local branch kept after merging will look diverged from `master`. Delete it rather than reusing it.

### The `ivan` remote

`ivan:/srv/git/hbt-rs.git` is a plain bare repo with no protection. Pushing `master` there directly is fine and unaffected by the above. It also carries a long-lived `develop` branch that `origin` does not have, and which drifts well behind `master`; don't assume a branch seen there exists on GitHub.

### Commit messages

`<scope>: <terse description>`, where the scope is the module or file being changed - `entity:`, `collection:`, `ci:`, `AGENTS.md:`. Use the type as the scope and don't requalify its members (`entity: make extended a BTreeSet`, not `entity: Entity::extended ...`).

Wrap the body at the usual width and explain *why*, particularly which invariant was wrong and what now makes it unrepresentable. Reference the companion issues in the other repos by full `henrytill/hbt-go#65` form, since bare `#65` resolves to this repo.

Do **not** hard-wrap prose in GitHub issue bodies, PR bodies, or comments - one long line per paragraph, and let GitHub wrap it. Commit messages are the exception.

## Conventions

- **Newtypes over bare primitives.** `Label(String)`, `CreatedAt(Time)`, and friends exist so the type checker catches field mix-ups. Follow the pattern when adding a field.
- **`const fn` where possible** on small accessors.
- **`#[must_use]`** on accessors returning owned or borrowed data.
- **Comments explain the bug that motivated the code.** Much of the codebase carries short notes of the form "X used to happen, so now Y" with an issue reference. This is deliberate and worth continuing: it stops a later simplification from quietly reintroducing a fixed bug.
- **Optional fields are omitted, not nulled.** The shared wire format leaves an unset optional field out entirely; `skip_serializing_if` handles this. schemars drops a `default` annotation its own `skip_serializing_if` would skip, so `#[schemars(extend("default" = ...))]` restates it where the published schema needs to document the empty case.
